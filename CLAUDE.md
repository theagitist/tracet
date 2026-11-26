# CLAUDE.md

Project instructions for Claude Code working on Tracet.

## Project summary

Tracet is a local desktop transcription app: **Tauri v2** shell with a
**React/TypeScript** frontend and a **Rust** backend, plus a **Python sidecar**
running **whisperX** for transcription, alignment, and pyannote speaker
diarization, and **PyAV** for media conversion. Release builds ship their own
Python runtime and the diarization model inside the `.app`, so users install
nothing else. Optional LLM accuracy review is provided via a local Ollama
instance over HTTP. Quality is the top priority for technical decisions
unless explicitly traded against performance.

Target platform: macOS on Apple Silicon only (decided 2026-11-25 for
0.2.0). Intel was dropped because torch 2.8, which whisperX 3.8 needs, has no
Intel macOS wheels. Releases ship one aarch64 `.dmg`. Non-macOS platforms are
not supported (a Windows plan exists in the Polivoxia vault).

## Stack at a glance

| Layer       | Tech                                          |
| ----------- | --------------------------------------------- |
| Shell       | Tauri v2 (Rust + WebView)                     |
| Frontend    | React 19 + TypeScript + Tailwind CSS 4 + Vite |
| State       | Zustand                                       |
| Transcribe  | whisperX (Python sidecar)                     |
| Diarize     | pyannote-audio (inside whisperX)              |
| LLM review  | Ollama HTTP API                               |
| Conversion  | PyAV (FFmpeg's libraries) in `convert.py`     |
| Runtime     | python-build-standalone 3.12, bundled         |
| Project I/O | `zip` crate, `.tracet` bundle = zip archive   |

## Run / build

```bash
./dev.sh                      # dev mode with hot reload (uses sidecar/.venv)
scripts/build-runtime.sh      # bundled Python runtime + diarization model (cached)
./build.sh                    # signed + notarized aarch64 .dmg
```

`build.sh` runs `scripts/build-runtime.sh`, copies the runtime into the
`.app`, then signs and notarizes. Without `signing.env` it leaves an unsigned
`.app` that still contains the runtime. A bare `cargo tauri build` produces
an app WITHOUT the runtime, which cannot transcribe.

`build.sh` reads code-signing and notarization credentials from
`signing.env` (gitignored). It signs in a non-synced temp dir because a
file-sync daemon watching this tree re-stamps `com.apple.FinderInfo` on the
`.app` within seconds, which `codesign` rejects. See the release section.

The first runtime build needs a HuggingFace read token to fetch the gated
diarization model: `HF_TOKEN=...` or `HF_TOKEN_FILE=<path>` (signing.env may
set either). After that the model is cached in
`~/Library/Caches/tracet-build/models/` and no token is needed.

`./dev.sh` exists because some development setups have a broken nvm
lazy-load shim that breaks bare `npm`/`node`. It locates a usable node
binary by scanning `~/.nvm/versions/node` directly. Don't assume `npm`
or `node` are on PATH. Use `./dev.sh`, or the binaries under
`~/.nvm/versions/node/<version>/bin/` (`node`, `npm`, `npx`).

## Backend layout (`src-tauri/src/`)

- `commands/`: one file per Tauri command:
  - `audio.rs`: `read_audio_file` (custom replacement for fs plugin reads,
    used for blob-URL audio playback; bypasses fs plugin scope which is
    flaky on macOS temp paths)
  - `convert.rs`: runs `sidecar/convert.py` to make a 16 kHz mono WAV;
    reads the duration from the WAV header
  - `transcribe.rs`: drives the whisperX sidecar, parses results, applies
    auto-flagging heuristics
  - `diarize.rs`: currently a stub (whisperX handles diarization inline)
  - `review.rs`: Ollama LLM coherence review (graceful fallback when
    Ollama isn't reachable)
  - `export.rs`: `.txt` / `.md` / `.ai.md` formatting (the AI-friendly
    variant has fixed structure: YAML frontmatter, numbered segments,
    inline strikethrough for very-low-confidence words)
  - `project.rs`: `save_project` / `load_project` for `.tracet` bundles
  - `setup.rs`: setup status and the dev-only venv installer (release builds
    report ready because the runtime is bundled), Ollama checks
  - `hardware.rs`: chip / RAM detection, model profile filtering
- `pipeline/orchestrator.rs`: `run_pipeline` top-level command,
  composes convert + transcribe + review
- `sidecar/python.rs`: the one place that resolves sidecar paths. Release:
  scripts from `Resources/sidecar`, interpreter
  `Resources/python/bin/python3.12`, model `Resources/models/`. Debug: the
  repo's `sidecar/` and `sidecar/.venv`. Never use `CARGO_MANIFEST_DIR` in a
  release path (it is the build machine's path; 0.1.x broke this way).
  `sidecar_env()` sets the child's environment (no user `PYTHONPATH`, no
  bytecode writes, `TRACET_DIARIZATION_MODEL`).
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

- `diarize.py`: transcription entry point. JSON-over-stdin/stdout protocol.
  Emits `TRACET_PROGRESS:` lines on stderr that Rust forwards to the UI.
  Heartbeat thread emits "(Ns elapsed, still working...)" every 3 s during
  long phases so the UI never appears frozen. Downloads the whisper model up
  front with byte progress (`ensure_whisper_model`).
- `convert.py` + `audio_io.py`: PyAV decoding of every accepted format to
  16 kHz mono WAV, and the WAV loader `diarize.py` uses. Do not use
  `whisperx.load_audio`: it shells out to the `ffmpeg` command, which
  doesn't exist in release builds.
- `requirements.lock`: exact versions bundled into releases (regenerate from
  a tested venv with `pip freeze --exclude-editable`). `requirements.txt`
  is the loose dev list.
- Python must be **3.10 to 3.13** (whisperX 3.8 requires it; 3.14 breaks
  the stack). Releases bundle 3.12.
- The result line is prefixed with `###TRACET_RESULT###` because some
  libraries write log lines directly to stdout, polluting the JSON channel.

## Gotchas

1. **nvm lazy-load is broken on the dev machine.** Always use full binary
   paths for node/npm/npx, or use `./dev.sh`.
2. **Python 3.14 is too new, 3.9 too old.** The dev venv is created by
   `find_python_interpreter()` (prefers 3.12; a bare `python3` here is
   Homebrew's 3.15). Releases never touch the system Python.
3. **faster-whisper does not support MPS.** Transcription runs on CPU
   with int8 quantisation. Alignment and diarization use MPS.
4. **The whisper model downloads on first use** (about 1.6 GB for the
   default `large-v3-turbo`) into `~/.cache/huggingface/hub/`; alignment
   weights go to `~/.cache/torch/hub/checkpoints/`. `diarize.py` sets
   `HF_HUB_DISABLE_XET=1` because Xet reports progress in bursts and the
   download looked frozen.
5. **The fs plugin's path scope is unreliable for `/var/folders/...`** on
   macOS. Use the custom `read_audio_file` Tauri command for any reads
   from temp dirs, not `@tauri-apps/plugin-fs`.
6. **stdout is contended.** whisperX/pyannote sometimes write logs to
   stdout. The result JSON line is prefixed with `###TRACET_RESULT###`
   and Rust skips other lines.
7. **Diarization ships with the app.** `pyannote/speaker-diarization-community-1`
   (CC BY 4.0, 32 MB, pinned revision) is bundled by `build-runtime.sh` and
   loaded from a local folder with no token. It must stay credited in the
   README and the Settings dialog. whisperX 3.8 moved `DiarizationPipeline`
   to `whisperx.diarize` and renamed `use_auth_token` to `token`; diarization
   errors are caught and only logged, so a broken API shows up as "no
   speakers", not as an error.
8. **The signed bundle is read-only.** The sidecar runs with
   `PYTHONDONTWRITEBYTECODE=1` (bytecode is precompiled at build time);
   writing into the bundle would also invalidate the signature.
9. **pyannote imports torchcodec**, which warns that FFmpeg is missing.
   Tracet never uses it; `diarize.py` filters the warning so it doesn't
   reach the UI.

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
- `short-multi.wav`: 12.7 s, two voices alternating. For testing diarization.
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
- Don't use em dashes in authored content (commit messages, code
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
4. Build the signed + notarized artefact with `./build.sh` (needs
   `signing.env`; see "Code signing and notarization" below). It produces
   `src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/Tracet_<version>_aarch64.dmg`
   (about 650 MB), signed, notarized (Apple status Accepted), and stapled.
   Verify with `xcrun stapler validate <dmg>` and
   `spctl -a -t open --context context:primary-signature <dmg>`.
   If notarization fails, the signed dmg is kept as
   `..._aarch64.unnotarized.dmg`; resubmit it with
   `scripts/notarize.py <dmg> && xcrun stapler staple <dmg>` (with the
   `signing.env` variables exported) instead of rebuilding.
   `SKIP_NOTARIZE=1 ./build.sh` makes a signed test build without uploading.
5. Tag with an annotated tag: `git tag -a vX.Y.Z -m "Tracet X.Y.Z (...)"`.
6. Push: `git push origin main && git push origin vX.Y.Z`.
7. Create the release with the `.dmg` attached:
   `gh release create vX.Y.Z <aarch64-dmg> --title "Tracet X.Y.Z" --notes-file <notes>`.

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
  unsigned `.app`, adds the bundled runtime, then signs, packages the
  `.dmg`, notarizes, and staples.
- **Signing is inside out**, not `--deep`: `--deep` does not treat the ~440
  `.so`/`.dylib` files under `Resources/python` as nested code, and the
  notary rejects any Mach-O without a hardened-runtime signature and secure
  timestamp. Each is signed individually, then `python3.12` and the app with
  the entitlements, then every nested file is verified.
- **Notarization goes through `scripts/notarize.py`**, not `notarytool
  submit`: it uses Apple's Notary REST API with a parallel `aws s3 cp`
  upload (needs the `aws` CLI and `openssl`). notarytool's single-connection
  upload stalled on the dev machine's home Wi-Fi (80 KB/s on a 17.8 Mbps
  link). That Wi-Fi's whole uplink to AWS is only ~200 KB/s; the iPhone
  hotspot does ~7.5 MB/s, so release builds are best notarized from it.
  Bundled-runtime builds were `Accepted` this way on 2026-11-25.
- **Why sign in a temp dir:** a file-sync daemon watches this project tree
  and re-adds `com.apple.FinderInfo` (the custom-icon bit) to the `.app`
  within a few seconds of any `xattr` strip. `codesign` rejects bundles
  carrying FinderInfo ("resource fork ... detritus not allowed"). So
  `build.sh` copies the bundle into a `mktemp` dir under `/var/folders`
  (not synced), where nothing re-stamps it, and does all signing there.
- **Entitlements** are in `src-tauri/entitlements.plist`: hardened-runtime
  exceptions (`allow-jit`, `allow-unsigned-executable-memory`,
  `disable-library-validation`, `allow-dyld-environment-variables`) required
  by the Python/torch stack. They are on the main binary and on the bundled
  `python3.12`. These are NOT App Sandbox keys (see the Mac App Store plan
  in the Polivoxia vault for the sandboxed build).

## License

MIT.
