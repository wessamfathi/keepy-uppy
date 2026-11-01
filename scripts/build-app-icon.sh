#!/bin/sh
# Rebuild assets/AppIcon.icns from scripts/draw-app-icon.swift.
set -eu
cd "$(dirname "$0")/.."

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
swift scripts/draw-app-icon.swift "$tmp/icon-1024.png"

set_dir="$tmp/AppIcon.iconset"
mkdir "$set_dir"
for pt in 16 32 128 256 512; do
    sips -z "$pt" "$pt" "$tmp/icon-1024.png" --out "$set_dir/icon_${pt}x${pt}.png" >/dev/null
    px=$((pt * 2))
    sips -z "$px" "$px" "$tmp/icon-1024.png" --out "$set_dir/icon_${pt}x${pt}@2x.png" >/dev/null
done
iconutil -c icns "$set_dir" -o assets/AppIcon.icns
echo assets/AppIcon.icns
