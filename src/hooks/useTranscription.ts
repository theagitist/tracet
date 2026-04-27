import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save, ask } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { useTranscriptStore } from "../stores/transcriptStore";
import { loadSettings } from "../lib/settings";
import type {
  Transcript,
  PipelineOptions,
  PipelineProgress,
  ExportOptions,
} from "../types/transcript";

export function useTranscription() {
  const store = useTranscriptStore();

  const openFile = useCallback(async () => {
    const filePath = await open({
      multiple: false,
      filters: [
        {
          name: "Media Files",
          extensions: [
            "mp3",
            "wav",
            "flac",
            "m4a",
            "ogg",
            "aac",
            "mp4",
            "mkv",
            "avi",
            "webm",
            "mov",
          ],
        },
      ],
    });

    if (!filePath) return;

    const settings = loadSettings();

    // Warn upfront if speaker identification will be skipped, so the user
    // doesn't discover it 88% into a long transcription.
    if (!settings.hfToken.trim()) {
      const proceed = await ask(
        "No HuggingFace token is set, so speaker identification will be skipped: all speech will be attributed to one speaker.\n\nYou can set a token in Settings → HuggingFace Token.\n\nProceed without speaker identification?",
        {
          title: "Speaker identification disabled",
          kind: "warning",
          okLabel: "Proceed",
          cancelLabel: "Cancel",
        }
      );
      if (!proceed) return;
    }

    store.reset();
    store.setSourceFilePath(filePath);

    // Listen for progress events
    const unlisten = await listen<PipelineProgress>(
      "pipeline:progress",
      (event) => {
        store.updateProgress(event.payload);
      }
    );

    try {
      const options: PipelineOptions = {
        whisper_model: settings.whisperModel,
        enable_llm_review: settings.enableLlmReview,
        language: null,
        confidence_threshold: 0.7,
        hf_token: settings.hfToken.trim() || null,
        ollama_model: settings.ollamaModel,
        ollama_url: settings.ollamaUrl,
      };

      const transcript = await invoke<Transcript>("run_pipeline", {
        inputPath: filePath,
        options,
      });

      store.setTranscript(transcript);
    } catch (err) {
      store.setError(String(err));
    } finally {
      unlisten();
    }
  }, [store]);

  const saveProject = useCallback(async () => {
    if (!store.transcript) return;
    const defaultName = store.transcript.source_file.replace(
      /\.[^.]+$/,
      ".tracet"
    );
    const filePath = await save({
      defaultPath: defaultName,
      filters: [{ name: "Tracet Project", extensions: ["tracet"] }],
    });
    if (!filePath) return;
    try {
      await invoke("save_project", {
        transcript: store.transcript,
        outputPath: filePath,
      });
    } catch (err) {
      store.setError(`Failed to save project: ${err}`);
    }
  }, [store]);

  const openProject = useCallback(async () => {
    const filePath = await open({
      multiple: false,
      filters: [{ name: "Tracet Project", extensions: ["tracet"] }],
    });
    if (!filePath) return;
    store.reset();
    try {
      const transcript = await invoke<Transcript>("load_project", {
        inputPath: filePath,
      });
      store.setTranscript(transcript);
    } catch (err) {
      store.setError(`Failed to open project: ${err}`);
    }
  }, [store]);

  const exportTranscript = useCallback(
    async (options: ExportOptions) => {
      if (!store.transcript) return;

      try {
        const content = await invoke<string>("export_transcript", {
          transcript: store.transcript,
          options,
        });

        const ext = options.format === "Md" ? "md" : "txt";
        const defaultName = store.transcript.source_file.replace(
          /\.[^.]+$/,
          `.${ext}`
        );

        const filePath = await save({
          defaultPath: defaultName,
          filters: [
            {
              name: options.format === "Md" ? "Markdown" : "Text",
              extensions: [ext],
            },
          ],
        });

        if (filePath) {
          await writeTextFile(filePath, content);
        }
      } catch (err) {
        store.setError(String(err));
      }
    },
    [store]
  );

  return { openFile, exportTranscript, saveProject, openProject };
}
