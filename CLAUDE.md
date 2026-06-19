# CLAUDE.md

Project instructions for Claude Code working on Tracet.

## Project summary

Tracet is a local desktop transcription app: **Tauri v2** shell with a
**React/TypeScript** frontend and a **Rust** backend, plus a **Python sidecar**
running **whisperX** for transcription, alignment, and pyannote speaker
diarization. Optional LLM accuracy review is provided via a local Ollama
instance over HTTP. Quality is the top priority for technical decisions
unless explicitly traded against performance.

Target platform: macOS on Apple Silicon (primary). macOS on Intel (x86_64)
is supported on a best-effort basis: it works, but with no MPS acceleration
(CPU-only for transcription, alignment, and diarization), so it is slower.
Releases ship a `.dmg` per architecture. Non-macOS platforms are not
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
cargo tauri build             # quick local release (.app + .dmg), unsigned
./build.sh                    # signed + notarized aarch64 .dmg (default)
./build.sh x86_64             # signed + notarized Intel .dmg
./build.sh universal          # signed + notarized universal .dmg
```

`build.sh` reads code-signing and notarization credentials from
`signing.env` (gitignored). It signs in a non-synced temp dir because a
file-sync daemon watching this tree re-stamps `com.apple.FinderInfo` on the
`.app` within seconds, which `codesign` rejects. See the release section.

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
  - `export.rs`: `.txt` / `.md` / `.ai.md` formatting (the AI-friendly
    variant has fixed structure: YAML frontmatter, numbered segments,
    inline strikethrough for very-low-confidence words)
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
   with int8 quantisation. Alignment and diarization can use MPS on Apple
   Silicon. On Intel there is no MPS, so `diarize.py` falls back to CPU for
   all stages (it already defaults `torch_device` to `cpu` and only upgrades
   to `mps` when available, with a CPU retry around alignment).
4. **whisperX on first run downloads several GB.** Models cached at
   `~/.cache/huggingface/hub/` and `~/.cache/torch/hub/checkpoints/`.
   `large-v3-turbo` is the default and offers the best speed/quality tradeoff.
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
the message". It is used when forwarding library log lines whose granularity
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
- Don't mock at module boundaries that aren't currently tested. Most of
  this app is integration-tested by hand.

## Conventions

- **Commit messages** follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).
  Format: `type(scope): subject` in lowercase, imperative mood, no trailing
  period. Body explains the *why*. Common types: `feat`, `fix`, `docs`,
  `refactor`, `chore`, `perf`. Use `!` after the type for breaking changes
  (e.g. `feat(export)!: drop legacy txt format`).
- **Never use em dashes** in any authored content (commit messages, code
  comments, docs, UI copy, release notes, GitHub repo description, etc.).
  Reach for proper punctuation instead: colon to introduce an explanation
  or list, period to break sentences, comma for a brief aside, semicolon
  to join two related independent clauses, or parentheses for asides.
  Never substitute with a single hyphen.
- **No Claude attribution** in commits, PR descriptions, or release notes.
  Drop the standard `Co-Authored-By: Claude ...` trailer entirely.

## Release process

Release artefacts live in GitHub Releases (not GitHub Packages, which is
for code registries like npm). The flow:

1. Bump the version in three places: `package.json`, `src-tauri/Cargo.toml`,
   `src-tauri/tauri.conf.json`. They must agree.
2. Update README/CLAUDE.md if behaviour changed.
3. Commit using Conventional Commits.
4. Build the signed + notarized artefacts with `./build.sh` (needs
   `signing.env`; see "Code signing and notarization" below). Build both
   architectures:
   - `./build.sh aarch64` produces
     `src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/Tracet_<version>_aarch64.dmg`
   - `./build.sh x86_64` produces
     `src-tauri/target/x86_64-apple-darwin/release/bundle/dmg/Tracet_<version>_x64.dmg`
   Each is signed, notarized (Apple status Accepted), and stapled. Verify
   with `xcrun stapler validate <dmg>` and
   `spctl -a -t open --context context:primary-signature <dmg>`.
5. Tag with an annotated tag: `git tag -a vX.Y.Z -m "Tracet X.Y.Z (...)"`.
6. Push: `git push origin main && git push origin vX.Y.Z`.
7. Create the release with BOTH `.dmg` files attached:
   `gh release create vX.Y.Z <aarch64-dmg> <x64-dmg> --title "Tracet X.Y.Z" --notes-file <notes>`.

Force-pushing a tag (e.g. after a history rewrite) preserves the release
on GitHub. The release is bound to the tag *name*, not the SHA. Deleting
a remote tag, however, deletes the associated release.

## Code signing and notarization

Releases are distributed outside the Mac App Store as signed + notarized
`.dmg` files (Developer ID). The App Store path (sandboxed) is a separate,
deferred effort.

- **Credentials** live in `signing.env` (gitignored, never committed):
  `APPLE_SIGNING_IDENTITY` (the "Developer ID Application: ..." cert common
  name) plus the App Store Connect API key trio `APPLE_API_ISSUER`,
  `APPLE_API_KEY` (key id), and `APPLE_API_KEY_PATH` (path to the `.p8`,
  stored under `~/.appstoreconnect/private_keys/`, not in the repo).
- **`build.sh` does the work**, not bare `cargo tauri build`. It builds an
  unsigned `.app`, then signs, packages the `.dmg`, notarizes, and staples.
- **Why sign in a temp dir:** a file-sync daemon watches this project tree
  and re-adds `com.apple.FinderInfo` (the custom-icon bit) to the `.app`
  within a few seconds of any `xattr` strip. `codesign` rejects bundles
  carrying FinderInfo ("resource fork ... detritus not allowed"). So
  `build.sh` copies the bundle into a `mktemp` dir under `/var/folders`
  (not synced), where nothing re-stamps it, and does all signing there.
- **Entitlements** are in `src-tauri/entitlements.plist`: hardened-runtime
  exceptions (`allow-jit`, `allow-unsigned-executable-memory`,
  `disable-library-validation`, `allow-dyld-environment-variables`) required
  by the Python/torch stack. These are NOT App Sandbox keys.

## License

MIT.
