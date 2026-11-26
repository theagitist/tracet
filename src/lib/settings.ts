// Lightweight settings persistence using localStorage.
// Centralised so the same keys are used by SettingsDialog (writer)
// and useTranscription (reader).

export interface UserSettings {
  whisperModel: string;
  ollamaModel: string;
  ollamaUrl: string;
  enableLlmReview: boolean;
}

const KEY = "tracet:settings";

const DEFAULTS: UserSettings = {
  whisperModel: "large-v3-turbo",
  ollamaModel: "llama3.1:8b",
  ollamaUrl: "http://localhost:11434",
  enableLlmReview: false,
};

export function loadSettings(): UserSettings {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return { ...DEFAULTS };
    // Up to 0.1.x the HuggingFace token was stored here in plain text. The
    // diarization model now ships with the app, so erase it from storage.
    const { hfToken, ...parsed } = JSON.parse(raw);
    if (hfToken !== undefined) localStorage.setItem(KEY, JSON.stringify(parsed));
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
