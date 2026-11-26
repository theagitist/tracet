use std::path::Path;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::models::transcript::PipelineProgress;
use crate::sidecar::python;

fn emit(app: &AppHandle, percent: u8, message: &str) {
    let _ = app.emit(
        "pipeline:progress",
        PipelineProgress {
            step: "converting".to_string(),
            percent,
            message: message.to_string(),
        },
    );
}

/// Convert any audio/video file to 16 kHz mono WAV with the sidecar's
/// `convert.py` (PyAV), so no ffmpeg binary is needed.
#[tauri::command]
pub async fn convert_media(app: AppHandle, input_path: String) -> Result<String, String> {
    if !Path::new(&input_path).exists() {
        return Err(format!("File not found: {}", input_path));
    }
    emit(&app, 0, "Converting media to WAV...");

    let output_path = tempfile::tempdir()
        .map_err(|e| e.to_string())?
        .keep()
        .join("audio.wav");

    let mut cmd = Command::new(python::python_executable(&app)?);
    python::sidecar_env(&app, &mut cmd);
    let mut child = cmd
        .arg(python::script_dir(&app)?.join("convert.py"))
        .arg(&input_path)
        .arg(&output_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start media conversion: {}", e))?;

    let mut lines = BufReader::new(child.stderr.take().ok_or("No stderr from converter")?).lines();
    let mut error = None;
    while let Ok(Some(line)) = lines.next_line().await {
        if let Some(rest) = line.strip_prefix("TRACET_PROGRESS: ") {
            if let Some((percent, message)) = rest.split_once(' ') {
                if let Ok(p) = percent.parse::<u8>() {
                    // 0 means "keep the current percent", so never send it.
                    emit(&app, p.max(1), message);
                }
            }
        } else if let Some(msg) = line.strip_prefix("TRACET_ERROR: ") {
            error = Some(msg.to_string());
        }
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!(
            "Could not convert this file: {}",
            error.unwrap_or_else(|| format!("converter exited with {}", status))
        ));
    }

    emit(&app, 100, "Media conversion complete");
    Ok(output_path.to_string_lossy().to_string())
}

/// Duration of a PCM WAV in milliseconds, read from its header (the
/// converter always writes PCM, so ffprobe isn't needed).
pub async fn get_duration_ms(input_path: &Path) -> Result<u64, String> {
    let bytes = tokio::fs::read(input_path).await.map_err(|e| e.to_string())?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("Not a WAV file".to_string());
    }
    let mut pos = 12;
    let mut byte_rate = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let body = pos + 8;
        if id == b"fmt " && body + 12 <= bytes.len() {
            byte_rate = Some(u32::from_le_bytes(bytes[body + 8..body + 12].try_into().unwrap()));
        } else if id == b"data" {
            let rate = byte_rate.filter(|r| *r > 0).ok_or("WAV has no format chunk")?;
            let data = size.min(bytes.len() - body) as u64;
            return Ok(data * 1000 / rate as u64);
        }
        pos = body + size + (size & 1);
    }
    Err("WAV has no data chunk".to_string())
}
