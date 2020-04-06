#!/bin/bash

set -ex

cargo clean --doc

cargo fmt --all -- --check
# NOTE: Tests are run in single threaded mode to prevent DB race conditions
cargo test --release -- --test-threads=1
cargo test --release --all-features -- --test-threads=1
cargo bench --no-run

cargo +nightly doc --all-features
linkchecker target/doc/event_sauce/index.html
# linkchecker target/doc/event_sauce_derive/index.html
linkchecker target/doc/event_sauce_storage_sqlx/index.html
