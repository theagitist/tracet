// Lightweight settings persistence using localStorage.
// Centralised so the same keys are used by SettingsDialog (writer)
// and useTranscription (reader).

export interface UserSettings {
  hfToken: string;
  whisperModel: string;
  ollamaModel: string;
  ollamaUrl: string;
  enableLlmReview: boolean;
}

const KEY = "tracet:settings";

const DEFAULTS: UserSettings = {
  hfToken: "",
  whisperModel: "large-v3-turbo",
  ollamaModel: "llama3.1:8b",
  ollamaUrl: "http://localhost:11434",
  enableLlmReview: false,
};

export function loadSettings(): UserSettings {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return { ...DEFAULTS };
    const parsed = JSON.parse(raw);
    return { ...DEFAULTS, ...parsed };
  } catch {
    return { ...DEFAULTS };
  }
}

export function saveSettings(settings: Partial<UserSettings>): void {
  const current = loadSettings();
  const next = { ...current, ...settings };
  localStorage.setItem(KEY, JSON.stringify(next));
}
