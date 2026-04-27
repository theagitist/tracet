use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::process::Command;

use crate::models::transcript::PipelineProgress;

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
pub async fn check_setup_status() -> Result<SetupStatus, String> {
    let ffmpeg_installed = is_command_available("ffmpeg").await;
    let python_available = is_command_available("python3").await;
    let venv_ready = find_sidecar_dir()
        .map(|d| d.join(".venv").join("bin").join("python3").exists())
        .unwrap_or(false);
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
    if !is_command_available("python3").await {
        return Err(
            "Python 3 is not installed. Please install it from python.org or via Homebrew: brew install python3"
                .to_string(),
        );
    }
    emit_setup_progress(&app, "Python ready", 30);

    // Step 3: Set up Python venv with whisperX
    let sidecar_dir = find_sidecar_dir()?;
    let venv_path = sidecar_dir.join(".venv");

    if !venv_path.join("bin").join("python3").exists() {
        emit_setup_progress(&app, "Creating Python virtual environment...", 35);
        run_command("python3", &["-m", "venv", venv_path.to_str().unwrap()]).await?;
    }

    emit_setup_progress(&app, "Installing whisperX and dependencies (this may take a few minutes)...", 40);
    let pip = venv_path.join("bin").join("pip");
    let requirements = sidecar_dir.join("requirements.txt");
    run_command(
        pip.to_str().unwrap(),
        &["install", "-r", requirements.to_str().unwrap()],
    )
    .await?;
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

fn find_sidecar_dir() -> Result<PathBuf, String> {
    // Development: relative to Cargo manifest
    let dev_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("sidecar");
    if dev_path.exists() {
        return Ok(dev_path);
    }

    // Production: in the app's data directory
    let data_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.tracet.app")
        .join("sidecar");
    if data_path.exists() {
        return Ok(data_path);
    }

    // Create it in production
    std::fs::create_dir_all(&data_path).map_err(|e| format!("Failed to create sidecar dir: {}", e))?;

    // Copy requirements.txt to the production sidecar dir
    // In a bundled app, this would come from resources
    Err("Sidecar directory not configured. Please reinstall the app.".to_string())
}
