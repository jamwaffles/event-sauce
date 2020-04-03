#!/usr/bin/env bash

set -ex

crates=("event-sauce" "storage-sqlx")

for crate in ${crates[@]}; do
    pushd $crate
    cargo readme --no-title --no-badges --no-indent-headings > README.md
    popd
done

