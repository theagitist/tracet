import { useTranscriptStore } from "../stores/transcriptStore";
import { TranscriptSegment } from "./TranscriptSegment";
import {
  useTranscriptAudio,
  usePlayerStatus,
} from "../hooks/useSegmentPlayer";
import { AlertTriangle, CheckCircle2, Loader2 } from "lucide-react";

export function TranscriptViewer() {
  const transcript = useTranscriptStore((s) => s.transcript);
  useTranscriptAudio(transcript?.audio_path ?? null);
  const audioStatus = usePlayerStatus();

  if (!transcript) return null;

  return (
    <div className="mx-auto max-w-3xl px-6 py-6">
      <div className="mb-4">
        <h1 className="text-lg font-medium text-[var(--color-text)]">
          {transcript.source_file}
        </h1>
        <p className="text-xs text-[var(--color-text-muted)]">
          {formatDuration(transcript.duration_ms)} &middot;{" "}
          {transcript.segments.length} segments &middot;{" "}
          {transcript.speakers.length} speakers
        </p>
      </div>

      <AudioStatusBanner status={audioStatus} audioPath={transcript.audio_path} />

      <div className="space-y-1">
        {transcript.segments.map((segment) => (
          <TranscriptSegment key={segment.id} segment={segment} />
        ))}
      </div>
    </div>
  );
}

function AudioStatusBanner({
  status,
  audioPath,
}: {
  status: { state: string; error: string | null; sourcePath: string | null };
  audioPath: string;
}) {
  if (status.state === "loading") {
    return (
      <div className="mb-4 flex items-center gap-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] px-3 py-2 text-xs text-[var(--color-text-muted)]">
        <Loader2 size={12} className="animate-spin" />
        Loading audio for playback...
      </div>
    );
  }
  if (status.state === "error") {
    return (
      <div className="mb-4 rounded border border-red-500/30 bg-red-500/5 px-3 py-2 text-xs text-red-400">
        <div className="flex items-center gap-2 font-medium">
          <AlertTriangle size={12} />
          Audio playback unavailable
        </div>
        <div className="mt-1 text-red-400/80">{status.error}</div>
        <div className="mt-1 break-all text-[10px] text-red-400/60">
          path: {audioPath}
        </div>
      </div>
    );
  }
  if (status.state === "ready") {
    return (
      <div className="mb-4 flex items-center gap-2 rounded border border-[var(--color-border)] bg-[var(--color-surface)] px-3 py-2 text-xs text-[var(--color-text-muted)]">
        <CheckCircle2 size={12} className="text-[var(--color-success)]" />
        Audio ready: click <span className="mx-0.5">▶</span> on any segment to
        hear it
      </div>
    );
  }
  return null;
}

function formatDuration(ms: number): string {
  const totalSecs = Math.floor(ms / 1000);
  const hours = Math.floor(totalSecs / 3600);
  const minutes = Math.floor((totalSecs % 3600) / 60);
  const seconds = totalSecs % 60;
  if (hours > 0) {
    return `${hours}h ${minutes}m ${seconds}s`;
  }
  return `${minutes}m ${seconds}s`;
}
