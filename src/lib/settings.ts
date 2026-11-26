// Lightweight settings persistence using localStorage.
// Centralised so the same keys are used by SettingsDialog (writer)
// and useTranscription (reader).

export interface UserSettings {
  whisperModel: string;
}

const KEY = "tracet:settings";

const DEFAULTS: UserSettings = {
  whisperModel: "large-v3-turbo",
};

export function loadSettings(): UserSettings {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return { ...DEFAULTS };
    // Keep only known keys. This erases the HuggingFace token that 0.1.x
    // stored here in plain text and the Ollama settings dropped in 0.2.1.
    const stored = JSON.parse(raw);
    const settings: UserSettings = {
      whisperModel: stored.whisperModel ?? DEFAULTS.whisperModel,
    };
    if (Object.keys(stored).length !== 1 || !("whisperModel" in stored)) {
      localStorage.setItem(KEY, JSON.stringify(settings));
    }
    return settings;
  } catch {
    return { ...DEFAULTS };
  }
}

export function saveSettings(settings: Partial<UserSettings>): void {
  const current = loadSettings();
  const next = { ...current, ...settings };
  localStorage.setItem(KEY, JSON.stringify(next));
}
