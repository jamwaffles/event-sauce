# Event sauce

[![Build Status](https://circleci.com/gh/jamwaffles/event-sauce/tree/master.svg?style=shield)](https://circleci.com/gh/jamwaffles/event-sauce/tree/master)

The event sourcing paradigm for Rust

- [`event-sauce`](event-sauce) [![Docs.rs](https://docs.rs/event-sauce/badge.svg)](https://docs.rs/event-sauce) - the main event sourcing crate
- [`event-sauce-derive`](event-sauce-derive) [![Docs.rs](https://docs.rs/event-sauce-derive/badge.svg)](https://docs.rs/event-sauce-derive) - derives for easier implementation
- [`event-sauce-storage-sqlx`](event-sauce-storage-sqlx) [![Docs.rs](https://docs.rs/event-sauce-storage-sqlx/badge.svg)](https://docs.rs/event-sauce-storage-sqlx) - [sqlx](https://crates.io/crates/sqlx) storage backend interface for event-sauce.

## Build environment

1. Install [rustup](https://rustup.rs)
1. Install rustfmt with `rustup component add rustfmt`
1. Install the nightly toolchain with `rustup toolchain add nightly`
1. Install `cargo-readme`
1. Install `linkchecker` with `pip install linkchecker`

To run integration tests, Postgres must be running:

```bash
docker-compose up -d
```

You can connect to it at <postgres://sauce:sauce@localhost:5432/sauce>

To emulate a full CI build, run:

```bash
./build.sh
```

Other useful commands:

- `cargo build` - build the crate
- `cargo doc` - generate documentation
- `cargo test --lib` or `cargo test --doc` - run lib or doc tests respectively, does not require Postgres server
- `cargo test` - run all tests (requires local Postgres server to be running)
