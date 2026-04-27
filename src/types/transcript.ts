export type SegmentType = "Speech" | "Noise" | "Music" | "Silence" | "Unknown";

export interface TokenConfidence {
  token: string;
  confidence: number;
}

export interface TranscriptSegment {
  id: string;
  start_ms: number;
  end_ms: number;
  text: string;
  speaker_id: string | null;
  speaker_name: string | null;
  confidence: number;
  token_confidences: TokenConfidence[];
  is_flagged: boolean;
  flag_reason: string | null;
  segment_type: SegmentType;
}

export interface Speaker {
  id: string;
  name: string | null;
  color: string;
}

export interface Transcript {
  id: string;
  source_file: string;
  audio_path: string;
  segments: TranscriptSegment[];
  speakers: Speaker[];
  created_at: string;
  duration_ms: number;
}

export interface PipelineOptions {
  whisper_model: string;
  enable_llm_review: boolean;
  language: string | null;
  confidence_threshold: number;
  hf_token: string | null;
  ollama_model: string | null;
  ollama_url: string | null;
}

export interface PipelineProgress {
  step: string;
  percent: number;
  message: string;
}

export interface ExportOptions {
  format: "Txt" | "Md" | "AiMd";
  include_timestamps: boolean;
  include_speaker_labels: boolean;
  highlight_low_confidence: boolean;
  include_annotations: boolean;
}

export type ChipTier = "Base" | "Pro" | "Max" | "Other";

export interface HardwareInfo {
  ram_gb: number;
  chip: string;
  chip_tier: ChipTier;
  gpu_cores: number | null;
  model_name: string;
}

export type ProfileCategory = "Whisper" | "Ollama";

export interface ModelProfile {
  category: ProfileCategory;
  id: string;
  name: string;
  quality_stars: number;
  speed_label: string;
  estimated_realtime_factor: string;
  ram_required_gb: number;
  shortcomings: string[];
  available: boolean;
}
