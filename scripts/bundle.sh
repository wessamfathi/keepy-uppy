#!/bin/sh
# Build "target/release/Keepy Uppy.app", ad-hoc signed for local use.
# Developer ID signing and notarization happen in the release script.
set -eu
cd "$(dirname "$0")/.."

cargo build --release
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
app="target/release/Keepy Uppy.app"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS"
cp target/release/keepy-uppy "$app/Contents/MacOS/"
sed "s/@VERSION@/$version/g" packaging/Info.plist > "$app/Contents/Info.plist"
plutil -lint -s "$app/Contents/Info.plist"
codesign --force --sign - "$app"

echo "$app"
