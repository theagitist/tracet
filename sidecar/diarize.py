#!/usr/bin/env python3
"""Transcription + diarization sidecar for Tracet.

Uses whisperX for high-quality transcription with word-level timestamps,
phoneme alignment, and integrated pyannote speaker diarization.

Protocol (JSON over stdin/stdout):
  Input:  {"wav_path": "...", "hf_token": "...", "model_size": "large-v3", "language": null}
  Output: {"segments": [...], "speakers": [...], "language": "en"}

Each segment:
  {"start": 0.0, "end": 2.5, "text": "...", "speaker": "SPEAKER_00",
   "words": [{"word": "hello", "start": 0.0, "end": 0.3, "score": 0.95, "speaker": "SPEAKER_00"}]}
"""

import json
import logging
import os
import sys


# Marker used to identify our JSON output line. Some libraries (whisperX/pyannote)
# write log lines to stdout, so we prefix our payload to disambiguate.
RESULT_MARKER = "###TRACET_RESULT###"


def main():
    # Force all Python logging output to stderr: many ML libraries default to
    # stdout, which corrupts our JSON-over-stdout protocol.
    logging.basicConfig(stream=sys.stderr, force=True)

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            request = json.loads(line)
        except json.JSONDecodeError as e:
            print(f"{RESULT_MARKER}{json.dumps({'error': f'Invalid JSON: {e}'})}", flush=True)
            continue

        wav_path = request.get("wav_path")
        hf_token = request.get("hf_token") or os.environ.get("HF_TOKEN")
        model_size = request.get("model_size", "large-v3")
        language = request.get("language")  # None = auto-detect

        if not wav_path:
            print(f"{RESULT_MARKER}{json.dumps({'error': 'wav_path is required'})}", flush=True)
            continue

        if not os.path.exists(wav_path):
            print(f"{RESULT_MARKER}{json.dumps({'error': f'File not found: {wav_path}'})}", flush=True)
            continue

        try:
            result = run_transcription(wav_path, hf_token, model_size, language)
            stop_heartbeat()
            print(f"{RESULT_MARKER}{json.dumps(result)}", flush=True)
        except Exception as e:
            stop_heartbeat()
            print(f"{RESULT_MARKER}{json.dumps({'error': str(e)})}", flush=True)


def progress(percent: int, message: str) -> None:
    """Emit a structured progress event to stderr that Rust will parse."""
    sys.stderr.write(f"TRACET_PROGRESS: {percent} {message}\n")
    sys.stderr.flush()


# Heartbeat: while a long operation is running, emit a "still working" message
# every few seconds so the UI knows the process isn't frozen.
import threading
import time

_heartbeat_stop: threading.Event | None = None
_heartbeat_thread: threading.Thread | None = None


def start_heartbeat(percent: int, base_message: str) -> None:
    """Start emitting periodic 'still working' messages with elapsed time."""
    global _heartbeat_stop, _heartbeat_thread
    stop_heartbeat()  # ensure any previous one is stopped

    _heartbeat_stop = threading.Event()
    started = time.monotonic()

    def loop(stop_event: threading.Event):
        while not stop_event.wait(3.0):
            elapsed = int(time.monotonic() - started)
            mins, secs = divmod(elapsed, 60)
            elapsed_str = f"{mins}m {secs:02d}s" if mins else f"{secs}s"
            progress(percent, f"{base_message} ({elapsed_str} elapsed, still working...)")

    _heartbeat_thread = threading.Thread(target=loop, args=(_heartbeat_stop,), daemon=True)
    _heartbeat_thread.start()


def stop_heartbeat() -> None:
    global _heartbeat_stop, _heartbeat_thread
    if _heartbeat_stop is not None:
        _heartbeat_stop.set()
    if _heartbeat_thread is not None:
        _heartbeat_thread.join(timeout=0.5)
    _heartbeat_stop = None
    _heartbeat_thread = None


