use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub ram_gb: u64,
    pub chip: String,
    pub chip_tier: ChipTier,
    pub gpu_cores: Option<u32>,
    pub model_name: String,
}

/// Simplified tier for deciding which models are viable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChipTier {
    /// M1/M2/M3 base: 8-core GPU, shared memory
    Base,
    /// M1/M2/M3 Pro: 14-18 core GPU, more memory bandwidth
    Pro,
    /// M1/M2/M3 Max/Ultra: 30+ core GPU, high bandwidth
    Max,
    /// Intel or unknown
    Other,
}

/// Detect the hardware of the current machine.
#[tauri::command]
pub fn detect_hardware() -> Result<HardwareInfo, String> {
    let ram_bytes = sysctl_u64("hw.memsize").unwrap_or(0);
    let ram_gb = ram_bytes / (1024 * 1024 * 1024);

    let chip = sysctl_string("machdep.cpu.brand_string")
        .unwrap_or_else(|| "Unknown".to_string());

    let chip_tier = classify_chip(&chip);

    let gpu_cores = get_gpu_cores();

    let model_name = get_model_name().unwrap_or_else(|| "Mac".to_string());

    Ok(HardwareInfo {
        ram_gb,
        chip,
        chip_tier,
        gpu_cores,
        model_name,
    })
}

fn sysctl_u64(key: &str) -> Option<u64> {
    let output = Command::new("sysctl")
        .args(["-n", key])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&output.stdout);
    s.trim().parse().ok()
}

