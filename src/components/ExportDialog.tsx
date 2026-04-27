import { useState } from "react";
import { X, FileText, FileType, Sparkles } from "lucide-react";
import { useTranscription } from "../hooks/useTranscription";
import type { ExportOptions } from "../types/transcript";

interface Props {
  onClose: () => void;
}

const FORMATS = [
  {
    id: "Md" as const,
    label: "Markdown",
    ext: ".md",
    icon: FileType,
    description: "Standard prose for humans, docs, blogs",
  },
  {
    id: "Txt" as const,
    label: "Plain Text",
    ext: ".txt",
    icon: FileText,
    description: "Minimal, no formatting",
  },
  {
    id: "AiMd" as const,
    label: "AI-friendly Markdown",
    ext: ".ai.md",
    icon: Sparkles,
    description:
      "Structured metadata + numbered segments: hand off to any LLM",
  },
];

export function ExportDialog({ onClose }: Props) {
  const { exportTranscript } = useTranscription();
  const [options, setOptions] = useState<ExportOptions>({
    format: "Md",
    include_timestamps: true,
    include_speaker_labels: true,
    highlight_low_confidence: true,
    include_annotations: true,
  });

  const handleExport = async () => {
    await exportTranscript(options);
    onClose();
  };

  // The AI-friendly export is opinionated: its structure is fixed and the
  // optional toggles below don't apply.
  const aiMode = options.format === "AiMd";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-[440px] rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] p-6 shadow-xl">
        <div className="mb-5 flex items-center justify-between">
          <h2 className="text-base font-medium text-[var(--color-text)]">
            Export Transcript
          </h2>
          <button
            onClick={onClose}
            className="rounded p-1 text-[var(--color-text-muted)] hover:bg-[var(--color-surface-hover)]"
          >
            <X size={16} />
          </button>
        </div>

        {/* Format selection */}
        <div className="mb-5">
          <label className="mb-2 block text-xs font-medium text-[var(--color-text-muted)]">
            Format
          </label>
          <div className="space-y-2">
            {FORMATS.map((fmt) => {
              const Icon = fmt.icon;
              const selected = options.format === fmt.id;
              return (
                <button
                  key={fmt.id}
                  onClick={() => setOptions({ ...options, format: fmt.id })}
                  className={`flex w-full items-start gap-3 rounded border px-3 py-2.5 text-left transition-colors ${
                    selected
                      ? "border-[var(--color-accent)] bg-[var(--color-accent)]/5"
                      : "border-[var(--color-border)] bg-[var(--color-bg)] hover:border-[var(--color-accent)]/50"
                  }`}
                >
                  <Icon
                    size={16}
                    className={`mt-0.5 shrink-0 ${selected ? "text-[var(--color-accent)]" : "text-[var(--color-text-muted)]"}`}
                  />
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-[var(--color-text)]">
                        {fmt.label}
                      </span>
                      <span className="text-xs text-[var(--color-text-muted)]">
                        {fmt.ext}
                      </span>
                    </div>
                    <div className="mt-0.5 text-xs text-[var(--color-text-muted)]">
                      {fmt.description}
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        {/* Options (only apply to .md and .txt) */}
        <div className={aiMode ? "pointer-events-none mb-5 opacity-40" : "mb-5"}>
          <label className="mb-2 block text-xs font-medium text-[var(--color-text-muted)]">
            Options {aiMode && "(fixed for AI format)"}
          </label>
          <div className="space-y-2.5">
            <Checkbox
              label="Include timestamps"
              checked={options.include_timestamps}
              onChange={(v) =>
                setOptions({ ...options, include_timestamps: v })
              }
            />
            <Checkbox
              label="Include speaker labels"
              checked={options.include_speaker_labels}
              onChange={(v) =>
                setOptions({ ...options, include_speaker_labels: v })
              }
            />
            <Checkbox
              label="Highlight low-confidence text"
              checked={options.highlight_low_confidence}
              onChange={(v) =>
                setOptions({ ...options, highlight_low_confidence: v })
              }
            />
            <Checkbox
              label="Include non-speech annotations"
              checked={options.include_annotations}
              onChange={(v) =>
                setOptions({ ...options, include_annotations: v })
              }
            />
          </div>
        </div>

        {/* Actions */}
        <div className="flex justify-end gap-2">
          <button
            onClick={onClose}
            className="rounded px-4 py-2 text-sm text-[var(--color-text-muted)] hover:text-[var(--color-text)]"
          >
            Cancel
          </button>
          <button
            onClick={handleExport}
            className="rounded bg-[var(--color-accent)] px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-[var(--color-accent-hover)]"
          >
            Export
          </button>
        </div>
      </div>
    </div>
  );
}

function Checkbox({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex cursor-pointer items-center gap-2">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="rounded border-[var(--color-border)] bg-[var(--color-bg)] accent-[var(--color-accent)]"
      />
      <span className="text-sm text-[var(--color-text)]">{label}</span>
    </label>
  );
}
