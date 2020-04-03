#!/bin/bash

set -ex

cargo clean --doc

cargo fmt --all -- --check
cargo test --release
cargo test --release --all-features
cargo bench --no-run

cargo +nightly doc --all-features
linkchecker target/doc/event_sauce/index.html
linkchecker target/doc/event_sauce_derive/index.html
