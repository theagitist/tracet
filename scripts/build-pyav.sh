#!/bin/bash
# Build a PyAV wheel against a decode-only, LGPL FFmpeg and print its path.
#
# Usage: scripts/build-pyav.sh <python3.12>
#
# Why: PyPI's PyAV wheels bundle an FFmpeg linked with x264 and x265 (GPL
# encoders), which makes that FFmpeg GPL and rules it out of the Mac App
# Store. Tracet only decodes audio, so this FFmpeg has no external libraries
# and enables just the demuxers, audio decoders and parsers for the formats
# the app accepts (mp3 wav flac m4a ogg aac mp4 mkv avi webm mov), plus the
# filters PyAV's AudioResampler builds its graph from.
#
# Cached in ~/Library/Caches/tracet-build; rebuilt when this script changes.
# Progress goes to stderr; stdout is only the wheel path.

set -euo pipefail

PY="$1"
FFMPEG_VERSION="8.0.1"
FFMPEG_SHA256="05ee0b03119b45c0bdb4df654b96802e909e0a752f72e4fe3794f487229e5a41"
PYAV_VERSION="17.0.1"
# Match the app's minimumSystemVersion; without it clang targets the build
# machine's macOS and the dylibs refuse to load on older systems.
export MACOSX_DEPLOYMENT_TARGET="11.0"

CACHE="$HOME/Library/Caches/tracet-build"
KEY="$(shasum -a 256 "${BASH_SOURCE[0]}" | cut -c1-12)"
WORK="$CACHE/pyav-$KEY"
WHEEL_DIR="$WORK/wheel"

if ls "$WHEEL_DIR"/av-*.whl >/dev/null 2>&1; then
    ls "$WHEEL_DIR"/av-*.whl
    exit 0
fi

log() { echo "$@" >&2; }
rm -rf "$WORK"
mkdir -p "$WORK"

TARBALL="$CACHE/ffmpeg-$FFMPEG_VERSION.tar.xz"
if [ ! -f "$TARBALL" ]; then
    log "==> Downloading FFmpeg $FFMPEG_VERSION"
    curl -fL --retry 3 -o "$TARBALL.part" "https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VERSION.tar.xz"
    mv "$TARBALL.part" "$TARBALL"
fi
echo "$FFMPEG_SHA256  $TARBALL" | shasum -a 256 -c - >/dev/null

log "==> Building decode-only FFmpeg $FFMPEG_VERSION (LGPL)"
tar -xJf "$TARBALL" -C "$WORK"
PREFIX="$WORK/ffmpeg"
(
    cd "$WORK/ffmpeg-$FFMPEG_VERSION"
    # No --enable-gpl / --enable-nonfree: the result is LGPL 2.1+.
    # --disable-autodetect keeps out system libraries (zlib, iconv,
    # AudioToolbox, VideoToolbox), so nothing outside the bundle is linked.
    ./configure --prefix="$PREFIX" \
        --enable-shared --disable-static --enable-pic \
        --disable-programs --disable-doc --disable-network \
        --disable-autodetect --disable-everything \
        --enable-protocol=file,pipe \
        --enable-demuxer=mov,matroska,avi,mp3,wav,flac,ogg,aac \
        --enable-decoder=aac,aac_fixed,aac_latm,mp3,mp3float,mp2,mp2float,mp1,mp1float \
        --enable-decoder=flac,alac,vorbis,opus,ac3,eac3,wmav1,wmav2,amrnb,amrwb \
        --enable-decoder=pcm_s16le,pcm_s16be,pcm_s24le,pcm_s24be,pcm_s32le,pcm_s32be \
        --enable-decoder=pcm_f32le,pcm_f32be,pcm_f64le,pcm_u8,pcm_alaw,pcm_mulaw \
        --enable-decoder=adpcm_ima_wav,adpcm_ms \
        --enable-parser=aac,aac_latm,mpegaudio,flac,vorbis,opus,ac3 \
        --enable-filter=abuffer,abuffersink,aformat,aresample,anull \
        >&2
    make -j"$(sysctl -n hw.ncpu)" >&2
    make install >&2
)

log "==> Building PyAV $PYAV_VERSION against it"
"$PY" -m venv "$WORK/venv"
"$WORK/venv/bin/pip" install --quiet --disable-pip-version-check delocate >&2
PKG_CONFIG_PATH="$PREFIX/lib/pkgconfig" PYTHONNOUSERSITE=1 \
    "$WORK/venv/bin/pip" wheel --quiet --disable-pip-version-check --no-deps \
    --no-binary av "av==$PYAV_VERSION" -w "$WORK/raw" >&2

# Copy the FFmpeg dylibs into the wheel (av/.dylibs) with relative install
# names, the way PyPI's wheels are made.
DYLD_FALLBACK_LIBRARY_PATH="$PREFIX/lib" \
    "$WORK/venv/bin/delocate-wheel" -w "$WHEEL_DIR" "$WORK/raw"/av-*.whl >&2
# The notices file ships FFmpeg's license text from here.
cp "$WORK/ffmpeg-$FFMPEG_VERSION/COPYING.LGPLv2.1" "$PREFIX/"
rm -rf "$WORK/ffmpeg-$FFMPEG_VERSION" "$WORK/raw" "$WORK/venv"
ls "$WHEEL_DIR"/av-*.whl
