use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub models_dir: PathBuf,
    pub hf_token: Option<String>,
    pub default_whisper_model: String,
    pub default_language: Option<String>,
    pub enable_llm_review: bool,
    pub confidence_threshold: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        let models_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("com.tracet.app")
            .join("models");

        Self {
            models_dir,
            hf_token: None,
            default_whisper_model: "ggml-large-v3-turbo.bin".to_string(),
            default_language: None,
            enable_llm_review: true,
            confidence_threshold: 0.7,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("com.tracet.app")
            .join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
