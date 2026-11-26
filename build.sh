#!/bin/bash
# Build a signed + notarized Tracet release .dmg (Apple Silicon only).
#
# Usage: ./build.sh
#
# The .app ships its own Python runtime (scripts/build-runtime.sh) in
# Contents/Resources/python. Intel is not supported: torch 2.8, which
# whisperX 3.8 needs, has no Intel macOS wheels.
#
# Why this isn't just `cargo tauri build`:
#   1. macOS stamps com.apple.FinderInfo (the custom-icon bit) on the .app,
#      and a file-sync daemon watching this project tree
#      (com.apple.fileprovider.fpfs#P) RE-ADDS it within seconds of any strip.
#      `codesign` rejects bundles carrying FinderInfo ("detritus not allowed").
#      So we copy the bundle into a non-synced mktemp dir (/var/folders/...)
#      and do all strip/sign/package/notarize/staple work THERE, where nothing
#      re-stamps the attributes, then copy the finished .dmg back.
#   2. Tauri bundles and signs atomically with no hook to strip xattrs first,
#      so we let Tauri build an UNSIGNED .app and sign it ourselves.
# Credentials come from ./signing.env (gitignored).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

TRIPLE="aarch64-apple-darwin"
DMG_ARCH="aarch64"
if [ "$#" -gt 0 ] && [ "$1" != "aarch64" ]; then
    echo "ERROR: only Apple Silicon (aarch64) builds are supported" >&2
    exit 1
fi

# --- node resolution (mirrors dev.sh; nvm lazy-load shim is broken) ----------
NODE_BIN=""
for v in $(ls -1 ~/.nvm/versions/node 2>/dev/null | sort -rV); do
    if [ -x "$HOME/.nvm/versions/node/$v/bin/node" ]; then
        NODE_BIN="$HOME/.nvm/versions/node/$v/bin"
        break
    fi
done
if [ -z "$NODE_BIN" ]; then
    if command -v node >/dev/null 2>&1; then
        NODE_BIN="$(dirname "$(command -v node)")"
    else
        echo "ERROR: No node installation found" >&2
        exit 1
    fi
fi
echo "Using node from: $NODE_BIN"
export PATH="$NODE_BIN:$PATH"
if [ ! -d node_modules ]; then
    echo "Installing npm dependencies..."
    "$NODE_BIN/npm" install
fi

# --- signing credentials -----------------------------------------------------
SIGN_ID=""
NOTARIZE=0
if [ -f signing.env ]; then
    # shellcheck disable=SC1091
    . ./signing.env
    SIGN_ID="${APPLE_SIGNING_IDENTITY:-}"
    if [ -n "${APPLE_API_KEY_PATH:-}" ] && [ -n "${APPLE_API_KEY:-}" ] && [ -n "${APPLE_API_ISSUER:-}" ]; then
        NOTARIZE=1
    fi
    # Hide signing vars from Tauri so it produces an UNSIGNED bundle.
    unset APPLE_SIGNING_IDENTITY APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD
fi

VERSION="$("$NODE_BIN/node" -p "require('./src-tauri/tauri.conf.json').version")"
ENTITLEMENTS="$SCRIPT_DIR/src-tauri/entitlements.plist"

# --- build unsigned .app -----------------------------------------------------
echo "==> Building unsigned app bundle ($TRIPLE)"
rustup target add "$TRIPLE" >/dev/null 2>&1 || true
cargo tauri build --target "$TRIPLE" --bundles app

APP="src-tauri/target/$TRIPLE/release/bundle/macos/Tracet.app"

echo "==> Building bundled Python runtime"
scripts/build-runtime.sh
RUNTIME="$SCRIPT_DIR/src-tauri/target/runtime/python"

if [ -z "$SIGN_ID" ]; then
    rm -rf "$APP/Contents/Resources/python"
    cp -c -R "$RUNTIME" "$APP/Contents/Resources/python"
    echo "No signing identity in signing.env; leaving unsigned bundle at $APP"
    exit 0
fi

# --- sign + package + notarize in a NON-SYNCED temp dir ----------------------
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
echo "==> Signing in non-synced work dir: $WORK"

cp -R "$APP" "$WORK/Tracet.app"
SAPP="$WORK/Tracet.app"
rm -rf "$SAPP/Contents/Resources/python"
cp -c -R "$RUNTIME" "$SAPP/Contents/Resources/python"
xattr -cr "$SAPP"

