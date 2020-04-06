#!/bin/bash

set -ex

crates=("event-sauce" "storage-sqlx")

cargo clean --doc

cargo fmt --all -- --check
# NOTE: Tests are run in single threaded mode to prevent DB race conditions
cargo test --release -- --test-threads=1
cargo test --release --all-features -- --test-threads=1
cargo bench --no-run

cargo +nightly doc --all-features

# Crate-specific checks
for crate in ${crates[@]}; do
    linkchecker target/doc/event_sauce/index.html

    readme="target/check-${crate}-README.md"

    pushd $crate
    cargo readme --no-title --no-badges --no-indent-headings > "../$readme"
    popd

    diff "${crate}/README.md" "$readme"
done
