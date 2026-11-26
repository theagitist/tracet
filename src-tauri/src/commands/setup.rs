use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::models::transcript::PipelineProgress;
use crate::sidecar::python;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStatus {
    pub ffmpeg_installed: bool,
    pub python_available: bool,
    pub venv_ready: bool,
    pub needs_setup: bool,
}

/// Check what's installed and what needs setup.
#[tauri::command]
pub async fn check_setup_status(app: AppHandle) -> Result<SetupStatus, String> {
    // Media decoding now uses PyAV inside the sidecar; no ffmpeg binary.
    let ffmpeg_installed = true;
    let venv_ready = python::is_runtime_ready(&app);
    // Once the venv exists the system interpreter no longer matters.
    let python_available = venv_ready || python::find_python_interpreter().await.is_ok();
    let needs_setup = !ffmpeg_installed || !python_available || !venv_ready;

    Ok(SetupStatus {
        ffmpeg_installed,
        python_available,
        venv_ready,
        needs_setup,
    })
}

/// Run the full first-time setup.
#[tauri::command]
pub async fn run_setup(app: AppHandle) -> Result<String, String> {
    // Step 1: Check Python
    emit_setup_progress(&app, "Checking Python...", 25);
    if !python::is_runtime_ready(&app) {
        let interpreter = python::find_python_interpreter().await?;
        emit_setup_progress(&app, &format!("Using {}", interpreter), 30);
    }
    emit_setup_progress(&app, "Python ready", 30);

    // Step 2: Set up Python venv with whisperX
    emit_setup_progress(&app, "Installing whisperX and dependencies (this may take a few minutes)...", 40);
    python::setup_venv(&app).await?;
    emit_setup_progress(&app, "whisperX installed", 85);

    emit_setup_progress(&app, "Setup complete!", 100);
    Ok("Setup complete".to_string())
}

fn emit_setup_progress(app: &AppHandle, message: &str, percent: u8) {
    let _ = app.emit(
        "setup:progress",
        PipelineProgress {
            step: "setup".to_string(),
            percent,
            message: message.to_string(),
        },
    );
}
