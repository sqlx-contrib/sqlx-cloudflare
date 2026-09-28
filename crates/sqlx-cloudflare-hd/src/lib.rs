//! Cloudflare Hyperdrive for sqlx.
//!
//! Hyperdrive speaks the Postgres wire protocol, so there is no driver here:
//! this crate opens the Worker's socket to a Hyperdrive binding and hands it to
//! sqlx's own Postgres driver. What you get back is a plain
//! [`PgConnection`] -- `sqlx::Postgres`, with
//! all of its types, `query_as`, `#[derive(sqlx::FromRow)]`, transactions and
//! `sqlx::migrate!`.
//!
//! ```no_run
//! use sqlx::Connection;
//!
//! async fn user_name(env: &worker::Env, id: i64) -> Result<String, sqlx::Error> {
//!     // One connection per request: Hyperdrive pools the connections to
//!     // the database behind it.
//!     let mut conn = sqlx_cloudflare_hd::connect(env, "HYPERDRIVE").await?;
//!
//!     let name = sqlx::query_scalar("SELECT name FROM users WHERE id = $1")
//!         .bind(id)
//!         .fetch_one(&mut conn)
//!         .await?;
//!
//!     conn.close().await?;
//!     Ok(name)
//! }
//! ```
//!
//! # What does not work
//!
//! - **`sqlx::Pool`.** It needs an async runtime to spawn its maintenance
//!   tasks, and a Worker cannot share a socket across requests anyway.
//!   Hyperdrive is the pool: connect once per request.
//! - **`PgListener`** (`LISTEN`/`NOTIFY`), which also spawns a task.
//!
//! This crate only *runs* on `wasm32-unknown-unknown`, inside a Worker. It
//! compiles on other targets so that docs and `cargo check` work.

mod socket;

use std::io;

use sqlx_core::net::Socket as _;
use sqlx_core::Error;
use sqlx_postgres::{PgConnectOptions, PgConnection, PgSslMode};
use worker::{Hyperdrive, SecureTransport};

use crate::socket::HdSocket;

/// A Postgres `SSLRequest`: length 8, then the request code 80877103.
const SSL_REQUEST: [u8; 8] = [0, 0, 0, 8, 0x04, 0xd2, 0x16, 0x2f];

/// Connects to the database behind the Hyperdrive binding called `binding`.
///
/// # Errors
///
/// When there is no binding by that name, or it is not a Hyperdrive
/// configuration; when the socket, TLS or the Postgres handshake fails; and
/// when the credentials Hyperdrive holds are rejected.
pub async fn connect(env: &worker::Env, binding: &str) -> Result<PgConnection, Error> {
    let hyperdrive = env
        .hyperdrive(binding)
        .map_err(|e| Error::Configuration(e.to_string().into()))?;
    connect_with(&hyperdrive, &options(&hyperdrive)?).await
}

/// The options Hyperdrive hands out, parsed from its connection string.
///
/// Start here to change anything before [`connect_with`] -- the application
/// name, the statement cache -- while keeping the credentials.
///
/// # Errors
///
/// When the connection string does not parse, which would be a Hyperdrive bug.
pub fn options(hyperdrive: &Hyperdrive) -> Result<PgConnectOptions, Error> {
    hyperdrive.connection_string().parse()
}

/// Connects through `hyperdrive` with `options`.
///
/// The socket always goes to Hyperdrive's host and port; the host, port and
/// socket path in `options` are ignored. `ssl_mode` is honoured, but the
/// Workers runtime performs the TLS handshake, so the certificate options
/// (`ssl_root_cert` and the client certificate and key) are not used.
///
/// # Errors
///
/// When the socket, TLS or the Postgres handshake fails, or the credentials
/// are rejected.
pub async fn connect_with(
    hyperdrive: &Hyperdrive,
    options: &PgConnectOptions,
) -> Result<PgConnection, Error> {
    let mode = options.get_ssl_mode();
    let transport = if wants_tls(mode) {
        SecureTransport::StartTls
    } else {
        SecureTransport::Off
    };
    let socket = worker::Socket::builder()
        .secure_transport(transport)
        .connect(hyperdrive.host(), hyperdrive.port())
        .map_err(|e| Error::Io(io::Error::other(e.to_string())))?;

    let socket = negotiate_tls(HdSocket::new(socket), mode).await?;

    // TLS is settled -- upgraded, or declined as `mode` allows -- so sqlx must
    // not ask the server again.
    let options = options.clone().ssl_mode(PgSslMode::Disable);
    PgConnection::connect_with_socket(&options, socket).await
}

fn wants_tls(mode: PgSslMode) -> bool {
    !matches!(mode, PgSslMode::Disable | PgSslMode::Allow)
}

/// Asks the server for TLS, as sqlx itself would, but upgrades through the
/// Workers runtime's `startTls()` rather than a TLS library of sqlx's.
async fn negotiate_tls(mut socket: HdSocket, mode: PgSslMode) -> Result<HdSocket, Error> {
    if !wants_tls(mode) {
        return Ok(socket);
    }

    let mut request = &SSL_REQUEST[..];
    while !request.is_empty() {
        let n = socket.write(request).await?;
        request = &request[n..];
    }
    socket.flush().await?;

    // The server answers with a single byte: `S` to go ahead, `N` to decline.
    let mut reply = [0u8; 1];
    let mut buf = &mut reply[..];
    if socket.read(&mut buf).await? == 0 {
        return Err(Error::Io(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "server closed the connection during TLS negotiation",
        )));
    }

    match reply[0] {
        b'S' => Ok(HdSocket::new(socket.into_inner()?.start_tls())),
        b'N' if matches!(mode, PgSslMode::Prefer) => Ok(socket),
        b'N' => Err(Error::Tls("server does not support TLS".into())),
        other => Err(Error::Protocol(format!(
            "unexpected response to SSLRequest: {other:#04x}"
        ))),
    }
}
