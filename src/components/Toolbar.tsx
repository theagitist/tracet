import { FileAudio, Download, Settings, FolderOpen, Save } from "lucide-react";
import { useTranscription } from "../hooks/useTranscription";
import { useTranscriptStore } from "../stores/transcriptStore";

interface ToolbarProps {
  onExport: () => void;
  onSettings: () => void;
  canTranscribe?: boolean;
}

export function Toolbar({
  onExport,
  onSettings,
  canTranscribe = true,
}: ToolbarProps) {
  const { openFile, saveProject, openProject } = useTranscription();
  const transcript = useTranscriptStore((s) => s.transcript);
  const processingState = useTranscriptStore((s) => s.processingState);
  const isProcessing = !["idle", "done", "error"].includes(processingState);

  return (
    <div className="flex h-12 items-center justify-between border-b border-[var(--color-border)] bg-[var(--color-surface)] px-4">
      <div className="flex items-center gap-1">
        <span className="mr-2 text-sm font-semibold tracking-wide text-[var(--color-text)]">
          Tracet
        </span>
        <div className="mr-2 h-4 w-px bg-[var(--color-border)]" />
        <ToolbarButton
          onClick={openFile}
          disabled={isProcessing || !canTranscribe}
          icon={<FileAudio size={16} />}
          label="Open Media"
          title={
            canTranscribe
              ? "Open an audio or video file and run transcription"
              : "Transcription dependencies are missing. Open the setup banner or Settings to install them."
          }
        />
        <ToolbarButton
          onClick={openProject}
          disabled={isProcessing}
          icon={<FolderOpen size={16} />}
          label="Open Project"
          title="Open a previously saved .tracet project"
        />
      </div>

      <div className="flex items-center gap-1">
        {transcript && (
          <>
            <ToolbarButton
              onClick={saveProject}
              icon={<Save size={16} />}
              label="Save Project"
              title="Save audio + transcript as a .tracet bundle"
            />
            <ToolbarButton
              onClick={onExport}
              icon={<Download size={16} />}
              label="Export"
              title="Export transcript as .txt or .md"
            />
          </>
        )}
        <ToolbarButton
          onClick={onSettings}
          icon={<Settings size={16} />}
          label=""
          title="Settings"
        />
      </div>
    </div>
  );
}

function ToolbarButton({
  onClick,
  disabled,
  icon,
  label,
  title,
}: {
  onClick: () => void;
  disabled?: boolean;
  icon: React.ReactNode;
  label: string;
  title?: string;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={title}
      className="flex items-center gap-2 rounded px-3 py-1.5 text-sm text-[var(--color-text-muted)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text)] disabled:opacity-40"
    >
      {icon}
      {label && <span>{label}</span>}
    </button>
  );
}
