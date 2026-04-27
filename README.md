# Tracet

A local desktop app for transcribing audio and video, with speaker
identification, word-level confidence scoring, and an editable transcript.
Everything runs on your machine: no cloud APIs.

## Features

- **High-quality transcription** with whisperX (faster-whisper + phoneme
  alignment + integrated speaker diarization)
- **Speaker identification** via pyannote: give each detected voice a custom
  name once and it propagates through the transcript and exports
- **Word-level confidence highlighting**: three-tier visual scale (yellow →
  red dashed underline) makes likely errors easy to spot
- **Per-segment audio playback**: click ▶ on any line to hear the original
  audio for that exact segment
- **Inline editing**: click any text to fix it, edits persist and appear in
  exports
- **Hardware-aware model selection**: settings detect your chip and RAM, then
  filter Whisper and Ollama model options to what your machine can actually run
- **Optional LLM accuracy review** via Ollama (any local model)
- **`.tracet` project bundles**: save a zip containing the converted audio
  + transcript so you can transcribe on a powerful machine and edit elsewhere
- **Three export formats**:
  - **Markdown** (`.md`): prose for humans
  - **Plain text** (`.txt`): minimal
  - **AI-friendly Markdown** (`.ai.md`): YAML frontmatter with metadata,
    numbered segments, structured-but-readable; designed to hand off to any
    LLM without committing to a specific downstream task

## Architecture

```
┌───────────────────────────────────────────────────┐
│  React + TypeScript + Tailwind CSS frontend       │
│  (Vite, served by Tauri webview)                  │
└──────────────┬────────────────────────────────────┘
               │ Tauri IPC
┌──────────────▼────────────────────────────────────┐
│  Rust backend (Tauri v2)                          │
│  - Pipeline orchestrator                          │
│  - FFmpeg child process for media conversion      │
│  - Python sidecar manager                         │
│  - Ollama HTTP client (LLM review)                │
│  - .tracet zip pack/unpack                        │
└──────────────┬────────────────────────────────────┘
               │ stdin/stdout JSON
┌──────────────▼────────────────────────────────────┐
│  Python sidecar (sidecar/.venv)                   │
│  - whisperX (faster-whisper backend)              │
│  - wav2vec2 phoneme alignment                     │
│  - pyannote speaker diarization                   │
└───────────────────────────────────────────────────┘
```

## Requirements

- macOS on Apple Silicon (M1/M2/M3 or later)
- Python 3.12 (3.14 is too new for the ML stack at the moment)
- FFmpeg (auto-installed on first run if Homebrew is available)
- Optional: Ollama (https://ollama.ai) for LLM-based accuracy review
- Optional: HuggingFace account and access token for speaker diarization.
  You will also need to accept the `pyannote/speaker-diarization-3.1`
  license. Tokens are issued at https://huggingface.co/settings/tokens.

## Installation

### From a release DMG

Download the latest `Tracet_<version>_aarch64.dmg`, drag the app to
Applications, and launch. On first run a setup screen offers to install the
remaining dependencies (FFmpeg, the Python venv, whisperX).

### From source

```bash
git clone git@github.com:theagitist/tracet.git
cd tracet
npm install
cargo tauri build
```

The packaged app and `.dmg` end up in
`src-tauri/target/release/bundle/`.

## Development

```bash
./dev.sh
```

`dev.sh` finds the latest installed Node (working around the broken nvm
lazy-load shim some setups have), installs npm deps if missing, then runs
`cargo tauri dev` with hot reload.

## Settings

A HuggingFace token is required for speaker diarization. Without it,
transcription still works but every utterance is attributed to a single
speaker. Set the token via Settings → HuggingFace Token; it persists in
localStorage.

For LLM-based review, install Ollama and pull a model:

```bash
brew install ollama
ollama pull llama3.1:8b
```

Then enable LLM review in Settings.

## Project layout

```
.
├── dev.sh                       # Dev launcher
├── index.html                   # Vite entry HTML
├── package.json                 # Frontend deps + scripts
├── src/                         # React + TypeScript frontend
│   ├── App.tsx
│   ├── components/              # Toolbar, TranscriptViewer, etc.
│   ├── hooks/                   # useTranscription, useSegmentPlayer, ...
│   ├── stores/                  # Zustand transcript store
│   ├── lib/                     # Settings persistence
│   └── types/                   # TypeScript mirrors of Rust structs
├── src-tauri/                   # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/            # Tauri ACL
│   └── src/
│       ├── lib.rs               # Command registration
│       ├── commands/            # Tauri commands (one file per concern)
│       ├── pipeline/            # End-to-end orchestrator
│       ├── models/              # Transcript / Speaker / config structs
│       └── sidecar/             # Python venv + FFmpeg helpers
├── sidecar/                     # Python sidecar
│   ├── diarize.py               # whisperX entry point
│   └── requirements.txt
└── test-audio/                  # Generated test files for QA
    ├── short-single.wav         # 4.7s, single voice
    ├── short-multi.wav          # 12.7s, two voices alternating
    └── troublesome.wav          # 16.8s, designed to stress whisperX
```

## Acknowledgments

Tracet wires together excellent open-source work:

- [whisperX](https://github.com/m-bain/whisperX): transcription + alignment
- [pyannote-audio](https://github.com/pyannote/pyannote-audio): speaker
  diarization
- [faster-whisper](https://github.com/SYSTRAN/faster-whisper): CTranslate2
  Whisper backend
- [Ollama](https://ollama.ai): local LLM runtime
- [Tauri](https://tauri.app): desktop shell
- [FFmpeg](https://ffmpeg.org)

## License

MIT. See [LICENSE](LICENSE).
