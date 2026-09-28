//! The scenarios the integration tests run, each inside a real Worker against
//! a scratch Postgres behind a local Hyperdrive binding.
//!
//! `GET /` lists the scenarios; `GET /<scenario>` runs one on a connection of
//! its own, as a request would, and answers `{"ok": true}` or
//! `{"ok": false, "error": "..."}`. The assertions live here, in Rust next to
//! the code they exercise, and `tests/driver.rs` on the host only has to walk
//! the list -- so adding a scenario is one function and one line in
//! `SCENARIOS`.

use std::future::Future;
use std::pin::Pin;

use sqlx::error::ErrorKind;
use sqlx::postgres::{PgConnection, PgSslMode};
use sqlx::{Connection, FromRow, Row};
use worker::{event, Context, Env, Request, Response};

type Outcome = Result<(), String>;
type Scenario = fn(Env) -> Pin<Box<dyn Future<Output = Outcome>>>;

macro_rules! scenarios {
    ($($name:ident),* $(,)?) => {
        const SCENARIOS: &[(&str, Scenario)] = &[
            $((stringify!($name), |env| Box::pin($name(env)))),*
        ];
    };
}

scenarios![
    binds_and_scalars,
    many_rows_span_many_reads,
    unique_violation,
    transactions,
    prepared_statement_reuse,
    from_row,
    migrations,
    ssl_disable_connects,
    ssl_require_fails_without_server_tls,
    connect_with_custom_options,
];

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> worker::Result<Response> {
    let name = req.path().trim_start_matches('/').to_owned();

    if name.is_empty() {
        let names: Vec<_> = SCENARIOS.iter().map(|(name, _)| *name).collect();
        return Response::from_json(&names);
    }

    let Some((_, scenario)) = SCENARIOS.iter().find(|(n, _)| *n == name) else {
        return Response::error(format!("no scenario `{name}`"), 404);
    };

    let body = match scenario(env).await {
        Ok(()) => serde_json::json!({ "ok": true }),
        Err(error) => serde_json::json!({ "ok": false, "error": error }),
    };

    Response::from_json(&body)
}

