#!/bin/sh
# Build a Developer ID signed, notarized, stapled DMG.
#
#   DEVELOPER_ID="Developer ID Application: Your Name (TEAMID)" \
#   NOTARY_PROFILE=keepy-uppy \
#   sh scripts/release.sh
#
# One-time setup for NOTARY_PROFILE (stores credentials in the keychain):
#   xcrun notarytool store-credentials keepy-uppy \
#     --apple-id you@example.com --team-id TEAMID --password <app-specific password>
set -eu
cd "$(dirname "$0")/.."

: "${DEVELOPER_ID:?set DEVELOPER_ID to your Developer ID Application identity}"
: "${NOTARY_PROFILE:?set NOTARY_PROFILE to a notarytool keychain profile}"

app=$(sh scripts/bundle.sh | tail -n 1)
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
dmg="target/release/keepy-uppy-$version.dmg"

# Hardened runtime and a secure timestamp are required for notarization.
codesign --force --options runtime --timestamp --sign "$DEVELOPER_ID" "$app"
codesign --verify --strict --verbose=2 "$app"

staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT
cp -R "$app" "$staging/"
ln -s /Applications "$staging/Applications"
rm -f "$dmg"
hdiutil create -volname "Keepy Uppy" -srcfolder "$staging" -format UDZO "$dmg"
codesign --timestamp --sign "$DEVELOPER_ID" "$dmg"

xcrun notarytool submit "$dmg" --keychain-profile "$NOTARY_PROFILE" --wait
xcrun stapler staple "$dmg"
spctl --assess --type open --context context:primary-signature --verbose "$dmg"

echo "$dmg"
