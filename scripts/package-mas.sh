#!/bin/bash
# Sign and package the Mac App Store build. Called by `MAS=1 ./build.sh`,
# which has already built the unsigned .app and the bundled runtime.
#
# Usage: scripts/package-mas.sh <unsigned Tracet.app> <version>
#
# Env (from signing.env, exported by build.sh):
#   APPLE_MAS_APP_IDENTITY        "Apple Distribution: ..." (signs the app)
#   APPLE_MAS_INSTALLER_IDENTITY  "3rd Party Mac Developer Installer: ..."
#                                 (Keychain may call it "Mac Installer Distribution")
#   MAS_PROVISIONING_PROFILE      path to the Mac App Store Connect profile
#                                 for ca.polivoxia.tracet
#   MAS_UPLOAD=1                  also validate and upload to App Store Connect
#                                 (needs APPLE_API_KEY and APPLE_API_ISSUER; the
#                                 .p8 must be in ~/.appstoreconnect/private_keys)

set -euo pipefail

APP_IN="$1"
VERSION="$2"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUNTIME_DIR="$HOME/Library/Caches/tracet-build/runtime"
APP_ENT="$ROOT/src-tauri/entitlements.mas.plist"
CHILD_ENT="$ROOT/src-tauri/entitlements.mas-child.plist"

for v in APPLE_MAS_APP_IDENTITY APPLE_MAS_INSTALLER_IDENTITY MAS_PROVISIONING_PROFILE; do
    if [ -z "${!v:-}" ]; then
        echo "ERROR: $v is not set (add it to signing.env)" >&2
        exit 1
    fi
done
PROFILE="${MAS_PROVISIONING_PROFILE/#\~/$HOME}"
[ -f "$PROFILE" ] || { echo "ERROR: provisioning profile not found: $PROFILE" >&2; exit 1; }

# The profile must be for this app; a mismatch only surfaces at upload.
PROFILE_ID="$(security cms -D -i "$PROFILE" | plutil -extract Entitlements.com\\.apple\\.application-identifier raw -o - -)"
WANT_ID="$(/usr/libexec/PlistBuddy -c 'Print :com.apple.application-identifier' "$APP_ENT")"
if [ "$PROFILE_ID" != "$WANT_ID" ]; then
    echo "ERROR: profile is for $PROFILE_ID, the app is $WANT_ID" >&2
    exit 1
fi

# Same reason as build.sh: sign outside the file-synced project tree.
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
echo "==> Preparing the App Store bundle in $WORK"
cp -R "$APP_IN" "$WORK/Tracet.app"
APP="$WORK/Tracet.app"
RES="$APP/Contents/Resources"
rm -rf "$RES/python" "$RES/models"
cp -c -R "$RUNTIME_DIR/python" "$RES/python"
cp -c -R "$RUNTIME_DIR/models" "$RES/models"
cp "$RUNTIME_DIR/THIRD_PARTY_NOTICES.txt" "$RES/"
if find "$APP" \( -name '* [0-9].*' -o -name '* [0-9]' \) -print | grep -q .; then
    echo "ERROR: sync conflict copies inside the bundle" >&2
    exit 1
fi

PLIST="$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Add :LSApplicationCategoryType string public.app-category.productivity' "$PLIST" 2>/dev/null ||
    /usr/libexec/PlistBuddy -c 'Set :LSApplicationCategoryType public.app-category.productivity' "$PLIST"
# Only standard HTTPS (the model download): exempt from export documentation.
/usr/libexec/PlistBuddy -c 'Add :ITSAppUsesNonExemptEncryption bool false' "$PLIST" 2>/dev/null ||
    /usr/libexec/PlistBuddy -c 'Set :ITSAppUsesNonExemptEncryption false' "$PLIST"
cp "$PROFILE" "$APP/Contents/embedded.provisionprofile"
xattr -cr "$APP"

# Inside out, as in build.sh. No hardened runtime: the App Store doesn't
# require it, and the sandbox is the security boundary here.
PYEXE="$RES/python/bin/python3.12"
MACHO_LIST="$WORK/macho.txt"
find "$RES/python" -type f \( -name '*.so' -o -name '*.dylib' -o -perm +111 \) -print0 \
    | xargs -0 file --no-pad | grep 'Mach-O' | cut -d: -f1 \
    | sed 's/ (for architecture .*)$//' | sort -u | grep -vxF "$PYEXE" > "$MACHO_LIST"
echo "==> Signing $(wc -l < "$MACHO_LIST" | tr -d ' ') nested Mach-O files"
tr '\n' '\0' < "$MACHO_LIST" \
    | xargs -0 -P 8 -n 25 codesign --force --timestamp --sign "$APPLE_MAS_APP_IDENTITY"
codesign --force --timestamp --entitlements "$CHILD_ENT" --sign "$APPLE_MAS_APP_IDENTITY" "$PYEXE"
codesign --force --timestamp --entitlements "$APP_ENT" --sign "$APPLE_MAS_APP_IDENTITY" "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"
BAD=0
while IFS= read -r f; do
    codesign --verify --strict "$f" 2>/dev/null || { echo "UNSIGNED: $f" >&2; BAD=1; }
done < <(cat "$MACHO_LIST"; echo "$PYEXE")
[ "$BAD" -eq 0 ] || { echo "ERROR: nested code failed verification" >&2; exit 1; }

echo "==> Building the installer package"
OUT_DIR="$ROOT/src-tauri/target/aarch64-apple-darwin/release/bundle/mas"
mkdir -p "$OUT_DIR"
PKG="$OUT_DIR/Tracet-$VERSION.pkg"
productbuild --component "$APP" /Applications --sign "$APPLE_MAS_INSTALLER_IDENTITY" "$WORK/Tracet.pkg"
cp "$WORK/Tracet.pkg" "$PKG"
echo "Package: $PKG"

if [ "${MAS_UPLOAD:-0}" = "1" ]; then
    echo "==> Validating with App Store Connect"
    xcrun altool --validate-app -f "$PKG" -t macos \
        --apiKey "$APPLE_API_KEY" --apiIssuer "$APPLE_API_ISSUER"
    echo "==> Uploading (about 600 MB)"
    xcrun altool --upload-app -f "$PKG" -t macos \
        --apiKey "$APPLE_API_KEY" --apiIssuer "$APPLE_API_ISSUER"
fi
