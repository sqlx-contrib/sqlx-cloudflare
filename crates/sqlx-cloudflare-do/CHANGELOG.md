# Changelog

## [0.3.0](https://github.com/sqlx-contrib/sqlx-cloudflare/compare/sqlx-cloudflare-do-v0.2.0...sqlx-cloudflare-do-v0.3.0) (2026-10-01)


### ⚠ BREAKING CHANGES

* `last_insert_rowid()` returns `i64`, and 0 where it returned `None`. `QueryResult::new` takes `last_insert_rowid: i64`.

### Features

* return last_insert_rowid as i64, as sqlx-sqlite does ([#14](https://github.com/sqlx-contrib/sqlx-cloudflare/issues/14)) ([3a66fb1](https://github.com/sqlx-contrib/sqlx-cloudflare/commit/3a66fb172600bf9f9a5e7c2400a69cc5e922ce6e))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * sqlx-cloudflare-core bumped from 0.1.0 to 0.2.0

## [0.2.0](https://github.com/sqlx-contrib/sqlx-cloudflare/compare/sqlx-cloudflare-do-v0.1.0...sqlx-cloudflare-do-v0.2.0) (2026-09-26)


### Features

* **do:** add DoConnection::transaction, interactive transactions ([1f14646](https://github.com/sqlx-contrib/sqlx-cloudflare/commit/1f146460df5b62486e16e8676e6cb2fe64eea138)), closes [#7](https://github.com/sqlx-contrib/sqlx-cloudflare/issues/7)

## [0.1.0](https://github.com/sqlx-contrib/sqlx-cloudflare/compare/sqlx-cloudflare-do-v0.1.0...sqlx-cloudflare-do-v0.1.0) (2026-09-26)


### Features

* **do:** add DoConnection::fetch_batch, streaming each statement's rows ([25f11ca](https://github.com/sqlx-contrib/sqlx-cloudflare/commit/25f11ca318d5d3adb69611b151fca2be9578e90a)), closes [#4](https://github.com/sqlx-contrib/sqlx-cloudflare/issues/4)
* **do:** add sqlx-cloudflare-do, a driver for Durable Object SQL storage ([ce8278d](https://github.com/sqlx-contrib/sqlx-cloudflare/commit/ce8278de2ecd2983c0d007a55da6e4e27eeda9ea))