fn sysctl_string(key: &str) -> Option<String> {
    let output = Command::new("sysctl")
        .args(["-n", key])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn classify_chip(chip: &str) -> ChipTier {
    let lower = chip.to_lowercase();
    if lower.contains("ultra") {
        ChipTier::Max
    } else if lower.contains("max") {
        ChipTier::Max
    } else if lower.contains("pro") {
        ChipTier::Pro
    } else if lower.contains("apple m") {
        ChipTier::Base
    } else {
        ChipTier::Other
    }
}

fn get_gpu_cores() -> Option<u32> {
    let output = Command::new("system_profiler")
        .args(["SPDisplaysDataType"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    // Look for "Total Number of Cores: 18" or similar
    for line in text.lines() {
        if line.contains("Total Number of Cores") {
            let parts: Vec<&str> = line.split(':').collect();
            if let Some(num_str) = parts.get(1) {
                return num_str.trim().parse().ok();
            }
        }
    }
    None
}

fn get_model_name() -> Option<String> {
    let output = Command::new("system_profiler")
        .args(["SPHardwareDataType"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Model Name:") {
            return Some(trimmed.replace("Model Name:", "").trim().to_string());
        }
    }
    None
}

/// Given hardware info, return which model profiles are available.
#[tauri::command]
pub fn get_available_profiles(hardware: HardwareInfo) -> Vec<ModelProfile> {
    let ram = hardware.ram_gb;
    let mut profiles = Vec::new();

    // --- Whisper models ---
    // All machines can run small
    profiles.push(ModelProfile {
        category: ProfileCategory::Whisper,
        id: "small".to_string(),
        name: "Whisper Small".to_string(),
        quality_stars: 2,
        speed_label: "Very Fast".to_string(),
        estimated_realtime_factor: "~10x faster than real-time".to_string(),
        ram_required_gb: 2,
        shortcomings: vec![
            "Noticeably lower accuracy on accented speech".to_string(),
            "More errors on technical vocabulary".to_string(),
            "Weaker at distinguishing similar-sounding words".to_string(),
        ],
        available: true,
    });

    // 8GB+ can run medium
    profiles.push(ModelProfile {
        category: ProfileCategory::Whisper,
        id: "medium".to_string(),
        name: "Whisper Medium".to_string(),
        quality_stars: 3,
        speed_label: "Fast".to_string(),
        estimated_realtime_factor: "~5x faster than real-time".to_string(),
        ram_required_gb: 5,
        shortcomings: vec![
            "Good general accuracy but not best-in-class".to_string(),
            "May struggle with heavy accents or domain jargon".to_string(),
        ],
        available: ram >= 8,
    });

    // 8GB+ can run large-v3-turbo (smaller than full large-v3)
    profiles.push(ModelProfile {
        category: ProfileCategory::Whisper,
        id: "large-v3-turbo".to_string(),
        name: "Whisper Large V3 Turbo".to_string(),
        quality_stars: 4,
        speed_label: "Moderate".to_string(),
        estimated_realtime_factor: "~2-3x faster than real-time".to_string(),
        ram_required_gb: 6,
        shortcomings: vec![
            "~97% of Large V3 accuracy: rarely noticeable difference".to_string(),
        ],
        available: ram >= 8,
    });

    // 12GB+ for full large-v3 (needs ~10GB with alignment model loaded)
    profiles.push(ModelProfile {
        category: ProfileCategory::Whisper,
        id: "large-v3".to_string(),
        name: "Whisper Large V3".to_string(),
        quality_stars: 5,
        speed_label: "Slower".to_string(),
        estimated_realtime_factor: "~1-2x real-time".to_string(),
        ram_required_gb: 10,
        shortcomings: vec![],
        available: ram >= 12,
    });

    // --- Ollama models ---
    // 8GB machines: only small models
    profiles.push(ModelProfile {
        category: ProfileCategory::Ollama,
        id: "llama3.2:3b".to_string(),
        name: "Llama 3.2 3B".to_string(),
        quality_stars: 2,
        speed_label: "Very Fast".to_string(),
        estimated_realtime_factor: "~50 tokens/sec".to_string(),
        ram_required_gb: 4,
        shortcomings: vec![
            "Limited reasoning: may miss subtle transcription errors".to_string(),
            "Less reliable JSON output".to_string(),
        ],
        available: true,
    });

    profiles.push(ModelProfile {
        category: ProfileCategory::Ollama,
        id: "llama3.1:8b".to_string(),
        name: "Llama 3.1 8B".to_string(),
        quality_stars: 3,
        speed_label: "Fast".to_string(),
        estimated_realtime_factor: "~30 tokens/sec".to_string(),
        ram_required_gb: 6,
        shortcomings: vec![
            "Good at catching obvious errors".to_string(),
            "May miss context-dependent mistakes".to_string(),
        ],
        available: ram >= 8,
    });

    // 16GB+ for 14B class models
    profiles.push(ModelProfile {
        category: ProfileCategory::Ollama,
        id: "qwen2.5:14b".to_string(),
        name: "Qwen 2.5 14B".to_string(),
        quality_stars: 4,
        speed_label: "Moderate".to_string(),
        estimated_realtime_factor: "~15 tokens/sec".to_string(),
        ram_required_gb: 12,
        shortcomings: vec![
            "Strong reasoning, good at context-dependent corrections".to_string(),
        ],
        available: ram >= 16,
    });

    // 18GB+ (like our M3 Pro) for the top tier we'll support
    profiles.push(ModelProfile {
        category: ProfileCategory::Ollama,
        id: "qwen2.5:32b".to_string(),
        name: "Qwen 2.5 32B".to_string(),
        quality_stars: 5,
        speed_label: "Slow".to_string(),
        estimated_realtime_factor: "~8 tokens/sec".to_string(),
        ram_required_gb: 18,
        shortcomings: vec![],
        available: ram >= 18,
    });

    // --- Diarization note: pyannote always runs, no model choice needed ---

    profiles
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub category: ProfileCategory,
    pub id: String,
    pub name: String,
    /// 1-5 quality rating
    pub quality_stars: u8,
    pub speed_label: String,
    pub estimated_realtime_factor: String,
    pub ram_required_gb: u64,
    pub shortcomings: Vec<String>,
    /// Whether this profile can run on the detected hardware
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProfileCategory {
    Whisper,
    Ollama,
}
