# Changelog

## [0.2.0](https://github.com/sqlx-contrib/sqlx-cloudflare/compare/sqlx-cloudflare-core-v0.1.0...sqlx-cloudflare-core-v0.2.0) (2026-10-01)


### ⚠ BREAKING CHANGES

* `last_insert_rowid()` returns `i64`, and 0 where it returned `None`. `QueryResult::new` takes `last_insert_rowid: i64`.

### Features

* return last_insert_rowid as i64, as sqlx-sqlite does ([#14](https://github.com/sqlx-contrib/sqlx-cloudflare/issues/14)) ([3a66fb1](https://github.com/sqlx-contrib/sqlx-cloudflare/commit/3a66fb172600bf9f9a5e7c2400a69cc5e922ce6e))

## [0.1.0](https://github.com/sqlx-contrib/sqlx-cloudflare/compare/sqlx-cloudflare-core-v0.1.0...sqlx-cloudflare-core-v0.1.0) (2026-09-26)


### Features

* **core:** add BatchResult, one batch statement's rows and result ([190f08a](https://github.com/sqlx-contrib/sqlx-cloudflare/commit/190f08a85f503ba3aaf699be0825d02b2c914ddb))
* **core:** add sqlx-cloudflare-core, shared by the SQLite drivers ([4e96045](https://github.com/sqlx-contrib/sqlx-cloudflare/commit/4e96045dfe7882a673fb65994cf8f09ab179f7d1))
