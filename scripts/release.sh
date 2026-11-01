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

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
dmg="target/release/keepy-uppy-$version.dmg"

# Every release gets a new version. Bump first with scripts/bump-version.sh.
git fetch -q --tags origin
if git rev-parse -q --verify "refs/tags/v$version" >/dev/null; then
    echo "v$version is already released. Run: sh scripts/bump-version.sh <new-version>" >&2
    exit 1
fi

app=$(sh scripts/bundle.sh | tail -n 1)

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

sha=$(shasum -a 256 "$dmg" | cut -d' ' -f1)
cat >&2 <<EOM

Next:
  git tag -s v$version -m "Keepy Uppy $version" && git push origin v$version
  gh release create v$version "$dmg" --verify-tag --title "Keepy Uppy $version"
  In wessamfathi/homebrew-tap, Casks/keepy-uppy.rb:
    version "$version"
    sha256 "$sha"
EOM
echo "$dmg"
