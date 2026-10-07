#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo 'Usage: package-native-macos.sh <release-binary> <output-directory>' >&2
  exit 2
fi

binary="$1"
out="$2"
if [[ ! -f "$binary" ]]; then
  echo "Native release binary missing: $binary" >&2
  exit 1
fi

version="$(cargo metadata --locked --no-deps --format-version 1 --manifest-path src-native/Cargo.toml | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "neon-hud-native"))')"
short_version="${version%%-*}"
app="$out/Neon HUD Native.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$binary" "$app/Contents/MacOS/neon-hud-native"
chmod 755 "$app/Contents/MacOS/neon-hud-native"
cp LICENSE LICENSE-MIT LICENSE-APACHE NOTICE THIRD-PARTY-NOTICES.md "$app/Contents/Resources/"
cp -R src-native/licenses "$app/Contents/Resources/"
cp src-tauri/icons/icon.icns "$app/Contents/Resources/NeonHud.icns"

cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Neon HUD Native</string>
  <key>CFBundleDisplayName</key><string>Neon HUD Native</string>
  <key>CFBundleIdentifier</key><string>io.github.ajaxcbcb.neonhud.native</string>
  <key>CFBundleExecutable</key><string>neon-hud-native</string>
  <key>CFBundleIconFile</key><string>NeonHud</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$short_version</string>
  <key>CFBundleVersion</key><string>$short_version</string>
  <key>NeonHudNativeVersion</key><string>$version</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
EOF
cat > "$out/PREVIEW.txt" <<EOF
Neon HUD Native $version is an unsigned preview. Open the app from the DMG or ZIP.
Updates use the signed native-preview channel in Settings / Preferences / Startup & updates.
Copy the app to a writable Applications folder before applying updates.
EOF
cp "$out/PREVIEW.txt" "$app/Contents/Resources/PREVIEW.txt"

plutil -lint "$app/Contents/Info.plist"
ditto -c -k --keepParent "$app" "$out/neon-hud-native-macos-unsigned-preview.zip"
hdiutil create -volname 'Neon HUD Native Preview' -srcfolder "$app" -ov -format UDZO "$out/neon-hud-native-macos-unsigned-preview.dmg"
