import { useCallback } from "react";
import { useTranscriptStore } from "../stores/transcriptStore";

export function useSpeakers() {
  const transcript = useTranscriptStore((s) => s.transcript);
  const renameSpeaker = useTranscriptStore((s) => s.renameSpeaker);

  const speakers = transcript?.speakers ?? [];

  const handleRename = useCallback(
    (speakerId: string, newName: string) => {
      renameSpeaker(speakerId, newName);
    },
    [renameSpeaker]
  );

  return { speakers, handleRename };
}
