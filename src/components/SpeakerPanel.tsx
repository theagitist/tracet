import { useSpeakers } from "../hooks/useSpeakers";

export function SpeakerPanel() {
  const { speakers, handleRename } = useSpeakers();

  if (speakers.length === 0) return null;

  return (
    <div className="p-4">
      <h3 className="mb-3 text-xs font-semibold uppercase tracking-wider text-[var(--color-text-muted)]">
        Speakers
      </h3>
      <div className="space-y-3">
        {speakers.map((speaker) => (
          <div key={speaker.id} className="flex items-center gap-3">
            <div
              className="h-3 w-3 shrink-0 rounded-full"
              style={{ backgroundColor: speaker.color }}
            />
            <div className="min-w-0 flex-1">
              <div className="text-xs text-[var(--color-text-muted)]">
                {speaker.id}
              </div>
              <input
                type="text"
                placeholder="Enter name..."
                value={speaker.name || ""}
                onChange={(e) => handleRename(speaker.id, e.target.value)}
                className="w-full border-b border-transparent bg-transparent text-sm text-[var(--color-text)] placeholder-[var(--color-text-muted)] outline-none transition-colors focus:border-[var(--color-accent)]"
              />
            </div>
          </div>
        ))}
      </div>

      <p className="mt-4 text-xs text-[var(--color-text-muted)]">
        Assign names to each detected speaker. Names will appear in the
        transcript and export.
      </p>
    </div>
  );
}
