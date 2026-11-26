#!/bin/bash
# Build the self-contained Python runtime that release builds ship inside
# Tracet.app/Contents/Resources/python.
#
# Usage: scripts/build-runtime.sh [out_dir]   (default: src-tauri/target/runtime)
#
# Why bundle instead of building a venv on the user's Mac: the old first-run
# setup depended on Homebrew, on whichever `python3` happened to be first on
# PATH (3.9 from Apple, 3.15 from Homebrew: both break whisperX), and on
# whatever whisperX pip resolved that day (3.8 silently broke diarization).
# Shipping a pinned interpreter plus sidecar/requirements.lock removes all of
# that. Apple Silicon only: torch 2.8 has no Intel macOS wheels.
#
# The result is cached: it is rebuilt only when this script or the lock file
# changes. Signing happens later, in build.sh.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT="${1:-$ROOT/src-tauri/target/runtime}"
LOCK="$ROOT/sidecar/requirements.lock"

PBS_TAG="20261124"
PBS_FILE="cpython-3.12.16+${PBS_TAG}-aarch64-apple-darwin-install_only_stripped.tar.gz"
PBS_SHA256="9d6d9d0d85feda3f9cb4eb3e048f0fed21ed280feda087927eaa4fbc7124826f"
PBS_URL="https://github.com/astral-sh/python-build-standalone/releases/download/${PBS_TAG}/${PBS_FILE}"

STAMP="$(cat "${BASH_SOURCE[0]}" "$LOCK" | shasum -a 256 | cut -d' ' -f1)"
if [ -f "$OUT/.stamp" ] && [ "$(cat "$OUT/.stamp")" = "$STAMP" ]; then
    echo "Runtime up to date: $OUT"
    exit 0
fi

# Keep the download out of the project tree (it is file-synced).
CACHE="$HOME/Library/Caches/tracet-build"
mkdir -p "$CACHE"
if [ ! -f "$CACHE/$PBS_FILE" ]; then
    echo "==> Downloading $PBS_FILE"
    curl -fL --retry 3 -o "$CACHE/$PBS_FILE.part" "$PBS_URL"
    mv "$CACHE/$PBS_FILE.part" "$CACHE/$PBS_FILE"
fi
echo "$PBS_SHA256  $CACHE/$PBS_FILE" | shasum -a 256 -c - >/dev/null

echo "==> Extracting Python into $OUT"
rm -rf "$OUT"
mkdir -p "$OUT"
tar -xzf "$CACHE/$PBS_FILE" -C "$OUT"
PY="$OUT/python/bin/python3.12"

echo "==> Installing locked dependencies"
PYTHONNOUSERSITE=1 "$PY" -m pip install --no-cache-dir --disable-pip-version-check \
    --quiet -r "$LOCK"
# Fail the build, not the user's first transcription, if the stack is broken.
PYTHONNOUSERSITE=1 "$PY" -c "import whisperx, torch, av; from whisperx.diarize import DiarizationPipeline"

echo "==> Trimming"
STDLIB="$OUT/python/lib/python3.12"
rm -rf "$STDLIB/test" "$STDLIB/idlelib" "$STDLIB/tkinter" "$STDLIB/turtledemo" \
       "$STDLIB/ensurepip" "$STDLIB/lib-dynload/_tkinter"*.so \
       "$OUT/python/lib/tcl"* "$OUT/python/lib/tk"* "$OUT/python/lib/itcl"* \
       "$OUT/python/lib/thread"* "$OUT/python/share"
# Some wheels ship universal (arm64 + x86_64) binaries; drop the Intel half.
find "$OUT/python" -type f \( -name '*.so' -o -name '*.dylib' \) -print0 |
    while IFS= read -r -d '' f; do
        if lipo -archs "$f" 2>/dev/null | grep -q x86_64 && lipo -archs "$f" | grep -q arm64; then
            lipo -thin arm64 "$f" -output "$f.thin" && mv "$f.thin" "$f"
        fi
    done

# The bundle is read-only once signed, so precompile instead of letting
# Python try to write __pycache__ at runtime.
echo "==> Precompiling bytecode"
find "$OUT/python" -name __pycache__ -type d -prune -exec rm -rf {} +
"$PY" -m compileall -q -j 0 "$OUT/python/lib" >/dev/null || true

echo "$STAMP" > "$OUT/.stamp"
echo "Runtime ready: $OUT ($(du -sh "$OUT/python" | cut -f1))"