/// `Err` with the message unless `condition` holds.
fn ensure(condition: bool, message: impl Into<String>) -> Outcome {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn fail(error: sqlx::Error) -> String {
    error.to_string()
}

async fn connect(env: &Env) -> Result<PgConnection, String> {
    sqlx_cloudflare_hd::connect(env, "HD").await.map_err(fail)
}

async fn binds_and_scalars(env: Env) -> Outcome {
    let mut conn = connect(&env).await?;
    let row = sqlx::query(
        "SELECT $1::int4 + 1, $2::int8, $3::float8, $4::bool, $5::text, $6::bytea, NULL::int4",
    )
    .bind(41_i32)
    .bind(i64::MAX)
    .bind(1.5_f64)
    .bind(true)
    .bind("héllo")
    .bind(vec![0_u8, 255, 7])
    .fetch_one(&mut conn)
    .await
    .map_err(fail)?;
    let got = (
        row.try_get::<i32, _>(0).map_err(fail)?,
        row.try_get::<i64, _>(1).map_err(fail)?,
        row.try_get::<f64, _>(2).map_err(fail)?,
        row.try_get::<bool, _>(3).map_err(fail)?,
        row.try_get::<String, _>(4).map_err(fail)?,
        row.try_get::<Vec<u8>, _>(5).map_err(fail)?,
        row.try_get::<Option<i32>, _>(6).map_err(fail)?,
    );
    let want = (42, i64::MAX, 1.5, true, "héllo".to_owned(), vec![0, 255, 7], None);
    ensure(got == want, format!("got {got:?}, want {want:?}"))
}

async fn many_rows_span_many_reads(env: Env) -> Outcome {
    let mut conn = connect(&env).await?;
    // About 200 KiB of rows: far more than one read from the socket.
    let rows = sqlx::query("SELECT g, repeat('x', 100) || g FROM generate_series(1, 2000) g")
        .fetch_all(&mut conn)
        .await
        .map_err(fail)?;
    let last: i32 = rows.last().ok_or("no rows")?.try_get(0).map_err(fail)?;
    ensure(
        rows.len() == 2000 && last == 2000,
        format!("{} rows, last {last}", rows.len()),
    )
}

async fn unique_violation(env: Env) -> Outcome {
    let mut conn = connect(&env).await?;
    sqlx::raw_sql("CREATE TEMP TABLE t (id int8 PRIMARY KEY); INSERT INTO t VALUES (1)")
        .execute(&mut conn)
        .await
        .map_err(fail)?;
    let error = sqlx::query("INSERT INTO t VALUES (1)")
        .execute(&mut conn)
        .await
        .err()
        .ok_or("duplicate insert succeeded")?;
    let kind = error.as_database_error().map(|e| e.kind());
    ensure(
        matches!(kind, Some(ErrorKind::UniqueViolation)),
        format!("{kind:?}: {error}"),
    )
}

async fn transactions(env: Env) -> Outcome {
    let mut conn = connect(&env).await?;
    sqlx::query("CREATE TEMP TABLE t (id int8 PRIMARY KEY)")
        .execute(&mut conn)
        .await
        .map_err(fail)?;

    let mut tx = conn.begin().await.map_err(fail)?;
    sqlx::query("INSERT INTO t VALUES (1)")
        .execute(&mut *tx)
        .await
        .map_err(fail)?;
    tx.rollback().await.map_err(fail)?;

    let mut tx = conn.begin().await.map_err(fail)?;
    sqlx::query("INSERT INTO t VALUES (2)")
        .execute(&mut *tx)
        .await
        .map_err(fail)?;
    tx.commit().await.map_err(fail)?;

    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM t")
        .fetch_all(&mut conn)
        .await
        .map_err(fail)?;
    ensure(ids == [2], format!("ids {ids:?}"))
}

async fn prepared_statement_reuse(env: Env) -> Outcome {
    let mut conn = connect(&env).await?;
    for i in 0..3_i64 {
        let n: i64 = sqlx::query_scalar("SELECT $1::int8 * 2")
            .bind(i)
            .fetch_one(&mut conn)
            .await
            .map_err(fail)?;
        ensure(n == i * 2, format!("{i} * 2 = {n}"))?;
    }
    conn.close().await.map_err(fail)
}

async fn from_row(env: Env) -> Outcome {
    #[derive(Debug, PartialEq, FromRow)]
    struct User {
        id: i64,
        name: String,
        email: Option<String>,
    }

    let mut conn = connect(&env).await?;
    let user: User =
        sqlx::query_as("SELECT 7::int8 AS id, 'Ada' AS name, NULL::text AS email")
            .fetch_one(&mut conn)
            .await
            .map_err(fail)?;
    let want = User {
        id: 7,
        name: "Ada".into(),
        email: None,
    };
    ensure(user == want, format!("got {user:?}"))
}

async fn migrations(env: Env) -> Outcome {
    let mut conn = connect(&env).await?;
    // Start clean, so every run applies both migrations from scratch.
    sqlx::raw_sql("DROP TABLE IF EXISTS widgets, _sqlx_migrations")
        .execute(&mut conn)
        .await
        .map_err(fail)?;
    let migrator = sqlx::migrate!("./migrations");
    migrator.run(&mut conn).await.map_err(|e| e.to_string())?;
    // A second run finds everything applied and does nothing.
    migrator.run(&mut conn).await.map_err(|e| e.to_string())?;

    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM widgets ORDER BY id")
        .fetch_all(&mut conn)
        .await
        .map_err(fail)?;
    let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations WHERE success")
        .fetch_one(&mut conn)
        .await
        .map_err(fail)?;
    ensure(
        names == ["sprocket", "gear"] && applied == 2,
        format!("widgets {names:?}, {applied} migrations applied"),
    )
}

// The scratch Postgres has TLS off, so the default `prefer` -- what every
// other scenario connects with -- is already the declined-TLS path. These two
// cover the modes either side of it.

async fn ssl_disable_connects(env: Env) -> Outcome {
    let hd = env.hyperdrive("HD").map_err(|e| e.to_string())?;
    let options = sqlx_cloudflare_hd::options(&hd)
        .map_err(fail)?
        .ssl_mode(PgSslMode::Disable);
    let mut conn = sqlx_cloudflare_hd::connect_with(&hd, &options)
        .await
        .map_err(fail)?;
    conn.ping().await.map_err(fail)
}

async fn ssl_require_fails_without_server_tls(env: Env) -> Outcome {
    let hd = env.hyperdrive("HD").map_err(|e| e.to_string())?;
    let options = sqlx_cloudflare_hd::options(&hd)
        .map_err(fail)?
        .ssl_mode(PgSslMode::Require);
    match sqlx_cloudflare_hd::connect_with(&hd, &options).await {
        Ok(_) => Err("connected without TLS under `require`".into()),
        Err(sqlx::Error::Tls(_)) => Ok(()),
        Err(error) => Err(format!("wrong error: {error:?}")),
    }
}

async fn connect_with_custom_options(env: Env) -> Outcome {
    let hd = env.hyperdrive("HD").map_err(|e| e.to_string())?;
    let options = sqlx_cloudflare_hd::options(&hd)
        .map_err(fail)?
        .application_name("hd-test");
    let mut conn = sqlx_cloudflare_hd::connect_with(&hd, &options)
        .await
        .map_err(fail)?;
    let name: String = sqlx::query_scalar("SELECT current_setting('application_name')")
        .fetch_one(&mut conn)
        .await
        .map_err(fail)?;
    ensure(name == "hd-test", format!("application_name {name:?}"))
}
