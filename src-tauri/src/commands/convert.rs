use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::process::Command;

use crate::models::transcript::PipelineProgress;

/// Convert any audio/video file to 16kHz mono WAV using FFmpeg.
#[tauri::command]
pub async fn convert_media(app: AppHandle, input_path: String) -> Result<String, String> {
    let input = Path::new(&input_path);
    if !input.exists() {
        return Err(format!("File not found: {}", input_path));
    }

    app.emit(
        "pipeline:progress",
        PipelineProgress {
            step: "converting".to_string(),
            percent: 0,
            message: "Converting media to WAV...".to_string(),
        },
    )
    .map_err(|e| e.to_string())?;

    let temp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let output_path = temp_dir.into_path().join("audio.wav");

    let output = Command::new("ffmpeg")
        .args([
            "-i",
            &input_path,
            "-ar",
            "16000",
            "-ac",
            "1",
            "-f",
            "wav",
            "-y",
            output_path.to_str().unwrap(),
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run FFmpeg: {}. Is FFmpeg installed?", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("FFmpeg conversion failed: {}", stderr));
    }

    app.emit(
        "pipeline:progress",
        PipelineProgress {
            step: "converting".to_string(),
            percent: 100,
            message: "Media conversion complete".to_string(),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(output_path.to_string_lossy().to_string())
}

/// Get the duration of a media file in milliseconds using FFprobe.
pub async fn get_duration_ms(input_path: &Path) -> Result<u64, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            input_path.to_str().unwrap(),
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run ffprobe: {}", e))?;

    let duration_str = String::from_utf8_lossy(&output.stdout);
    let duration_secs: f64 = duration_str
        .trim()
        .parse()
        .map_err(|_| "Failed to parse duration".to_string())?;

    Ok((duration_secs * 1000.0) as u64)
}
