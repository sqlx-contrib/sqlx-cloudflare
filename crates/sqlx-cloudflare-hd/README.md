# sqlx-cloudflare-hd

> Cloudflare Hyperdrive for sqlx: sqlx's own Postgres driver over a Workers
> Hyperdrive binding, so a Rust Worker gets a real `sqlx::Postgres` connection.

[![CI](https://github.com/sqlx-contrib/sqlx-cloudflare/actions/workflows/ci.yml/badge.svg)](https://github.com/sqlx-contrib/sqlx-cloudflare/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/sqlx-contrib/sqlx-cloudflare/blob/main/LICENSE)

> [!WARNING]
> **Not published yet.** This crate needs two sqlx changes that have not been
> released -- `PgConnection::connect_with_socket` and an `Instant` that works
> on `wasm32-unknown-unknown` -- proposed in
> [transact-rs/sqlx#4426](https://github.com/transact-rs/sqlx/issues/4426).
> Until they ship, it builds against that branch and cannot go to crates.io.

```rust
use sqlx::Connection;

#[derive(sqlx::FromRow)]
struct User {
    id: i64,
    name: String,
}

#[worker::event(fetch)]
async fn fetch(
    _req: worker::Request,
    env: worker::Env,
    _ctx: worker::Context,
) -> worker::Result<worker::Response> {
    let mut conn = sqlx_cloudflare_hd::connect(&env, "HYPERDRIVE")
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;

    let user = sqlx::query_as::<_, User>("SELECT id, name FROM users WHERE id = $1")
        .bind(1_i64)
        .fetch_one(&mut conn)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;

    conn.close()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    worker::Response::ok(user.name)
}
```

There is no driver in here. Hyperdrive speaks the Postgres wire protocol, so
this crate opens the Worker's socket to the binding, settles TLS through the
Workers runtime, and hands the socket to sqlx's Postgres driver. What comes
back is a plain `PgConnection`: every type sqlx-postgres maps, `query_as`,
`#[derive(sqlx::FromRow)]`, transactions and `sqlx::migrate!`.

Connect once per request. Hyperdrive pools the connections to your database,
and a Worker cannot share a socket across requests anyway, so there is no
`sqlx::Pool` -- it also needs an async runtime a Worker does not have. For the
same reason there is no `PgListener`.

## Options

`connect` uses the options Hyperdrive hands out. To change any of them while
keeping the credentials, start from `options` and connect with `connect_with`:

```rust
let hyperdrive = env.hyperdrive("HYPERDRIVE")?;
let options = sqlx_cloudflare_hd::options(&hyperdrive)?.application_name("my-worker");
let mut conn = sqlx_cloudflare_hd::connect_with(&hyperdrive, &options).await?;
```

The socket always goes to Hyperdrive's host and port. `sslmode` is honoured,
but the Workers runtime performs the TLS handshake, so the certificate options
are not used.
