#!/bin/bash
# Build the self-contained Python runtime that release builds ship inside
# Tracet.app/Contents/Resources/python.
#
# Usage: scripts/build-runtime.sh [out_dir]
#        (default: ~/Library/Caches/tracet-build/runtime)
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
# Not inside the project: ~/Documents is iCloud-synced, and iCloud once
# filled the runtime with 45,913 "name 2.ext" conflict copies that the next
# build shipped. ~/Library/Caches is never synced.
CACHE="$HOME/Library/Caches/tracet-build"
OUT="${1:-$CACHE/runtime}"
LOCK="$ROOT/sidecar/requirements.lock"

PBS_TAG="20261124"
PBS_FILE="cpython-3.12.16+${PBS_TAG}-aarch64-apple-darwin-install_only_stripped.tar.gz"
PBS_SHA256="9d6d9d0d85feda3f9cb4eb3e048f0fed21ed280feda087927eaa4fbc7124826f"
PBS_URL="https://github.com/astral-sh/python-build-standalone/releases/download/${PBS_TAG}/${PBS_FILE}"

STAMP="$(cat "${BASH_SOURCE[0]}" "$SCRIPT_DIR/build-pyav.sh" "$LOCK" | shasum -a 256 | cut -d' ' -f1)"
if [ -f "$OUT/.stamp" ] && [ "$(cat "$OUT/.stamp")" = "$STAMP" ]; then
    echo "Runtime up to date: $OUT"
    exit 0
fi

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

# PyAV comes from scripts/build-pyav.sh (decode-only LGPL FFmpeg), not PyPI,
# whose FFmpeg links the GPL x264/x265 encoders.
echo "==> Building PyAV against a decode-only LGPL FFmpeg"
AV_WHEEL="$("$SCRIPT_DIR/build-pyav.sh" "$PY")"
FFMPEG_LICENSE="$(dirname "$(dirname "$AV_WHEEL")")/ffmpeg/COPYING.LGPLv2.1"

echo "==> Installing locked dependencies"
grep -v '^av==' "$LOCK" > "$OUT/requirements-no-av.txt"
PYTHONNOUSERSITE=1 "$PY" -m pip install --no-cache-dir --disable-pip-version-check \
    --quiet -r "$OUT/requirements-no-av.txt" "$AV_WHEEL"
rm "$OUT/requirements-no-av.txt"
# Fail if a GPL FFmpeg slipped in (e.g. pip replaced the wheel from PyPI).
PYTHONNOUSERSITE=1 "$PY" - <<'PYEOF'
import ctypes, glob, os, sys, av
libs = os.path.join(os.path.dirname(av.__file__), ".dylibs")
codec = ctypes.CDLL(glob.glob(os.path.join(libs, "libavcodec*.dylib"))[0])
codec.avcodec_license.restype = ctypes.c_char_p
lic = codec.avcodec_license().decode()
if not lic.startswith("LGPL") or glob.glob(os.path.join(libs, "libx26*")):
    sys.exit(f"PyAV's FFmpeg is not the decode-only LGPL build: {lic}")
PYEOF
# Fail the build, not the user's first transcription, if the stack is broken.
PYTHONNOUSERSITE=1 "$PY" -c "import whisperx, torch, av; from whisperx.diarize import DiarizationPipeline"

echo "==> Trimming"
STDLIB="$OUT/python/lib/python3.12"
rm -rf "$STDLIB/test" "$STDLIB/idlelib" "$STDLIB/tkinter" "$STDLIB/turtledemo" \
       "$STDLIB/ensurepip" "$STDLIB/lib-dynload/_tkinter"*.so \
       "$OUT/python/lib/tcl"* "$OUT/python/lib/tk"* "$OUT/python/lib/itcl"* \
       "$OUT/python/lib/thread"* "$OUT/python/share"
# Packages' own test suites: ~146 MB that users never run, and joblib's
# test data (.gz pickles) draws "could not be unpacked" notary warnings.
find "$OUT/python/lib/python3.12/site-packages" -type d \( -name tests -o -name test \) -prune \
    -exec rm -rf {} +
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