# Sign inside out instead of --deep: --deep does not treat the hundreds of
# .so/.dylib files under Resources/python as nested code, and notarization
# rejects any Mach-O that lacks a hardened-runtime signature and timestamp.
PYEXE="$SAPP/Contents/Resources/python/bin/python3.12"
MACHO_LIST="$WORK/macho.txt"
find "$SAPP/Contents/Resources/python" -type f \( -name '*.so' -o -name '*.dylib' -o -perm +111 \) -print0 \
    | xargs -0 file --no-pad | grep 'Mach-O' | cut -d: -f1 \
    | sed 's/ (for architecture .*)$//' | sort -u | grep -vxF "$PYEXE" > "$MACHO_LIST"
echo "==> Signing $(wc -l < "$MACHO_LIST" | tr -d ' ') nested Mach-O files"
tr '\n' '\0' < "$MACHO_LIST" \
    | xargs -0 -P 8 -n 25 codesign --force --options runtime --timestamp --sign "$SIGN_ID"
# The interpreter loads torch & co., so it carries the hardened-runtime
# exceptions; the main binary does too, as before.
codesign --force --options runtime --timestamp \
    --entitlements "$ENTITLEMENTS" --sign "$SIGN_ID" "$PYEXE"
codesign --force --options runtime --timestamp \
    --entitlements "$ENTITLEMENTS" --sign "$SIGN_ID" "$SAPP"
codesign --verify --deep --strict --verbose=2 "$SAPP"
# --verify --deep doesn't descend into Resources, so check the nested code.
BAD=0
while IFS= read -r f; do
    codesign --verify --strict "$f" 2>/dev/null || { echo "UNSIGNED: $f" >&2; BAD=1; }
done < <(cat "$MACHO_LIST"; echo "$PYEXE")
[ "$BAD" -eq 0 ] || { echo "ERROR: nested code failed verification" >&2; exit 1; }

echo "==> Packaging .dmg"
STAGING="$WORK/staging"
mkdir -p "$STAGING"
cp -R "$SAPP" "$STAGING/Tracet.app"
ln -s /Applications "$STAGING/Applications"
DMG_TMP="$WORK/Tracet.dmg"
hdiutil create -volname "Tracet" -srcfolder "$STAGING" -ov -format UDZO "$DMG_TMP" >/dev/null
codesign --force --timestamp --sign "$SIGN_ID" "$DMG_TMP"

if [ "$NOTARIZE" -eq 1 ]; then
    # The bundled runtime makes the upload ~600 MB, and the notary upload
    # sometimes dies with deadlineExceeded; retry before giving up, and keep
    # the signed dmg if it still fails so a retry doesn't need a rebuild.
    echo "==> Submitting to Apple notary service (can take a few minutes)"
    NOTARIZED=0
    for attempt in 1 2 3; do
        if xcrun notarytool submit "$DMG_TMP" \
            --key "$APPLE_API_KEY_PATH" \
            --key-id "$APPLE_API_KEY" \
            --issuer "$APPLE_API_ISSUER" \
            --wait 2>&1 | tee "$WORK/notary.log" && grep -q 'status: Accepted' "$WORK/notary.log"; then
            NOTARIZED=1
            break
        fi
        echo "Notarization attempt $attempt failed" >&2
    done
    if [ "$NOTARIZED" -ne 1 ]; then
        KEEP="src-tauri/target/$TRIPLE/release/bundle/dmg/Tracet_${VERSION}_${DMG_ARCH}.unnotarized.dmg"
        mkdir -p "$(dirname "$KEEP")"
        cp "$DMG_TMP" "$KEEP"
        echo "ERROR: notarization failed; signed dmg kept at $KEEP" >&2
        exit 1
    fi
    echo "==> Stapling ticket"
    xcrun stapler staple "$DMG_TMP"
    xcrun stapler validate "$DMG_TMP"
else
    echo "WARNING: notarization creds missing; .dmg is signed but NOT notarized." >&2
fi

# --- copy finished .dmg back into the repo -----------------------------------
DMG_DIR="src-tauri/target/$TRIPLE/release/bundle/dmg"
mkdir -p "$DMG_DIR"
DMG="$DMG_DIR/Tracet_${VERSION}_${DMG_ARCH}.dmg"
cp "$DMG_TMP" "$DMG"

echo ""
if [ "$NOTARIZE" -eq 1 ]; then
    echo "Done. Signed + notarized + stapled artifact:"
else
    echo "Done. Signed (NOT notarized) artifact:"
fi
echo "  $DMG"