def run_transcription(
    wav_path: str,
    hf_token: str | None,
    model_size: str,
    language: str | None,
) -> dict:
    """Run whisperX transcription + alignment + diarization."""
    progress(31, "Importing PyTorch and whisperX (heavy modules)")
    start_heartbeat(31, "Importing dependencies")

    import whisperx
    import torch

    stop_heartbeat()
    progress(33, f"Loading whisper {model_size} weights into memory")
    start_heartbeat(33, f"Loading whisper {model_size} weights")

    transcribe_device = "cpu"
    transcribe_compute_type = "int8"

    torch_device = "cpu"
    if torch.backends.mps.is_available():
        torch_device = "mps"

    model = whisperx.load_model(
        model_size,
        device=transcribe_device,
        compute_type=transcribe_compute_type,
        language=language,
    )
    stop_heartbeat()

    progress(45, "Transcribing audio")
    start_heartbeat(45, "Transcribing audio")
    audio = whisperx.load_audio(wav_path)
    result = model.transcribe(audio, batch_size=16)
    stop_heartbeat()

    detected_language = result.get("language", language or "en")
    progress(60, f"Transcription complete (detected: {detected_language})")

    del model

    progress(62, f"Loading {detected_language} alignment model")
    start_heartbeat(62, f"Loading {detected_language} alignment model")
    try:
        align_model, align_metadata = whisperx.load_align_model(
            language_code=detected_language,
            device=torch_device,
        )
        stop_heartbeat()
        progress(70, "Aligning words to audio for precise timestamps")
        start_heartbeat(70, "Aligning words")
        result = whisperx.align(
            result["segments"],
            align_model,
            align_metadata,
            audio,
            torch_device,
            return_char_alignments=False,
        )
        stop_heartbeat()
        del align_model
    except Exception as e:
        stop_heartbeat()
        sys.stderr.write(f"Alignment on {torch_device} failed: {e}, retrying on CPU\n")
        torch_device = "cpu"
        align_model, align_metadata = whisperx.load_align_model(
            language_code=detected_language,
            device=torch_device,
        )
        result = whisperx.align(
            result["segments"],
            align_model,
            align_metadata,
            audio,
            torch_device,
            return_char_alignments=False,
        )
        del align_model

    progress(80, "Alignment complete")

    unique_speakers = set()
    if hf_token:
        try:
            progress(82, "Identifying speakers (diarization)")
            start_heartbeat(82, "Identifying speakers")
            diarize_model = whisperx.DiarizationPipeline(
                use_auth_token=hf_token,
                device=torch_device,
            )
            diarize_segments = diarize_model(audio)
            result = whisperx.assign_word_speakers(diarize_segments, result)
            stop_heartbeat()
            del diarize_model
            progress(88, "Speaker identification complete")
        except Exception as e:
            stop_heartbeat()
            sys.stderr.write(f"Diarization failed: {e}\n")

    # Keep heartbeat going through finalization (formatting + JSON output) so
    # the UI doesn't appear stuck at 88% during the post-processing.
    progress(89, "Finalizing transcript")
    start_heartbeat(89, "Finalizing transcript")

    # Step 4: Format output
    segments = []
    for seg in result.get("segments", []):
        speaker = seg.get("speaker")
        if speaker:
            unique_speakers.add(speaker)

        words = []
        for w in seg.get("words", []):
            word_speaker = w.get("speaker")
            if word_speaker:
                unique_speakers.add(word_speaker)
            words.append({
                "word": w.get("word", ""),
                "start": round(w.get("start", 0), 3),
                "end": round(w.get("end", 0), 3),
                "score": round(w.get("score", 0), 4),
                "speaker": word_speaker,
            })

        segments.append({
            "start": round(seg.get("start", 0), 3),
            "end": round(seg.get("end", 0), 3),
            "text": seg.get("text", "").strip(),
            "speaker": speaker,
            "words": words,
        })

    speakers = sorted(unique_speakers)

    return {
        "segments": segments,
        "speakers": speakers,
        "language": detected_language,
    }


if __name__ == "__main__":
    main()