# Speaker diarization model, shipped so users never need a HuggingFace token.
# pyannote/speaker-diarization-community-1 is CC-BY-4.0 (redistribution with
# attribution); the HF repo is gated, so only this build step needs a token
# (HF_TOKEN, or HF_TOKEN_FILE pointing at a file holding one).
DIAR_REPO="pyannote/speaker-diarization-community-1"
DIAR_REV="3533c8cf8e369892e6b79ff1bf80f7b0286a54ee"
DIAR_CACHE="$CACHE/models/speaker-diarization-community-1-$DIAR_REV"
if [ ! -f "$DIAR_CACHE/config.yaml" ]; then
    echo "==> Downloading $DIAR_REPO@${DIAR_REV:0:8}"
    if [ -z "${HF_TOKEN:-}" ] && [ -n "${HF_TOKEN_FILE:-}" ]; then
        HF_TOKEN="$(tr -d '[:space:]' < "${HF_TOKEN_FILE/#\~/$HOME}")"
    fi
    if [ -z "${HF_TOKEN:-}" ]; then
        echo "ERROR: set HF_TOKEN or HF_TOKEN_FILE (a read token with the $DIAR_REPO conditions accepted)" >&2
        exit 1
    fi
    rm -rf "$DIAR_CACHE.part"
    HF_TOKEN="$HF_TOKEN" PYTHONNOUSERSITE=1 "$PY" - "$DIAR_REPO" "$DIAR_REV" "$DIAR_CACHE.part" <<'PYEOF'
import os, sys
from huggingface_hub import snapshot_download
repo, rev, dest = sys.argv[1:4]
snapshot_download(repo, revision=rev, local_dir=dest, token=os.environ["HF_TOKEN"])
PYEOF
    rm -rf "$DIAR_CACHE.part/.cache"
    mv "$DIAR_CACHE.part" "$DIAR_CACHE"
fi
DIAR_OUT="$OUT/models/pyannote-speaker-diarization-community-1"
mkdir -p "$OUT/models"
cp -R "$DIAR_CACHE" "$DIAR_OUT"
cat > "$DIAR_OUT/ATTRIBUTION.txt" <<EOF
pyannote/speaker-diarization-community-1 (revision $DIAR_REV)
by pyannoteAI, https://huggingface.co/$DIAR_REPO
Licensed under Creative Commons Attribution 4.0 International (CC BY 4.0):
https://creativecommons.org/licenses/by/4.0/
Redistributed unmodified with Tracet.
EOF
# Offline smoke test: the bundled copy must diarize without a token.
PYTHONNOUSERSITE=1 HF_HUB_OFFLINE=1 "$PY" - "$DIAR_OUT" <<'PYEOF'
import sys, warnings
warnings.filterwarnings("ignore")
from whisperx.diarize import DiarizationPipeline
DiarizationPipeline(model_name=sys.argv[1], token=None, device="cpu")
PYEOF

# Third-party notices, shipped as Resources/THIRD_PARTY_NOTICES.txt. Each
# package's own license files stay in its *.dist-info inside the bundle.
mkdir -p "$OUT/python/licenses"
cp "$FFMPEG_LICENSE" "$OUT/python/licenses/FFmpeg-LGPL-2.1.txt"
PYTHONNOUSERSITE=1 "$PY" - "$OUT/THIRD_PARTY_NOTICES.txt" "$DIAR_REV" <<'PYEOF'
import sys
from importlib.metadata import distributions
out, diar_rev = sys.argv[1:3]

def license_of(dist):
    meta = dist.metadata
    lic = meta.get("License-Expression") or ""
    if not lic:
        raw = (meta.get("License") or "").strip()
        # Some packages paste the whole license text into this field, or "UNKNOWN".
        lic = raw if raw and raw != "UNKNOWN" and "\n" not in raw and len(raw) < 80 else ""
    if not lic:
        lic = ", ".join(c.split(" :: ")[-1] for c in meta.get_all("Classifier") or []
                        if c.startswith("License ::"))
    if not lic:
        # Fall back to the license file's first line ("MIT License", ...).
        for f in dist.files or []:
            if "dist-info" in str(f) and f.name.upper().startswith(("LICENSE", "LICENCE", "COPYING")):
                first = next((l.strip() for l in f.locate().read_text(errors="replace").splitlines() if l.strip()), "")
                lic = first[:60]
                break
    return lic or "see the license files in its dist-info folder"

pkgs = sorted({(d.metadata["Name"], d.version, license_of(d)) for d in distributions()},
              key=lambda p: p[0].lower())
with open(out, "w") as f:
    f.write(f"""Tracet third-party notices

Tracet itself is MIT licensed: https://github.com/theagitist/tracet

Tracet bundles the following third-party software.

Python 3.12 (python-build-standalone)
  Python Software Foundation License; text in python/lib/python3.12/LICENSE.txt
  https://www.python.org/ and https://github.com/astral-sh/python-build-standalone

FFmpeg 8.0.1 (libavcodec, libavformat, libavutil, libavfilter, libavdevice,
libswresample, libswscale; inside python/lib/python3.12/site-packages/av/.dylibs)
  GNU Lesser General Public License, version 2.1 or later; text in
  python/licenses/FFmpeg-LGPL-2.1.txt. Built as a decode-only LGPL library
  without GPL or nonfree components by scripts/build-pyav.sh in the Tracet
  repository, from the unmodified source at
  https://ffmpeg.org/releases/ffmpeg-8.0.1.tar.xz

pyannote/speaker-diarization-community-1 (revision {diar_rev})
  by pyannoteAI, Creative Commons Attribution 4.0 International (CC BY 4.0),
  https://huggingface.co/pyannote/speaker-diarization-community-1
  Redistributed unmodified; see models/.../ATTRIBUTION.txt

Python packages (license files in python/lib/python3.12/site-packages/<name>-<version>.dist-info):

""")
    for name, version, lic in pkgs:
        f.write(f"  {name} {version}: {lic}\n")
PYEOF

echo "$STAMP" > "$OUT/.stamp"
echo "Runtime ready: $OUT ($(du -sh "$OUT/python" | cut -f1))"
