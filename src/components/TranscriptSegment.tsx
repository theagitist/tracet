import { useCallback, useRef, useState } from "react";
import { useTranscriptStore } from "../stores/transcriptStore";
import type {
  TranscriptSegment as TSegment,
  TokenConfidence,
} from "../types/transcript";
import { AlertTriangle, Pencil, Play, Square } from "lucide-react";
import { useSegmentPlayer } from "../hooks/useSegmentPlayer";

interface Props {
  segment: TSegment;
}

export function TranscriptSegment({ segment }: Props) {
  const updateSegmentText = useTranscriptStore((s) => s.updateSegmentText);
  const transcript = useTranscriptStore((s) => s.transcript);
  const textRef = useRef<HTMLSpanElement>(null);
  const [isEditing, setIsEditing] = useState(false);
  const { isPlaying, toggle } = useSegmentPlayer(segment.id);

  const speaker = transcript?.speakers.find(
    (s) => s.id === segment.speaker_id
  );
  const speakerLabel =
    segment.speaker_name || speaker?.name || segment.speaker_id;
  const speakerColor = speaker?.color || "#6b7280";

  const handleBlur = useCallback(() => {
    setIsEditing(false);
    if (textRef.current) {
      const newText = textRef.current.textContent || "";
      if (newText !== segment.text) {
        updateSegmentText(segment.id, newText);
      }
    }
  }, [segment.id, segment.text, updateSegmentText]);

  // Non-speech segment
  if (segment.segment_type !== "Speech") {
    const label =
      {
        Noise: "[noise]",
        Music: "[music]",
        Silence: "[silence]",
        Unknown: "[unknown]",
      }[segment.segment_type] || "[unknown]";

    return (
      <div className="annotation flex items-center gap-2 py-1">
        <span className="text-xs text-[var(--color-text-muted)]">
          {formatTimestamp(segment.start_ms)}
        </span>
        <span>{label}</span>
      </div>
    );
  }

  return (
    <div
      className={`group flex gap-3 rounded px-3 py-2 transition-colors ${
        isPlaying ? "bg-[var(--color-accent)]/10" : "hover:bg-[var(--color-surface)]"
      } ${segment.is_flagged ? "segment-flagged" : ""}`}
    >
      {/* Play / stop button */}
      <button
        onClick={() => toggle(segment.start_ms, segment.end_ms)}
        className={`mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded transition-colors ${
          isPlaying
            ? "bg-[var(--color-accent)] text-white"
            : "text-[var(--color-text-muted)] opacity-50 hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text)] hover:opacity-100 group-hover:opacity-100"
        }`}
        title={isPlaying ? "Stop" : "Play this segment"}
      >
        {isPlaying ? <Square size={12} fill="currentColor" /> : <Play size={12} fill="currentColor" />}
      </button>

      {/* Timestamp */}
      <span className="mt-1 shrink-0 font-mono text-xs text-[var(--color-text-muted)]">
        {formatTimestamp(segment.start_ms)}
      </span>

      <div className="min-w-0 flex-1">
        {/* Speaker badge */}
        {speakerLabel && (
          <span
            className="mb-1 mr-2 inline-block rounded-full px-2 py-0.5 text-xs font-medium"
            style={{
              backgroundColor: `${speakerColor}20`,
              color: speakerColor,
            }}
          >
            {speakerLabel}
          </span>
        )}

        {/* Editable text with confidence highlighting */}
        <span
          ref={textRef}
          contentEditable
          suppressContentEditableWarning
          onFocus={() => setIsEditing(true)}
          onBlur={handleBlur}
          className={`segment-text text-sm leading-relaxed text-[var(--color-text)] ${
            isEditing ? "is-editing" : ""
          }`}
          dangerouslySetInnerHTML={{
            __html: renderConfidenceTokens(
              segment.text,
              segment.token_confidences
            ),
          }}
        />

        {/* Edit hint (only on hover, when not editing) */}
        {!isEditing && (
          <span
            className="ml-1 inline-flex items-center align-baseline text-[var(--color-text-muted)] opacity-0 transition-opacity group-hover:opacity-50"
            title="Click the text to edit"
          >
            <Pencil size={10} />
          </span>
        )}

        {/* Flag indicator */}
        {segment.is_flagged && segment.flag_reason && (
          <div className="mt-1 flex items-center gap-1 text-xs text-[var(--color-warning)]">
            <AlertTriangle size={12} />
            <span>{segment.flag_reason}</span>
          </div>
        )}
      </div>

      {/* Confidence indicator */}
      <div className="mt-1 shrink-0 opacity-0 transition-opacity group-hover:opacity-100">
        <span
          className={`text-xs tabular-nums ${
            segment.confidence >= 0.8
              ? "text-[var(--color-success)]"
              : segment.confidence >= 0.6
                ? "text-[var(--color-warning)]"
                : "text-[var(--color-danger)]"
          }`}
        >
          {Math.round(segment.confidence * 100)}%
        </span>
      </div>
    </div>
  );
}

function renderConfidenceTokens(
  text: string,
  tokens: TokenConfidence[]
): string {
  if (tokens.length === 0) return escapeHtml(text);

  let result = "";
  let remaining = text;

  for (const tc of tokens) {
    const idx = remaining.indexOf(tc.token);
    if (idx === -1) continue;

    if (idx > 0) {
      result += escapeHtml(remaining.slice(0, idx));
    }

    if (tc.confidence < 0.3) {
      result += `<span class="token-very-low-confidence">${escapeHtml(tc.token)}</span>`;
    } else if (tc.confidence < 0.5) {
      result += `<span class="token-low-confidence">${escapeHtml(tc.token)}</span>`;
    } else if (tc.confidence < 0.7) {
      result += `<span class="token-medium-confidence">${escapeHtml(tc.token)}</span>`;
    } else {
      result += escapeHtml(tc.token);
    }

    remaining = remaining.slice(idx + tc.token.length);
  }

  if (remaining) {
    result += escapeHtml(remaining);
  }

  return result;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function formatTimestamp(ms: number): string {
  const totalSecs = Math.floor(ms / 1000);
  const hours = Math.floor(totalSecs / 3600);
  const minutes = Math.floor((totalSecs % 3600) / 60);
  const seconds = totalSecs % 60;
  if (hours > 0) {
    return `${pad(hours)}:${pad(minutes)}:${pad(seconds)}`;
  }
  return `${pad(minutes)}:${pad(seconds)}`;
}

function pad(n: number): string {
  return n.toString().padStart(2, "0");
}
