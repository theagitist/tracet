import { Toolbar } from "./components/Toolbar";
import { ProgressIndicator } from "./components/ProgressIndicator";
import { TranscriptViewer } from "./components/TranscriptViewer";
import { SpeakerPanel } from "./components/SpeakerPanel";
import { ExportDialog } from "./components/ExportDialog";
import { SettingsDialog } from "./components/SettingsDialog";
import { SetupScreen } from "./components/SetupScreen";
import { useTranscriptStore } from "./stores/transcriptStore";
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SetupStatus {
  needs_setup: boolean;
}

function App() {
  const processingState = useTranscriptStore((s) => s.processingState);
  const transcript = useTranscriptStore((s) => s.transcript);
  const error = useTranscriptStore((s) => s.error);
  const [showExport, setShowExport] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [setupNeeded, setSetupNeeded] = useState<boolean | null>(null);

  useEffect(() => {
    invoke<SetupStatus>("check_setup_status")
      .then((s) => setSetupNeeded(s.needs_setup))
      .catch(() => setSetupNeeded(false));
  }, []);

  const isProcessing = !["idle", "done", "error"].includes(processingState);
  const hasTranscript = transcript !== null && processingState === "done";

  // Show setup screen on first launch if dependencies are missing
  if (setupNeeded === true) {
    return (
      <div className="flex h-screen flex-col bg-[var(--color-bg)]">
        <SetupScreen onComplete={() => setSetupNeeded(false)} />
      </div>
    );
  }

  // Still checking setup status
  if (setupNeeded === null) {
    return <div className="flex h-screen bg-[var(--color-bg)]" />;
  }

  return (
    <div className="flex h-screen flex-col bg-[var(--color-bg)]">
      <Toolbar
        onExport={() => setShowExport(true)}
        onSettings={() => setShowSettings(true)}
      />

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

      {!isProcessing && !hasTranscript && !error && <WelcomeScreen />}

      {showExport && <ExportDialog onClose={() => setShowExport(false)} />}
      {showSettings && (
        <SettingsDialog onClose={() => setShowSettings(false)} />
      )}
    </div>
  );
}

function WelcomeScreen() {
  return (
    <div className="flex flex-1 items-center justify-center">
      <div className="text-center">
        <div className="mb-4 text-6xl opacity-20">🎙</div>
        <h2 className="mb-2 text-xl font-medium text-[var(--color-text)]">
          Welcome to Tracet
        </h2>
        <p className="text-sm text-[var(--color-text-muted)]">
          Open an audio or video file to start transcribing
        </p>
        <p className="mt-1 text-xs text-[var(--color-text-muted)]">
          Supports MP3, WAV, FLAC, M4A, MP4, MKV, AVI, and more
        </p>
      </div>
    </div>
  );
}

export default App;
