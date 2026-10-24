#!/bin/sh
# Compila rdAPP + ironrdp-viewer y genera rdAPP.app (y opcionalmente lo instala).
# Uso: scripts/bundle.sh [--install]
set -e
. "$HOME/.cargo/env" 2>/dev/null || true
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export MACOSX_DEPLOYMENT_TARGET=11.0
# cmake es necesario para compilar libopus (audio). pip lo instala en segundos.
for d in "$HOME"/Library/Python/*/bin; do [ -x "$d/cmake" ] && PATH="$d:$PATH"; done
export PATH
command -v cmake >/dev/null || { echo "Falta cmake: python3 -m pip install --user cmake"; exit 1; }

# Versión de IronRDP sobre la que están hechos los parches.
IRONRDP_REV=b69cda22ec692e212caffdbb9fe0c4e5ff1d2696
if [ ! -d vendor/IronRDP ]; then
  git init -q vendor/IronRDP
  git -C vendor/IronRDP remote add origin https://github.com/Devolutions/IronRDP.git
  git -C vendor/IronRDP fetch -q --depth 1 origin "$IRONRDP_REV"
  git -C vendor/IronRDP checkout -q FETCH_HEAD
fi
# Parches propios sobre IronRDP (se aplican una sola vez)
for p in patches/*.patch; do
  [ -f "$p" ] || continue
  git -C vendor/IronRDP apply --check "$ROOT/$p" 2>/dev/null && git -C vendor/IronRDP apply "$ROOT/$p" && echo "Parche aplicado: $p"
done

(cd vendor/IronRDP && cargo build --release -p ironrdp-viewer --features openh264)
cargo build --release

# Se construye y firma fuera de Documents (iCloud añade atributos que rompen codesign).
APP="$(mktemp -d)/rdAPP.app"
mkdir -p "$APP/Contents/MacOS"
cp target/release/rdapp "$APP/Contents/MacOS/rdapp"
cp vendor/IronRDP/target/release/ironrdp-viewer "$APP/Contents/MacOS/ironrdp-viewer"
strip -x "$APP/Contents/MacOS/ironrdp-viewer"

# Icono (.icns) desde assets/icon.png
mkdir -p "$APP/Contents/Resources"
ICONSET="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET"
for s in 16 32 128 256 512; do
  sips -z $s $s assets/icon.png --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
  d=$((s*2)); sips -z $d $d assets/icon.png --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>rdAPP</string>
  <key>CFBundleDisplayName</key><string>rdAPP</string>
  <key>CFBundleIdentifier</key><string>io.github.rdapp</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleExecutable</key><string>rdapp</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

xattr -cr "$APP"
codesign --force --deep -s - "$APP"

mkdir -p "$ROOT/dist"
rm -rf "$ROOT/dist/rdAPP.app"
ditto --norsrc "$APP" "$ROOT/dist/rdAPP.app"
echo "OK: $ROOT/dist/rdAPP.app"

if [ "$1" = "--install" ]; then
  mkdir -p "$HOME/Applications"
  rm -rf "$HOME/Applications/rdAPP.app"
  ditto --norsrc "$APP" "$HOME/Applications/rdAPP.app"
  codesign --verify "$HOME/Applications/rdAPP.app" && echo "Instalado en ~/Applications/rdAPP.app"
fi
rm -rf "$(dirname "$APP")"
