import { useState } from "react";
import { X } from "lucide-react";
import { useTranscription } from "../hooks/useTranscription";
import type { ExportOptions } from "../types/transcript";

interface Props {
  onClose: () => void;
}

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

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-96 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] p-6 shadow-xl">
        <div className="mb-4 flex items-center justify-between">
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
        <div className="mb-4">
          <label className="mb-2 block text-xs font-medium text-[var(--color-text-muted)]">
            Format
          </label>
          <div className="flex gap-2">
            {(["Md", "Txt"] as const).map((fmt) => (
              <button
                key={fmt}
                onClick={() => setOptions({ ...options, format: fmt })}
                className={`rounded px-4 py-2 text-sm transition-colors ${
                  options.format === fmt
                    ? "bg-[var(--color-accent)] text-white"
                    : "bg-[var(--color-bg)] text-[var(--color-text-muted)] hover:text-[var(--color-text)]"
                }`}
              >
                {fmt === "Md" ? "Markdown (.md)" : "Plain Text (.txt)"}
              </button>
            ))}
          </div>
        </div>

        {/* Options */}
        <div className="mb-6 space-y-3">
          <Checkbox
            label="Include timestamps"
            checked={options.include_timestamps}
            onChange={(v) => setOptions({ ...options, include_timestamps: v })}
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
