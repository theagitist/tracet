use serde::{Deserialize, Serialize};
use std::ffi::CString;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub ram_gb: u64,
    pub chip: String,
    pub chip_tier: ChipTier,
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

    Ok(HardwareInfo {
        ram_gb,
        chip,
        chip_tier,
    })
}

// sysctlbyname instead of running `sysctl`: no subprocess, which also keeps
// this working inside the App Sandbox.
fn sysctl_raw(key: &str) -> Option<Vec<u8>> {
    let name = CString::new(key).ok()?;
    let mut len: libc::size_t = 0;
    let rc = unsafe { libc::sysctlbyname(name.as_ptr(), std::ptr::null_mut(), &mut len, std::ptr::null_mut(), 0) };
    if rc != 0 || len == 0 {
        return None;
    }
    let mut buf = vec![0u8; len];
    let rc = unsafe { libc::sysctlbyname(name.as_ptr(), buf.as_mut_ptr().cast(), &mut len, std::ptr::null_mut(), 0) };
    if rc != 0 {
        return None;
    }
    buf.truncate(len);
    Some(buf)
}

fn sysctl_u64(key: &str) -> Option<u64> {
    let buf = sysctl_raw(key)?;
    Some(u64::from_ne_bytes(buf.get(..8)?.try_into().ok()?))
}

fn sysctl_string(key: &str) -> Option<String> {
    let buf = sysctl_raw(key)?;
    let s = String::from_utf8_lossy(&buf).trim_end_matches('\0').trim().to_string();
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
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_ram_and_chip_without_subprocesses() {
        let hw = super::detect_hardware().unwrap();
        assert!(hw.ram_gb > 0);
        assert!(hw.chip.starts_with("Apple") || hw.chip.contains("Intel"), "{}", hw.chip);
    }
}
