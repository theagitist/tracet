use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::process::Command;

use crate::models::transcript::PipelineProgress;
use crate::sidecar::python;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStatus {
    pub ffmpeg_installed: bool,
    pub python_available: bool,
    pub venv_ready: bool,
    pub ollama_installed: bool,
    pub ollama_running: bool,
    pub needs_setup: bool,
}

/// Check what's installed and what needs setup.
#[tauri::command]
pub async fn check_setup_status(app: AppHandle) -> Result<SetupStatus, String> {
    let ffmpeg_installed = is_command_available("ffmpeg").await;
    let venv_ready = python::is_runtime_ready(&app);
    // Once the venv exists the system interpreter no longer matters.
    let python_available = venv_ready || python::find_python_interpreter().await.is_ok();
    let ollama_installed = is_command_available("ollama").await;
    let ollama_running = reqwest::Client::new()
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .is_ok();

    let needs_setup = !ffmpeg_installed || !python_available || !venv_ready;

    Ok(SetupStatus {
        ffmpeg_installed,
        python_available,
        venv_ready,
        ollama_installed,
        ollama_running,
        needs_setup,
    })
}

/// Run the full first-time setup.
#[tauri::command]
pub async fn run_setup(app: AppHandle) -> Result<String, String> {
    // Step 1: Check/install FFmpeg
    emit_setup_progress(&app, "Checking FFmpeg...", 0);
    if !is_command_available("ffmpeg").await {
        emit_setup_progress(&app, "Installing FFmpeg via Homebrew...", 5);
        install_via_brew("ffmpeg").await?;
    }
    emit_setup_progress(&app, "FFmpeg ready", 20);

    // Step 2: Check Python
    emit_setup_progress(&app, "Checking Python...", 25);
    if !python::is_runtime_ready(&app) {
        let interpreter = python::find_python_interpreter().await?;
        emit_setup_progress(&app, &format!("Using {}", interpreter), 30);
    }
    emit_setup_progress(&app, "Python ready", 30);

    // Step 3: Set up Python venv with whisperX
    emit_setup_progress(&app, "Installing whisperX and dependencies (this may take a few minutes)...", 40);
    python::setup_venv(&app).await?;
    emit_setup_progress(&app, "whisperX installed", 85);

    // Step 4: Check Ollama (optional: just inform, don't block)
    emit_setup_progress(&app, "Checking Ollama...", 90);
    let ollama_installed = is_command_available("ollama").await;
    if !ollama_installed {
        emit_setup_progress(&app, "Ollama not found: accuracy review will use confidence scores only. Install from ollama.ai for LLM review.", 95);
    } else {
        emit_setup_progress(&app, "Ollama found", 95);
    }

    emit_setup_progress(&app, "Setup complete!", 100);
    Ok("Setup complete".to_string())
}

/// Install Ollama via the official install script.
#[tauri::command]
pub async fn install_ollama() -> Result<String, String> {
    // Check if brew is available and use it
    if is_command_available("brew").await {
        run_command("brew", &["install", "ollama"]).await?;
        return Ok("Ollama installed via Homebrew".to_string());
    }
    Err("Please install Ollama manually from https://ollama.ai".to_string())
}

async fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}


async fn install_via_brew(package: &str) -> Result<(), String> {
    if !is_command_available("brew").await {
        return Err(format!(
            "{} is not installed and Homebrew is not available. Please install {} manually.",
            package, package
        ));
    }

    let output = Command::new("brew")
        .args(["install", package])
        .output()
        .await
        .map_err(|e| format!("Failed to run brew install: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("brew install {} failed: {}", package, stderr));
    }

    Ok(())
}

async fn run_command(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("Failed to run {}: {}", cmd, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{} failed: {}", cmd, stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
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
