# Changelog

## [0.2.0](https://github.com/sqlx-contrib/sqlx-cloudflare/compare/sqlx-cloudflare-d1-v0.1.0...sqlx-cloudflare-d1-v0.2.0) (2026-10-01)


### ⚠ BREAKING CHANGES

* `last_insert_rowid()` returns `i64`, and 0 where it returned `None`. `QueryResult::new` takes `last_insert_rowid: i64`.

### Features

* return last_insert_rowid as i64, as sqlx-sqlite does ([#14](https://github.com/sqlx-contrib/sqlx-cloudflare/issues/14)) ([3a66fb1](https://github.com/sqlx-contrib/sqlx-cloudflare/commit/3a66fb172600bf9f9a5e7c2400a69cc5e922ce6e))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * sqlx-cloudflare-core bumped from 0.1.0 to 0.2.0

## [0.1.0](https://github.com/sqlx-contrib/sqlx-cloudflare/compare/sqlx-cloudflare-d1-v0.1.0...sqlx-cloudflare-d1-v0.1.0) (2026-09-26)


### Features

* **d1:** add D1Connection::fetch_batch, streaming each statement's rows ([ebd4e1f](https://github.com/sqlx-contrib/sqlx-cloudflare/commit/ebd4e1ff6c301aa95d0696d996a32f5620f3d753)), closes [#4](https://github.com/sqlx-contrib/sqlx-cloudflare/issues/4)
