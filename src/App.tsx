import { Toolbar } from "./components/Toolbar";
import { ProgressIndicator } from "./components/ProgressIndicator";
import { TranscriptViewer } from "./components/TranscriptViewer";
import { SpeakerPanel } from "./components/SpeakerPanel";
import { ExportDialog } from "./components/ExportDialog";
import { SettingsDialog } from "./components/SettingsDialog";
import { SetupScreen } from "./components/SetupScreen";
import { useTranscriptStore } from "./stores/transcriptStore";
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ask } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, Wrench, X } from "lucide-react";

interface SetupStatus {
  ffmpeg_installed: boolean;
  python_available: boolean;
  venv_ready: boolean;
  ollama_installed: boolean;
  ollama_running: boolean;
  needs_setup: boolean;
}

function App() {
  const processingState = useTranscriptStore((s) => s.processingState);
  const transcript = useTranscriptStore((s) => s.transcript);
  const error = useTranscriptStore((s) => s.error);
  const [showExport, setShowExport] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showSetup, setShowSetup] = useState(false);
  const [bannerDismissed, setBannerDismissed] = useState(false);
  const [setupStatus, setSetupStatus] = useState<SetupStatus | null>(null);

  const refreshSetup = useCallback(() => {
    invoke<SetupStatus>("check_setup_status")
      .then(setSetupStatus)
      .catch(() => setSetupStatus(null));
  }, []);

  useEffect(() => {
    refreshSetup();
  }, [refreshSetup]);

  // Intercept window close to warn before discarding an unsaved transcription.
  // Read store state at event time (not via the React selector hooks) so the
  // handler always sees the latest values without re-subscribing on every
  // mutation.
  useEffect(() => {
    const appWindow = getCurrentWindow();
    const unlistenP = appWindow.onCloseRequested(async (event) => {
      const state = useTranscriptStore.getState();
      if (!state.transcript || !state.isUnsavedNew) return;

      event.preventDefault();
      const proceed = await ask(
        "You have a new transcription that has not been saved as a .tracet project. Closing now will discard it.\n\nClose without saving?",
        {
          title: "Unsaved transcription",
          kind: "warning",
          okLabel: "Close anyway",
          cancelLabel: "Cancel",
        },
      );
      if (proceed) {
        await appWindow.destroy();
      }
    });
    return () => {
      unlistenP.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);

  const canTranscribe =
    !!setupStatus &&
    setupStatus.ffmpeg_installed &&
    setupStatus.python_available &&
    setupStatus.venv_ready;

  const isProcessing = !["idle", "done", "error"].includes(processingState);
  const hasTranscript = transcript !== null && processingState === "done";

  return (
    <div className="flex h-screen flex-col bg-[var(--color-bg)]">
      <Toolbar
        onExport={() => setShowExport(true)}
        onSettings={() => setShowSettings(true)}
        canTranscribe={canTranscribe}
      />

      {setupStatus && !canTranscribe && !bannerDismissed && (
        <SetupBanner
          status={setupStatus}
          onSetUp={() => setShowSetup(true)}
          onDismiss={() => setBannerDismissed(true)}
        />
      )}

      {error && (
        <div className="mx-4 mt-2 rounded border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-400">
          {error}
        </div>
      )}

      {isProcessing && <ProgressIndicator />}

      {hasTranscript && (
        <div className="flex min-h-0 flex-1">
          <div className="flex-1 overflow-y-auto">
            <TranscriptViewer />
          </div>
          {transcript.speakers.length > 0 && (
            <div className="w-72 shrink-0 border-l border-[var(--color-border)]">
              <SpeakerPanel />
            </div>
          )}
        </div>
      )}

      {!isProcessing && !hasTranscript && !error && (
        <WelcomeScreen canTranscribe={canTranscribe} />
      )}

      {showExport && <ExportDialog onClose={() => setShowExport(false)} />}
      {showSettings && (
        <SettingsDialog onClose={() => setShowSettings(false)} />
      )}
      {showSetup && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <SetupScreen
            onComplete={() => {
              setShowSetup(false);
              refreshSetup();
            }}
            onClose={() => setShowSetup(false)}
          />
        </div>
      )}
    </div>
  );
}

function SetupBanner({
  status,
  onSetUp,
  onDismiss,
}: {
  status: SetupStatus;
  onSetUp: () => void;
  onDismiss: () => void;
}) {
  const missing: string[] = [];
  if (!status.ffmpeg_installed) missing.push("FFmpeg");
  if (!status.python_available) missing.push("Python");
  if (!status.venv_ready) missing.push("whisperX");

  return (
    <div className="flex items-center gap-3 border-b border-[var(--color-border)] bg-amber-500/10 px-4 py-2 text-xs text-amber-300">
      <AlertTriangle size={14} className="shrink-0" />
      <span className="flex-1">
        Transcription is disabled because {missing.join(", ")}{" "}
        {missing.length > 1 ? "are" : "is"} missing. You can still open saved{" "}
        <code className="rounded bg-black/20 px-1">.tracet</code> projects.
      </span>
      <button
        onClick={onSetUp}
        className="flex items-center gap-1 rounded bg-amber-500/20 px-2 py-1 text-amber-200 hover:bg-amber-500/30"
      >
        <Wrench size={12} /> Set up
      </button>
      <button
        onClick={onDismiss}
        title="Dismiss"
        className="text-[var(--color-text-muted)] hover:text-[var(--color-text)]"
      >
        <X size={14} />
      </button>
    </div>
  );
}

function WelcomeScreen({ canTranscribe }: { canTranscribe: boolean }) {
  return (
    <div className="flex flex-1 items-center justify-center">
      <div className="max-w-md px-6 text-center">
        <div className="mb-4 text-6xl opacity-20">🎙</div>
        <h2 className="mb-2 text-xl font-medium text-[var(--color-text)]">
          Welcome to Tracet
        </h2>
        {canTranscribe ? (
          <>
            <p className="text-sm text-[var(--color-text-muted)]">
              Open an audio or video file to start transcribing
            </p>
            <p className="mt-1 text-xs text-[var(--color-text-muted)]">
              Supports MP3, WAV, FLAC, M4A, MP4, MKV, AVI, and more
            </p>
          </>
        ) : (
          <p className="text-sm text-[var(--color-text-muted)]">
            Open a saved{" "}
            <code className="rounded bg-black/20 px-1">.tracet</code> project,
            or install transcription dependencies to start transcribing new
            recordings.
          </p>
        )}
      </div>
    </div>
  );
}

export default App;
