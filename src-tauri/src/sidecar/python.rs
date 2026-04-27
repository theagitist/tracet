use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Check if the Python virtual environment is set up.
pub fn is_venv_ready(sidecar_dir: &Path) -> bool {
    sidecar_dir
        .join(".venv")
        .join("bin")
        .join("python3")
        .exists()
}

/// Set up the Python virtual environment and install dependencies.
pub async fn setup_venv(sidecar_dir: &Path) -> Result<String, String> {
    let venv_path = sidecar_dir.join(".venv");
    let requirements = sidecar_dir.join("requirements.txt");

    if !requirements.exists() {
        return Err(format!(
            "requirements.txt not found at {}",
            requirements.display()
        ));
    }

    // Create venv
    let output = Command::new("python3")
        .args(["-m", "venv", venv_path.to_str().unwrap()])
        .output()
        .await
        .map_err(|e| format!("Failed to create venv: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to create venv: {}", stderr));
    }

    // Install dependencies
    let pip = venv_path.join("bin").join("pip");
    let output = Command::new(pip.to_str().unwrap())
        .args([
            "install",
            "-r",
            requirements.to_str().unwrap(),
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to install dependencies: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to install dependencies: {}", stderr));
    }

    Ok("Python environment set up successfully".to_string())
}

/// Get the path to the Python binary in the venv.
pub fn python_bin(sidecar_dir: &Path) -> PathBuf {
    sidecar_dir.join(".venv").join("bin").join("python3")
}

/// Tauri command to set up the Python sidecar from the frontend.
#[tauri::command]
pub async fn setup_python_sidecar() -> Result<String, String> {
    let sidecar_dir = find_sidecar_dir()?;
    setup_venv(&sidecar_dir).await
}

/// Tauri command to check if the Python sidecar is ready.
#[tauri::command]
pub fn check_python_sidecar() -> Result<bool, String> {
    let sidecar_dir = find_sidecar_dir()?;
    Ok(is_venv_ready(&sidecar_dir))
}

fn find_sidecar_dir() -> Result<PathBuf, String> {
    let dev_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("sidecar");
    if dev_path.exists() {
        return Ok(dev_path);
    }
    Err("Could not locate sidecar directory".to_string())
}
