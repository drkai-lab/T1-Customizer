#!/usr/bin/env bash
# Wrap the Rust release binary as a macOS .app and a CLI copy.
set -euo pipefail

arch="${1:?arch}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin="${root}/rust/target/release/t1-keyboard-config"
app="${root}/dist-mac/T1 Keyboard Config.app"

"$bin" --version
"$bin" --help >/dev/null

mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "${root}/dist-mac"
cp "$bin" "$app/Contents/MacOS/T1 Keyboard Config"
cp "${root}/icons/app-icon.png" "$app/Contents/Resources/app-icon.png"
cp -R "${root}/icons" "$app/Contents/Resources/icons"
cat > "$app/Contents/Info.plist" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>T1-VREmoKey</string>
  <key>CFBundleDisplayName</key><string>T1-VREmoKey</string>
  <key>CFBundleIdentifier</key><string>lab.drkai.t1-keyboard-config</string>
  <key>CFBundleVersion</key><string>2.2.4</string>
  <key>CFBundleShortVersionString</key><string>2.2.4</string>
  <key>CFBundleExecutable</key><string>T1 Keyboard Config</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
</dict></plist>
EOF
chmod +x "$app/Contents/MacOS/T1 Keyboard Config"
ditto -c -k --keepParent "$app" "${root}/dist-mac/T1-Keyboard-Config-${arch}.app.zip"
cp "$bin" "${root}/dist-mac/t1-keyboard-config-cli-${arch}"
"$app/Contents/MacOS/T1 Keyboard Config" --version
