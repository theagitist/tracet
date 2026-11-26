#!/usr/bin/env python3
"""Convert any audio/video file to 16 kHz mono 16-bit WAV.

Usage: convert.py <input> <output.wav>

Emits "TRACET_PROGRESS: <percent> <message>" lines on stderr, like
diarize.py. On failure the last stderr line is "TRACET_ERROR: <message>"
and the exit code is 1.
"""

import sys

from audio_io import decode, write_wav


def progress(percent):
    sys.stderr.write(f"TRACET_PROGRESS: {percent} Converting media ({percent}%)\n")
    sys.stderr.flush()


def main():
    src, dst = sys.argv[1], sys.argv[2]
    samples = decode(src, progress)
    write_wav(dst, samples)
    progress(100)


if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        sys.stderr.write(f"TRACET_ERROR: {e}\n")
        sys.exit(1)
