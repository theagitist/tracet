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

Download the latest [`.dmg` from GitHub Releases](https://github.com/theagitist/tracet/releases/latest),
drag the app to Applications, and launch. On first run a setup screen
offers to install the remaining dependencies (FFmpeg, the Python venv,
whisperX).

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

## Privacy

Tracet is privacy-first by design. Audio files, transcripts, and saved
`.tracet` projects never leave your machine for normal use. There is no
telemetry, no analytics, no crash reporting, and no auto-updater. The
app does not phone home.

There are exactly three places where the app makes network requests:

1. **Model downloads on first run.** whisperX and pyannote download their
   model weights from HuggingFace and the Torch Hub the first time you
   transcribe. These are one-way downloads (no audio or transcript data
   is sent), and once cached at `~/.cache/huggingface/hub/` and
   `~/.cache/torch/hub/checkpoints/`, transcription works fully offline.
2. **HuggingFace token use.** If you set a HuggingFace token to enable
   speaker diarization, that token is sent to `huggingface.co` solely to
   authenticate the pyannote model download. It is not transmitted to
   any other endpoint. The token is stored as plain text in your
   browser's `localStorage` (under the key `tracet:settings`) and is
   readable by anything with access to your user account on this
   machine. Treat it like a low-stakes credential and do not share the
   `tracet:settings` value.
3. **Ollama LLM review (optional, local by default).** When LLM review is
   enabled, transcript text is sent to whatever URL the **Ollama URL**
   setting points at. The default is `http://localhost:11434`, so the
   data stays on this machine. If you change that URL to a non-local
   host (anything other than `localhost`, `127.0.0.1`, `::1`, or
   `0.0.0.0`), every transcript segment Tracet reviews will be sent in
   plaintext to that host. Settings detects this case, shows a red
   warning, and requires you to type `I understand` to apply the change.
   Keep the URL on `localhost` to preserve the privacy guarantee.

Things that explicitly do NOT happen:

- No transcript, audio, or project data is ever sent to a server Tracet
  controls. There is no Tracet backend.
- No analytics SDKs (Sentry, PostHog, Mixpanel, Amplitude, Google
  Analytics, Segment, etc.) are bundled.
- The Tauri auto-updater is not registered. You update by downloading a
  new release manually.
- The webview only loads bundled local assets; no third-party scripts
  are fetched at runtime.

If you want to verify any of this, the only outbound HTTP code paths
are `src-tauri/src/commands/setup.rs` (Ollama health check), and
`src-tauri/src/commands/review.rs` (Ollama generate call); plus the
HuggingFace fetches inside the Python sidecar (`sidecar/diarize.py` via
the whisperX and pyannote libraries).

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
