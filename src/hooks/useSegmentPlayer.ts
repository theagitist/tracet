import { useEffect, useRef, useSyncExternalStore } from "react";
import { invoke } from "@tauri-apps/api/core";

// A tiny global audio controller. There is one <audio> element shared by all
// segment play buttons so that pressing play on one stops the previous one.
//
// We load the audio file via the fs plugin (returns bytes), wrap it in a Blob
// and use a blob: URL. This avoids asset-protocol scope quirks entirely.

type PlayerState = "idle" | "loading" | "ready" | "error";

class SegmentPlayer {
  private audio: HTMLAudioElement | null = null;
  private currentSegmentId: string | null = null;
  private endStopMs: number | null = null;
  private rafId: number | null = null;
  private blobUrl: string | null = null;
  private currentSourcePath: string | null = null;
  private loadError: string | null = null;
  private state: PlayerState = "idle";
  private listeners = new Set<() => void>();

  private getAudio(): HTMLAudioElement {
    if (!this.audio) {
      this.audio = new Audio();
      this.audio.preload = "auto";
      this.audio.addEventListener("ended", () => {
        this.currentSegmentId = null;
        this.endStopMs = null;
        this.notify();
      });
      this.audio.addEventListener("error", () => {
        const err = this.audio?.error;
        this.loadError = err
          ? `Audio load error (code ${err.code}): ${err.message || "unknown"}`
          : "Unknown audio error";
        console.error("[SegmentPlayer]", this.loadError, this.audio?.src);
        this.currentSegmentId = null;
        this.notify();
      });
    }
    return this.audio;
  }

  async setSource(filePath: string) {
    if (filePath === this.currentSourcePath) return;
    this.currentSourcePath = filePath;
    this.loadError = null;
    this.state = "loading";
    this.notify();

    // Revoke previous blob URL to free memory
    if (this.blobUrl) {
      URL.revokeObjectURL(this.blobUrl);
      this.blobUrl = null;
    }

    try {
      // Custom Tauri command: bypasses tauri-plugin-fs scope checks which
      // are finicky with macOS temp paths (/var/folders/...).
      const bytes = await invoke<number[]>("read_audio_file", { path: filePath });
      const u8 = new Uint8Array(bytes);
      const blob = new Blob([u8 as BlobPart], { type: "audio/wav" });
      this.blobUrl = URL.createObjectURL(blob);

      const audio = this.getAudio();
      audio.src = this.blobUrl;
      audio.load();
      this.state = "ready";
      console.log(
        "[SegmentPlayer] loaded",
        filePath,
        u8.length,
        "bytes, blob URL:",
        this.blobUrl
      );
      this.notify();
    } catch (err) {
      this.loadError = `Failed to read audio file: ${err}`;
      this.state = "error";
      console.error("[SegmentPlayer]", this.loadError, "path:", filePath);
      this.notify();
    }
  }

  play(segmentId: string, startMs: number, endMs: number) {
    if (this.loadError) {
      console.error("[SegmentPlayer] not playing: load error:", this.loadError);
      return;
    }
    const audio = this.getAudio();
    if (!audio.src) {
      console.error("[SegmentPlayer] no source set");
      return;
    }

    audio.currentTime = startMs / 1000;
    this.currentSegmentId = segmentId;
    this.endStopMs = endMs;
    this.notify();

    audio.play().catch((err) => {
      console.error("[SegmentPlayer] play failed:", err);
      this.currentSegmentId = null;
      this.notify();
    });

    this.tickStop();
  }

  toggle(segmentId: string, startMs: number, endMs: number) {
    if (this.currentSegmentId === segmentId) {
      this.stop();
    } else {
      this.play(segmentId, startMs, endMs);
    }
  }

  stop() {
    if (this.audio) {
      this.audio.pause();
    }
    if (this.rafId !== null) {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
    this.currentSegmentId = null;
    this.endStopMs = null;
    this.notify();
  }

  private tickStop() {
    if (this.rafId !== null) cancelAnimationFrame(this.rafId);
    const step = () => {
      if (
        this.audio &&
        this.endStopMs !== null &&
        this.audio.currentTime * 1000 >= this.endStopMs
      ) {
        this.stop();
        return;
      }
      if (this.currentSegmentId !== null) {
        this.rafId = requestAnimationFrame(step);
      }
    };
    this.rafId = requestAnimationFrame(step);
  }

  getPlayingSegmentId(): string | null {
    return this.currentSegmentId;
  }

  getStatusSnapshot(): { state: PlayerState; error: string | null; sourcePath: string | null } {
    return { state: this.state, error: this.loadError, sourcePath: this.currentSourcePath };
  }

  // Stable cached snapshot for useSyncExternalStore (must return the same
  // reference when nothing changed: otherwise React loops).
  private statusCache = {
    state: this.state,
    error: this.loadError,
    sourcePath: this.currentSourcePath,
  };

  getStatusCached() {
    if (
      this.statusCache.state !== this.state ||
      this.statusCache.error !== this.loadError ||
      this.statusCache.sourcePath !== this.currentSourcePath
    ) {
      this.statusCache = {
        state: this.state,
        error: this.loadError,
        sourcePath: this.currentSourcePath,
      };
    }
    return this.statusCache;
  }

  subscribe(cb: () => void): () => void {
    this.listeners.add(cb);
    return () => {
      this.listeners.delete(cb);
    };
  }

  private notify() {
    this.listeners.forEach((cb) => cb());
  }
}

const player = new SegmentPlayer();

export function useSegmentPlayer(segmentId: string) {
  const playingId = useSyncExternalStore(
    (cb) => player.subscribe(cb),
    () => player.getPlayingSegmentId()
  );

  return {
    isPlaying: playingId === segmentId,
    toggle: (startMs: number, endMs: number) =>
      player.toggle(segmentId, startMs, endMs),
  };
}

export function usePlayerStatus() {
  return useSyncExternalStore(
    (cb) => player.subscribe(cb),
    () => player.getStatusCached()
  );
}

export function useTranscriptAudio(filePath: string | null) {
  const lastSetRef = useRef<string | null>(null);
  useEffect(() => {
    if (filePath && filePath !== lastSetRef.current) {
      lastSetRef.current = filePath;
      void player.setSource(filePath);
    }
    return () => {
      player.stop();
    };
  }, [filePath]);
}
