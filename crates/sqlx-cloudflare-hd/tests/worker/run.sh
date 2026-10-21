#!/usr/bin/env bash
# Starts a scratch Postgres, builds the test Worker, serves it with
# `wrangler dev --local` with its Hyperdrive binding pointed at that Postgres,
# and runs `tests/driver.rs` against it. Part of what `make test-worker` runs.
#
# Scratch means a fresh cluster in a temporary directory, on TCP only, with
# SCRAM passwords and no TLS: the connection goes through the password
# handshake, and the default `sslmode=prefer` through the declined-TLS path.
# The user and password are throwaway values for this cluster alone.
set -euo pipefail

cd "$(dirname "$0")"
port="${HD_WORKER_PORT:-8789}"
pg_port="${HD_PG_PORT:-55433}"
state="$(mktemp -d)"
export WRANGLER_SEND_METRICS=false

cleanup() {
  kill "${wrangler:-}" 2>/dev/null || true
  pg_ctl -D "$state/pg" -m immediate stop >/dev/null 2>&1 || true
  rm -rf "$state"
}
trap cleanup EXIT

echo hd >"$state/pwfile"
initdb -D "$state/pg" -U hd --auth=scram-sha-256 --pwfile="$state/pwfile" >/dev/null
# No Unix socket: its path would sit under $TMPDIR, which on macOS is longer
# than a socket path may be.
pg_ctl -D "$state/pg" -l "$state/postgres.log" -w \
  -o "-p $pg_port -c listen_addresses=127.0.0.1 -c unix_socket_directories=''" start >/dev/null
PGPASSWORD=hd psql -X -h 127.0.0.1 -p "$pg_port" -U hd -d postgres -qc 'CREATE DATABASE hd'

export CLOUDFLARE_HYPERDRIVE_LOCAL_CONNECTION_STRING_HD="postgres://hd:hd@127.0.0.1:$pg_port/hd"

worker-build --release

wrangler dev --local --port "$port" >"$state/wrangler.log" 2>&1 &
wrangler=$!

for _ in $(seq 1 60); do
  if curl -sf "http://localhost:$port/" >/dev/null; then
    break
  fi
  if ! kill -0 "$wrangler" 2>/dev/null; then
    cat "$state/wrangler.log"
    exit 1
  fi
  sleep 1
done

cd ../..
HD_WORKER_URL="http://localhost:$port" cargo test --test driver -- --nocapture
