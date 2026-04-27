import { useTranscriptStore } from "../stores/transcriptStore";

const STEP_LABELS: Record<string, string> = {
  converting: "Converting media",
  transcribing: "Transcribing audio",
  diarizing: "Identifying speakers",
  reviewing: "Reviewing accuracy",
};

const STEPS = ["converting", "transcribing", "diarizing", "reviewing"];

export function ProgressIndicator() {
  const processingState = useTranscriptStore((s) => s.processingState);
  const progress = useTranscriptStore((s) => s.progress);
  const progressMessage = useTranscriptStore((s) => s.progressMessage);

  const stepLabel = STEP_LABELS[processingState] || processingState;
  const currentStepIndex = STEPS.indexOf(processingState);

  // While the heartbeat is firing (message has "elapsed"), the percent is fixed
  // and meaningless: show an indeterminate animated bar instead.
  const isWorking = / \(\d.* elapsed/.test(progressMessage);

  return (
    <div className="flex flex-1 flex-col items-center justify-center px-6">
      <div className="w-full max-w-md">
        <div className="mb-3 flex items-center justify-between text-base">
          <span className="font-medium text-[var(--color-text)]">
            {stepLabel}
          </span>
          {!isWorking && (
            <span className="text-sm tabular-nums text-[var(--color-text-muted)]">
              {progress}%
            </span>
          )}
        </div>

        <div className="relative h-2 overflow-hidden rounded-full bg-[var(--color-surface)]">
          {isWorking ? (
            <div className="absolute inset-y-0 w-1/3 animate-[indeterminate_1.6s_ease-in-out_infinite] rounded-full bg-[var(--color-accent)]" />
          ) : (
            <div
              className="h-full rounded-full bg-[var(--color-accent)] transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          )}
        </div>

        <ProgressMessage message={progressMessage} working={isWorking} />

        <div className="mt-5 flex justify-center gap-2">
          {STEPS.map((step, i) => (
            <div
              key={step}
              className={`h-2 w-2 rounded-full transition-colors ${
                i < currentStepIndex
                  ? "bg-[var(--color-success)]"
                  : i === currentStepIndex
                    ? "bg-[var(--color-accent)]"
                    : "bg-[var(--color-border)]"
              }`}
              title={STEP_LABELS[step]}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function ProgressMessage({
  message,
  working,
}: {
  message: string;
  working: boolean;
}) {
  // If the message has a parenthesised suffix (e.g. "(39s elapsed, still working...)"),
  // render it on its own line with a smaller font for hierarchy.
  const parenIdx = message.indexOf(" (");
  if (parenIdx === -1) {
    return (
      <p className="mt-3 text-center text-sm text-[var(--color-text)]">
        {message}
      </p>
    );
  }
  return (
    <div className="mt-3 text-center">
      <p className="text-sm text-[var(--color-text)]">
        {message.slice(0, parenIdx)}
      </p>
      <p
        className={`mt-1 text-xs tabular-nums text-[var(--color-text-muted)] ${
          working ? "animate-pulse" : ""
        }`}
      >
        {message.slice(parenIdx + 1)}
      </p>
    </div>
  );
}
