import { create } from "zustand";
import type {
  Transcript,
  Speaker,
  TranscriptSegment,
  PipelineProgress,
} from "../types/transcript";

type ProcessingState =
  | "idle"
  | "converting"
  | "transcribing"
  | "diarizing"
  | "reviewing"
  | "done"
  | "error";

interface TranscriptStore {
  transcript: Transcript | null;
  processingState: ProcessingState;
  progress: number;
  progressMessage: string;
  error: string | null;
  sourceFilePath: string | null;

  setTranscript: (t: Transcript) => void;
  updateSegmentText: (segmentId: string, newText: string) => void;
  renameSpeaker: (speakerId: string, newName: string) => void;
  updateProgress: (progress: PipelineProgress) => void;
  setError: (error: string) => void;
  reset: () => void;
  setSourceFilePath: (path: string) => void;
}

export const useTranscriptStore = create<TranscriptStore>((set) => ({
  transcript: null,
  processingState: "idle",
  progress: 0,
  progressMessage: "",
  error: null,
  sourceFilePath: null,

  setTranscript: (t) =>
    set({
      transcript: t,
      processingState: "done",
      progress: 100,
      progressMessage: "Complete",
      error: null,
    }),

  updateSegmentText: (segmentId, newText) =>
    set((state) => {
      if (!state.transcript) return state;
      const segments = state.transcript.segments.map(
        (seg: TranscriptSegment) =>
          seg.id === segmentId ? { ...seg, text: newText } : seg
      );
      return {
        transcript: { ...state.transcript, segments },
      };
    }),

  renameSpeaker: (speakerId, newName) =>
    set((state) => {
      if (!state.transcript) return state;

      const speakers = state.transcript.speakers.map((s: Speaker) =>
        s.id === speakerId ? { ...s, name: newName || null } : s
      );

      const segments = state.transcript.segments.map(
        (seg: TranscriptSegment) =>
          seg.speaker_id === speakerId
            ? { ...seg, speaker_name: newName || null }
            : seg
      );

      return {
        transcript: { ...state.transcript, speakers, segments },
      };
    }),

  updateProgress: (p) =>
    set((state) => {
      const stepMap: Record<string, ProcessingState> = {
        converting: "converting",
        transcribing: "transcribing",
        diarizing: "diarizing",
        reviewing: "reviewing",
        done: "done",
      };
      // percent === 0 is a sentinel meaning "keep current percent, just update message".
      const newProgress = p.percent === 0 ? state.progress : p.percent;
      return {
        processingState: stepMap[p.step] || "idle",
        progress: newProgress,
        progressMessage: p.message,
      };
    }),

  setError: (error) =>
    set({
      error,
      processingState: "error",
    }),

  reset: () =>
    set({
      transcript: null,
      processingState: "idle",
      progress: 0,
      progressMessage: "",
      error: null,
      sourceFilePath: null,
    }),

  setSourceFilePath: (path) => set({ sourceFilePath: path }),
}));
