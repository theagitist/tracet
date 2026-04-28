import { useState, useEffect } from "react";
import {
  Check,
  X,
  Loader2,
  Download,
  ExternalLink,
  AlertTriangle,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PipelineProgress } from "../types/transcript";

interface SetupStatus {
  ffmpeg_installed: boolean;
  python_available: boolean;
  venv_ready: boolean;
  ollama_installed: boolean;
  ollama_running: boolean;
  needs_setup: boolean;
}

interface Props {
  onComplete: () => void;
  onClose?: () => void;
}

export function SetupScreen({ onComplete, onClose }: Props) {
  const [status, setStatus] = useState<SetupStatus | null>(null);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState(0);
  const [progressMsg, setProgressMsg] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<SetupStatus>("check_setup_status").then(setStatus);
  }, []);

  const handleRunSetup = async () => {
    setRunning(true);
    setError(null);

    const unlisten = await listen<PipelineProgress>(
      "setup:progress",
      (event) => {
        setProgress(event.payload.percent);
        setProgressMsg(event.payload.message);
      }
    );

    try {
      await invoke("run_setup");
      const newStatus = await invoke<SetupStatus>("check_setup_status");
      setStatus(newStatus);
      if (!newStatus.needs_setup) {
        onComplete();
      }
    } catch (err) {
      setError(String(err));
    } finally {
      unlisten();
      setRunning(false);
    }
  };

  const handleSkip = () => {
    onComplete();
  };

  if (!status) {
    return (
      <div className="w-[480px] rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] p-8">
        <Loader2
          size={24}
          className="mx-auto animate-spin text-[var(--color-accent)]"
        />
      </div>
    );
  }

  if (!status.needs_setup) {
    onComplete();
    return null;
  }

  return (
    <div className="w-[480px] rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] p-8">
      <div className="mb-1 flex items-start justify-between">
        <h1 className="text-xl font-semibold text-[var(--color-text)]">
          Set up dependencies
        </h1>
        {onClose && (
          <button
            onClick={onClose}
            title="Close"
            className="rounded p-1 text-[var(--color-text-muted)] hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text)]"
          >
            <X size={16} />
          </button>
        )}
      </div>
      <p className="mb-6 text-sm text-[var(--color-text-muted)]">
        A few dependencies need to be set up before you can transcribe new
        recordings. Existing projects can be opened without these.
      </p>

        {/* Checklist */}
        <div className="mb-6 space-y-3">
          <CheckItem
            label="FFmpeg"
            detail="Audio/video conversion"
            installed={status.ffmpeg_installed}
          />
          <CheckItem
            label="Python 3"
            detail="Required for whisperX"
            installed={status.python_available}
          />
          <CheckItem
            label="whisperX + pyannote"
            detail="Transcription, alignment, speaker diarization"
            installed={status.venv_ready}
          />
          <CheckItem
            label="Ollama"
            detail="LLM accuracy review (optional)"
            installed={status.ollama_installed}
            optional
          />
        </div>

        {/* Progress bar when running */}
        {running && (
          <div className="mb-5">
            <div className="mb-1 flex justify-between text-xs">
              <span className="text-[var(--color-text-muted)]">
                {progressMsg}
              </span>
              <span className="text-[var(--color-text-muted)]">
                {progress}%
              </span>
            </div>
            <div className="h-2 overflow-hidden rounded-full bg-[var(--color-bg)]">
              <div
                className="h-full rounded-full bg-[var(--color-accent)] transition-all duration-500"
                style={{ width: `${progress}%` }}
              />
            </div>
          </div>
        )}

        {error && (
          <div className="mb-4 flex items-start gap-2 rounded border border-red-500/20 bg-red-500/5 px-3 py-2">
            <AlertTriangle
              size={14}
              className="mt-0.5 shrink-0 text-[var(--color-danger)]"
            />
            <span className="text-xs text-red-400">{error}</span>
          </div>
        )}

        {/* Actions */}
        <div className="flex items-center justify-between">
          <button
            onClick={handleSkip}
            className="text-xs text-[var(--color-text-muted)] hover:text-[var(--color-text)]"
          >
            Skip for now
          </button>
          <button
            onClick={handleRunSetup}
            disabled={running}
            className="flex items-center gap-2 rounded bg-[var(--color-accent)] px-5 py-2 text-sm font-medium text-white transition-colors hover:bg-[var(--color-accent-hover)] disabled:opacity-50"
          >
            {running ? (
              <>
                <Loader2 size={14} className="animate-spin" />
                Installing...
              </>
            ) : (
              <>
                <Download size={14} />
                Install Dependencies
              </>
            )}
          </button>
        </div>

        {!status.ollama_installed && (
          <div className="mt-4 rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2">
            <p className="text-xs text-[var(--color-text-muted)]">
              For LLM-powered accuracy review, install Ollama separately:{" "}
              <a
                href="https://ollama.ai"
                target="_blank"
                rel="noopener"
                className="inline-flex items-center gap-0.5 text-[var(--color-accent)] hover:underline"
              >
                ollama.ai <ExternalLink size={10} />
              </a>{" "}
              then run{" "}
              <code className="rounded bg-[var(--color-surface)] px-1">
                ollama pull llama3.1:8b
              </code>
            </p>
          </div>
        )}
    </div>
  );
}

function CheckItem({
  label,
  detail,
  installed,
  optional = false,
}: {
  label: string;
  detail: string;
  installed: boolean;
  optional?: boolean;
}) {
  return (
    <div className="flex items-center gap-3">
      <div
        className={`flex h-5 w-5 shrink-0 items-center justify-center rounded-full ${
          installed
            ? "bg-[var(--color-success)]"
            : optional
              ? "border border-dashed border-[var(--color-border)]"
              : "border border-[var(--color-danger)] bg-[var(--color-danger)]/10"
        }`}
      >
        {installed ? (
          <Check size={12} className="text-white" />
        ) : optional ? (
          <span className="text-[8px] text-[var(--color-text-muted)]">opt</span>
        ) : (
          <X size={10} className="text-[var(--color-danger)]" />
        )}
      </div>
      <div>
        <span className="text-sm text-[var(--color-text)]">{label}</span>
        {optional && (
          <span className="ml-1 text-[10px] text-[var(--color-text-muted)]">
            (optional)
          </span>
        )}
        <div className="text-xs text-[var(--color-text-muted)]">{detail}</div>
      </div>
    </div>
  );
}
