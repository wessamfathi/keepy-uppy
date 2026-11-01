#!/bin/sh
# Build "target/release/Keepy Uppy.app" as a universal (Apple Silicon + Intel)
# app, ad-hoc signed for local use. Needs: rustup target add x86_64-apple-darwin
# Developer ID signing and notarization happen in the release script.
set -eu
cd "$(dirname "$0")/.."

for target in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --release --target "$target"
done
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
app="target/release/Keepy Uppy.app"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
lipo -create -output "$app/Contents/MacOS/keepy-uppy" \
    target/aarch64-apple-darwin/release/keepy-uppy \
    target/x86_64-apple-darwin/release/keepy-uppy
cp assets/AppIcon.icns "$app/Contents/Resources/"
sed "s/@VERSION@/$version/g" packaging/Info.plist > "$app/Contents/Info.plist"
plutil -lint -s "$app/Contents/Info.plist"
codesign --force --sign - "$app"

echo "$app"
