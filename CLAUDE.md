# CLAUDE.md

Project instructions for Claude Code working on Tracet.

## Project summary

Tracet is a local desktop transcription app: **Tauri v2** shell with a
**React/TypeScript** frontend and a **Rust** backend, plus a **Python sidecar**
running **whisperX** for transcription, alignment, and pyannote speaker
diarization. Optional LLM accuracy review is provided via a local Ollama
instance over HTTP. Quality is the top priority for technical decisions
unless explicitly traded against performance.

Target platform: macOS on Apple Silicon. Other platforms are not currently
supported.

## Stack at a glance

| Layer       | Tech                                          |
| ----------- | --------------------------------------------- |
| Shell       | Tauri v2 (Rust + WebView)                     |
| Frontend    | React 19 + TypeScript + Tailwind CSS 4 + Vite |
| State       | Zustand                                       |
| Transcribe  | whisperX (Python sidecar)                     |
| Diarize     | pyannote-audio (inside whisperX)              |
| LLM review  | Ollama HTTP API                               |
| Conversion  | FFmpeg (system binary, child process)         |
| Project I/O | `zip` crate, `.tracet` bundle = zip archive   |

## Run / build

```bash
./dev.sh                      # dev mode with hot reload
cargo tauri build             # release .app + .dmg
```

`./dev.sh` exists because some development setups have a broken nvm
lazy-load shim that breaks bare `npm`/`node`. It locates a usable node
binary by scanning `~/.nvm/versions/node` directly. **Do not assume `npm`
or `node` are on PATH.** Use the full binary path:
`/Users/adri.m/.nvm/versions/node/v25.9.0/bin/{node,npm,npx}`.

## Backend layout (`src-tauri/src/`)

- `commands/`: one file per Tauri command:
  - `audio.rs`: `read_audio_file` (custom replacement for fs plugin reads,
    used for blob-URL audio playback; bypasses fs plugin scope which is
    flaky on macOS temp paths)
  - `convert.rs`: FFmpeg conversion to 16 kHz mono WAV
  - `transcribe.rs`: drives the whisperX sidecar, parses results, applies
    auto-flagging heuristics
  - `diarize.rs`: currently a stub (whisperX handles diarization inline)
  - `review.rs`: Ollama LLM coherence review (graceful fallback when
    Ollama isn't reachable)
  - `export.rs`: `.txt` / `.md` formatting
  - `project.rs`: `save_project` / `load_project` for `.tracet` bundles
  - `setup.rs`: first-run dependency installer (FFmpeg, venv)
  - `hardware.rs`: chip / RAM detection, model profile filtering
- `pipeline/orchestrator.rs`: `run_pipeline` top-level command,
  composes convert + transcribe + review
- `sidecar/`: process launchers (`ffmpeg.rs`, `python.rs`)
- `models/transcript.rs`: `Transcript`, `TranscriptSegment`, `Speaker`,
  `PipelineOptions`, `PipelineProgress`, `ExportOptions`, etc.

## Frontend layout (`src/`)

- `components/`: `Toolbar`, `TranscriptViewer`, `TranscriptSegment`,
  `SpeakerPanel`, `ExportDialog`, `SettingsDialog`, `SetupScreen`,
  `ProgressIndicator`
- `hooks/`: `useTranscription` (open/save/export), `useSegmentPlayer`
  (shared audio player + status), `useSpeakers`
- `stores/transcriptStore.ts`: Zustand store
- `lib/settings.ts`: localStorage-backed settings
- `types/transcript.ts`: TS mirrors of Rust structs

## Python sidecar (`sidecar/`)

- `diarize.py`: single entry point. JSON-over-stdin/stdout protocol.
  Emits `TRACET_PROGRESS:` lines on stderr that Rust forwards to the UI.
  Heartbeat thread emits "(Ns elapsed, still working...)" every 3 s during
  long phases so the UI never appears frozen.
- `requirements.txt`: pinned to `whisperx>=3.3`, `torch`, `torchaudio`
- Venv must use **Python 3.12** (3.14 breaks the ML stack)
- The result line is prefixed with `###TRACET_RESULT###` because some
  libraries write log lines directly to stdout, polluting the JSON channel.

## Critical gotchas

1. **nvm lazy-load is broken on the dev machine.** Always use full binary
   paths for node/npm/npx, or use `./dev.sh`.
2. **Python 3.14 is too new.** The venv must be created with `python3.12`.
   `commands/setup.rs` and `dev.sh` both assume 3.12.
3. **faster-whisper does not support MPS.** Transcription runs on CPU
   with int8 quantisation. Alignment and diarization can use MPS.
4. **whisperX on first run downloads several GB.** Models cached at
   `~/.cache/huggingface/hub/` and `~/.cache/torch/hub/checkpoints/`.
   `large-v3-turbo` is the default: best speed/quality tradeoff.
5. **The fs plugin's path scope is unreliable for `/var/folders/...`** on
   macOS. Use the custom `read_audio_file` Tauri command for any reads
   from temp dirs, not `@tauri-apps/plugin-fs`.
6. **stdout is contended.** whisperX/pyannote sometimes write logs to
   stdout. The result JSON line is prefixed with `###TRACET_RESULT###`
   and Rust skips other lines.
7. **Diarization needs an HF token AND a license accepted** for
   `pyannote/speaker-diarization-3.1`. Without one, transcription works
   but speakers are not separated. The pipeline now warns *before*
   processing starts (was a previous UX problem).

## Progress reporting

The pipeline emits `pipeline:progress` events shaped like:

```ts
{ step: "converting" | "transcribing" | "diarizing" | "reviewing" | "done",
  percent: 0..100,
  message: string }
```

`percent === 0` is a sentinel meaning "keep current percent, just update
the message": used when forwarding library log lines whose granularity
is unknown. The frontend shows an indeterminate animated bar whenever
the message contains `"(Xs elapsed"` (heartbeat is active).

## Auto-flagging heuristics

Segments are flagged automatically (no LLM required) when:

- Average word confidence < 55 %
- ≥ 2 words have alignment score < 0.05 (whisperX gave up aligning)
- The segment is short (< 12 words) and any word has confidence < 0.3
- 3+ words have confidence < 0.3 in a longer segment

Token-level highlighting uses three buckets:

- < 0.7 → yellow tint
- < 0.5 → stronger yellow
- < 0.3 → red with dashed underline

## Test files (`test-audio/`)

- `short-single.wav`: 4.7 s, single voice (Samantha). Use for fast iteration.
- `short-multi.wav`: 12.7 s, two voices alternating. For testing diarization
  once an HF token is set up.
- `troublesome.wav`: 16.8 s, designed to stress whisperX (fast scientific
  jargon + overlapping voices + speech buried in pink noise).

## Settings persistence

User settings live in `localStorage` under `tracet:settings`. Schema in
`src/lib/settings.ts`. The pipeline reads the latest values via
`loadSettings()` on every run, so changes in `SettingsDialog` take effect
immediately without an app restart.

## Coding style

- Avoid comments that just restate the code; comment the *why* when
  non-obvious (the gotchas above are exactly the cases worth commenting).
- Prefer editing existing files over creating new ones.
- Don't mock at module boundaries that aren't currently tested: most of
  this app is integration-tested by hand.

## License

MIT.
