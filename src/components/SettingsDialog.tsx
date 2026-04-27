import { useState, useEffect } from "react";
import {
  X,
  Download,
  Loader2,
  ExternalLink,
  Star,
  Cpu,
  AlertTriangle,
  Zap,
  Crown,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import type { HardwareInfo, ModelProfile } from "../types/transcript";
import { loadSettings, saveSettings } from "../lib/settings";

interface Props {
  onClose: () => void;
}

export function SettingsDialog({ onClose }: Props) {
  const initial = loadSettings();
  const [hardware, setHardware] = useState<HardwareInfo | null>(null);
  const [profiles, setProfiles] = useState<ModelProfile[]>([]);
  const [selectedWhisper, setSelectedWhisper] = useState(initial.whisperModel);
  const [selectedOllama, setSelectedOllama] = useState(initial.ollamaModel);
  const [hfToken, setHfToken] = useState(initial.hfToken);
  const [ollamaUrl, setOllamaUrl] = useState(initial.ollamaUrl);
  const [enableLlmReview] = useState(initial.enableLlmReview);
  const [ollamaStatus, setOllamaStatus] = useState<
    "checking" | "connected" | "disconnected"
  >("checking");
  const [pythonReady, setPythonReady] = useState<boolean | null>(null);
  const [settingUpPython, setSettingUpPython] = useState(false);

  // Persist any change immediately so the next pipeline run picks them up.
  useEffect(() => {
    saveSettings({
      whisperModel: selectedWhisper,
      ollamaModel: selectedOllama,
      hfToken,
      ollamaUrl,
      enableLlmReview,
    });
  }, [selectedWhisper, selectedOllama, hfToken, ollamaUrl, enableLlmReview]);

  useEffect(() => {
    // Detect hardware and get available profiles
    invoke<HardwareInfo>("detect_hardware").then((hw) => {
      setHardware(hw);
      invoke<ModelProfile[]>("get_available_profiles", {
        hardware: hw,
      }).then(setProfiles);
    });

    invoke<boolean>("check_python_sidecar")
      .then(setPythonReady)
      .catch(() => setPythonReady(false));
  }, []);

  useEffect(() => {
    setOllamaStatus("checking");
    fetch(`${ollamaUrl}/api/tags`)
      .then((r) => setOllamaStatus(r.ok ? "connected" : "disconnected"))
      .catch(() => setOllamaStatus("disconnected"));
  }, [ollamaUrl]);

  const handleSetupPython = async () => {
    setSettingUpPython(true);
    try {
      await invoke("setup_python_sidecar");
      setPythonReady(true);
    } catch (err) {
      console.error("Failed to set up Python:", err);
    } finally {
      setSettingUpPython(false);
    }
  };

  const whisperProfiles = profiles.filter((p) => p.category === "Whisper");
  const ollamaProfiles = profiles.filter((p) => p.category === "Ollama");

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="max-h-[85vh] w-[560px] overflow-y-auto rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] p-6 shadow-xl">
        <div className="mb-5 flex items-center justify-between">
          <h2 className="text-base font-medium text-[var(--color-text)]">
            Settings
          </h2>
          <button
            onClick={onClose}
            className="rounded p-1 text-[var(--color-text-muted)] hover:bg-[var(--color-surface-hover)]"
          >
            <X size={16} />
          </button>
        </div>

        {/* Hardware info banner */}
        {hardware && (
          <div className="mb-5 flex items-center gap-3 rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-4 py-3">
            <Cpu size={18} className="shrink-0 text-[var(--color-accent)]" />
            <div>
              <div className="text-sm font-medium text-[var(--color-text)]">
                {hardware.model_name} &mdash; {hardware.chip}
              </div>
              <div className="text-xs text-[var(--color-text-muted)]">
                {hardware.ram_gb} GB RAM
                {hardware.gpu_cores
                  ? ` \u00B7 ${hardware.gpu_cores}-core GPU`
                  : ""}
                {" \u00B7 "}
                Model options filtered for your hardware
              </div>
            </div>
          </div>
        )}

        {/* Python / whisperX environment */}
        <section className="mb-5">
          <h3 className="mb-2 text-sm font-medium text-[var(--color-text)]">
            Transcription Engine
          </h3>
          <div className="mb-3 flex items-center gap-3">
            <StatusDot
              status={
                pythonReady === null
                  ? "checking"
                  : pythonReady
                    ? "ok"
                    : "error"
              }
            />
            <span className="text-sm text-[var(--color-text)]">
              {pythonReady === null
                ? "Checking whisperX environment..."
                : pythonReady
                  ? "whisperX ready"
                  : "whisperX not installed"}
            </span>
            {!pythonReady && pythonReady !== null && (
              <button
                onClick={handleSetupPython}
                disabled={settingUpPython}
                className="ml-auto flex items-center gap-1 rounded bg-[var(--color-accent)] px-3 py-1.5 text-xs font-medium text-white disabled:opacity-50"
              >
                {settingUpPython ? (
                  <Loader2 size={12} className="animate-spin" />
                ) : (
                  <Download size={12} />
                )}
                {settingUpPython ? "Installing..." : "Install"}
              </button>
            )}
          </div>
        </section>

        {/* Whisper model selection */}
        <section className="mb-5">
          <div className="mb-1 flex items-center justify-between">
            <h3 className="text-sm font-medium text-[var(--color-text)]">
              Transcription Model
            </h3>
            <span className="text-xs text-[var(--color-text-muted)]">
              Quality vs. Speed
            </span>
          </div>
          <p className="mb-3 text-xs text-[var(--color-text-muted)]">
            Downloaded automatically on first use. Higher quality = slower
            processing.
          </p>
          <div className="space-y-2">
            {whisperProfiles.map((profile) => (
              <ModelCard
                key={profile.id}
                profile={profile}
                selected={selectedWhisper === profile.id}
                onSelect={() => {
                  if (profile.available) setSelectedWhisper(profile.id);
                }}
              />
            ))}
          </div>
        </section>

        {/* HuggingFace Token */}
        <section className="mb-5">
          <h3 className="mb-1 text-sm font-medium text-[var(--color-text)]">
            HuggingFace Token
          </h3>
          <p className="mb-2 text-xs text-[var(--color-text-muted)]">
            Required for speaker identification. Without it, transcription works
            but all speech is attributed to one speaker.
          </p>
          <input
            type="password"
            value={hfToken}
            onChange={(e) => setHfToken(e.target.value)}
            placeholder="hf_..."
            className="w-full rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm text-[var(--color-text)] placeholder-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)]"
          />
        </section>

        {/* Ollama LLM review */}
        <section className="mb-5">
          <div className="mb-1 flex items-center justify-between">
            <h3 className="text-sm font-medium text-[var(--color-text)]">
              Accuracy Review Model (Ollama)
            </h3>
            <div className="flex items-center gap-2">
              <StatusDot
                status={
                  ollamaStatus === "checking"
                    ? "checking"
                    : ollamaStatus === "connected"
                      ? "ok"
                      : "error"
                }
              />
              <span className="text-xs text-[var(--color-text-muted)]">
                {ollamaStatus === "connected"
                  ? "Connected"
                  : ollamaStatus === "checking"
                    ? "Checking..."
                    : "Not running"}
              </span>
            </div>
          </div>

          {ollamaStatus === "disconnected" && (
            <div className="mb-3 flex items-start gap-2 rounded border border-yellow-500/20 bg-yellow-500/5 px-3 py-2">
              <AlertTriangle
                size={14}
                className="mt-0.5 shrink-0 text-[var(--color-warning)]"
              />
              <div className="text-xs text-[var(--color-text-muted)]">
                Ollama is not running. Review will fall back to
                confidence-based flagging only.{" "}
                <a
                  href="https://ollama.ai"
                  target="_blank"
                  rel="noopener"
                  className="inline-flex items-center gap-0.5 text-[var(--color-accent)] hover:underline"
                >
                  Install Ollama <ExternalLink size={10} />
                </a>
              </div>
            </div>
          )}

          <p className="mb-3 text-xs text-[var(--color-text-muted)]">
            A local LLM reviews the transcript for errors. Larger models catch
            more subtle mistakes. Ollama is free and runs locally.
          </p>

          <div className="space-y-2">
            {ollamaProfiles.map((profile) => (
              <ModelCard
                key={profile.id}
                profile={profile}
                selected={selectedOllama === profile.id}
                onSelect={() => {
                  if (profile.available) setSelectedOllama(profile.id);
                }}
              />
            ))}
          </div>

          <div className="mt-3">
            <label className="mb-1 block text-xs text-[var(--color-text-muted)]">
              Ollama URL
            </label>
            <input
              type="text"
              value={ollamaUrl}
              onChange={(e) => setOllamaUrl(e.target.value)}
              className="w-full rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm text-[var(--color-text)] outline-none focus:border-[var(--color-accent)]"
            />
          </div>
        </section>

        {/* Actions */}
        <div className="flex justify-end">
          <button
            onClick={onClose}
            className="rounded bg-[var(--color-accent)] px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-[var(--color-accent-hover)]"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
}

function ModelCard({
  profile,
  selected,
  onSelect,
}: {
  profile: ModelProfile;
  selected: boolean;
  onSelect: () => void;
}) {
  const isBest = profile.quality_stars === 5 && profile.available;

  return (
    <button
      onClick={onSelect}
      disabled={!profile.available}
      className={`w-full rounded border text-left transition-colors ${
        !profile.available
          ? "cursor-not-allowed border-[var(--color-border)] bg-[var(--color-bg)] opacity-40"
          : selected
            ? "border-[var(--color-accent)] bg-[var(--color-accent)]/5"
            : "border-[var(--color-border)] bg-[var(--color-bg)] hover:border-[var(--color-accent)]/50"
      } px-3 py-2.5`}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-[var(--color-text)]">
              {profile.name}
            </span>
            {isBest && (
              <span className="flex items-center gap-0.5 rounded-full bg-[var(--color-accent)]/10 px-1.5 py-0.5 text-[10px] font-medium text-[var(--color-accent)]">
                <Crown size={10} /> Best
              </span>
            )}
            {!profile.available && (
              <span className="text-[10px] text-[var(--color-danger)]">
                Needs {profile.ram_required_gb}GB+ RAM
              </span>
            )}
          </div>
          <div className="mt-0.5 flex items-center gap-3 text-xs text-[var(--color-text-muted)]">
            <span className="flex items-center gap-1">
              <Stars count={profile.quality_stars} /> Quality
            </span>
            <span className="flex items-center gap-1">
              <Zap size={10} /> {profile.speed_label}
            </span>
            <span>{profile.estimated_realtime_factor}</span>
          </div>
          {profile.shortcomings.length > 0 && selected && (
            <ul className="mt-1.5 space-y-0.5">
              {profile.shortcomings.map((s, i) => (
                <li
                  key={i}
                  className="flex items-start gap-1 text-[11px] text-[var(--color-text-muted)]"
                >
                  <span className="mt-0.5 shrink-0">&#x2022;</span>
                  {s}
                </li>
              ))}
            </ul>
          )}
        </div>
        <div
          className={`mt-1 h-4 w-4 shrink-0 rounded-full border-2 transition-colors ${
            selected
              ? "border-[var(--color-accent)] bg-[var(--color-accent)]"
              : "border-[var(--color-border)]"
          }`}
        >
          {selected && (
            <div className="flex h-full items-center justify-center">
              <div className="h-1.5 w-1.5 rounded-full bg-white" />
            </div>
          )}
        </div>
      </div>
    </button>
  );
}

function Stars({ count }: { count: number }) {
  return (
    <span className="flex gap-px">
      {Array.from({ length: 5 }, (_, i) => (
        <Star
          key={i}
          size={10}
          className={
            i < count ? "fill-[var(--color-warning)] text-[var(--color-warning)]" : "text-[var(--color-border)]"
          }
        />
      ))}
    </span>
  );
}

function StatusDot({
  status,
}: {
  status: "checking" | "ok" | "error";
}) {
  return (
    <div
      className={`h-2 w-2 rounded-full ${
        status === "checking"
          ? "bg-gray-500"
          : status === "ok"
            ? "bg-[var(--color-success)]"
            : "bg-[var(--color-danger)]"
      }`}
    />
  );
}
