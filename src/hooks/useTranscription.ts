import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
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
        language: null,
        confidence_threshold: 0.7,
        // The diarization model ships with the app; no token needed.
        hf_token: null,
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
      store.markSaved();
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
      store.setLoadedTranscript(transcript);
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

        const formatInfo =
          options.format === "AiMd"
            ? { ext: "ai.md", filterName: "AI-friendly Markdown", filterExts: ["md"] }
            : options.format === "Md"
              ? { ext: "md", filterName: "Markdown", filterExts: ["md"] }
              : { ext: "txt", filterName: "Text", filterExts: ["txt"] };

        const defaultName = store.transcript.source_file.replace(
          /\.[^.]+$/,
          `.${formatInfo.ext}`
        );

        const filePath = await save({
          defaultPath: defaultName,
          filters: [
            { name: formatInfo.filterName, extensions: formatInfo.filterExts },
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
