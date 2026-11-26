import { useState, useEffect } from "react";
import {
  X,
  Download,
  Loader2,
  Star,
  Cpu,
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
  const [pythonReady, setPythonReady] = useState<boolean | null>(null);
  const [settingUpPython, setSettingUpPython] = useState(false);

  // Persist any change immediately so the next pipeline run picks them up.
  useEffect(() => {
    saveSettings({ whisperModel: selectedWhisper });
  }, [selectedWhisper]);

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
                {hardware.chip}
              </div>
              <div className="text-xs text-[var(--color-text-muted)]">
                {hardware.ram_gb} GB RAM
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

        {/* Actions. The credit is required by the model's CC BY 4.0 licence. */}
        <div className="flex items-end justify-between gap-4">
          <p className="text-xs text-[var(--color-text-muted)]">
            Speaker identification uses pyannote/speaker-diarization-community-1
            by pyannoteAI, licensed under CC BY 4.0.
          </p>
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
