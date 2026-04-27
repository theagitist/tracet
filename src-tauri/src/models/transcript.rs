use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SegmentType {
    Speech,
    Noise,
    Music,
    Silence,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfidence {
    pub token: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub speaker_id: Option<String>,
    pub speaker_name: Option<String>,
    pub confidence: f32,
    pub token_confidences: Vec<TokenConfidence>,
    pub is_flagged: bool,
    pub flag_reason: Option<String>,
    pub segment_type: SegmentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Speaker {
    pub id: String,
    pub name: Option<String>,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub id: String,
    pub source_file: String,
    /// Absolute path to the 16kHz mono WAV produced during the pipeline.
    /// Used by the frontend to play back audio for each segment.
    pub audio_path: String,
    pub segments: Vec<TranscriptSegment>,
    pub speakers: Vec<Speaker>,
    pub created_at: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiarizationSegment {
    pub start: f64,
    pub end: f64,
    pub speaker: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOptions {
    pub whisper_model: String,
    pub enable_llm_review: bool,
    pub language: Option<String>,
    pub confidence_threshold: f32,
    pub hf_token: Option<String>,
    pub ollama_model: Option<String>,
    pub ollama_url: Option<String>,
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self {
            whisper_model: "large-v3".to_string(),
            enable_llm_review: true,
            language: None,
            confidence_threshold: 0.7,
            hf_token: None,
            ollama_model: Some("llama3.1:8b".to_string()),
            ollama_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineProgress {
    pub step: String,
    pub percent: u8,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub include_timestamps: bool,
    pub include_speaker_labels: bool,
    pub highlight_low_confidence: bool,
    pub include_annotations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExportFormat {
    Txt,
    Md,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Md,
            include_timestamps: true,
            include_speaker_labels: true,
            highlight_low_confidence: true,
            include_annotations: true,
        }
    }
}
