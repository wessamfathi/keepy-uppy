#!/bin/sh
# Set the app version for the next release and commit it.
# Usage: sh scripts/bump-version.sh 0.1.1
set -eu
cd "$(dirname "$0")/.."

new=${1:?usage: sh scripts/bump-version.sh <new-version, e.g. 0.1.1>}
echo "$new" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || { echo "version must look like 1.2.3" >&2; exit 1; }
old=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)

# Must be greater than the current version (sort -V puts the larger one last).
[ "$new" != "$old" ] && [ "$(printf '%s\n%s\n' "$old" "$new" | sort -V | tail -n 1)" = "$new" ] \
    || { echo "$new is not greater than $old" >&2; exit 1; }

sed -i '' "s/^version = \"$old\"/version = \"$new\"/" Cargo.toml
cargo update -q --workspace   # refresh Cargo.lock's entry for this crate only
git add Cargo.toml Cargo.lock
git commit -q -m "Version $new"
echo "$old -> $new"
