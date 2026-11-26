use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tokio::process::Command;

/// Directory holding `diarize.py` and `requirements.txt`.
///
/// Debug builds use the repo's `sidecar/` so edits apply without a rebuild.
/// Release builds must never do that: `CARGO_MANIFEST_DIR` is a path on the
/// build machine, so a release that read it only worked on that one Mac.
pub fn script_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or("Invalid manifest dir")?
            .join("sidecar")
    } else {
        app.path()
            .resource_dir()
            .map_err(|e| format!("Could not locate app resources: {}", e))?
            .join("sidecar")
    };
    if !dir.join("diarize.py").exists() {
        return Err(format!(
            "Sidecar scripts not found at {}. Please reinstall the app.",
            dir.display()
        ));
    }
    Ok(dir)
}

/// The dev venv (`sidecar/.venv`). Release builds don't have one: they ship
/// a self-contained runtime instead (scripts/build-runtime.sh).
fn venv_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(script_dir(app)?.join(".venv"))
}

/// The interpreter that runs diarize.py.
pub fn python_executable(app: &AppHandle) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        return Ok(venv_dir(app)?.join("bin").join("python3"));
    }
    Ok(app
        .path()
        .resource_dir()
        .map_err(|e| format!("Could not locate app resources: {}", e))?
        .join("python")
        .join("bin")
        .join("python3.12"))
}

pub fn is_runtime_ready(app: &AppHandle) -> bool {
    python_executable(app).map(|p| p.exists()).unwrap_or(false)
}

/// The bundled pyannote diarization pipeline (scripts/build-runtime.sh).
/// Debug builds use the copy in the runtime build dir when it exists.
fn diarization_model_dir(app: &AppHandle) -> Option<PathBuf> {
    let models = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/runtime/models")
    } else {
        app.path().resource_dir().ok()?.join("models")
    };
    let dir = models.join("pyannote-speaker-diarization-community-1");
    dir.join("config.yaml").exists().then_some(dir)
}

/// Environment for the sidecar. The bundled runtime must not pick up the
/// user's own Python setup, and must not try to write `__pycache__` into the
/// signed, read-only bundle.
pub fn sidecar_env(app: &AppHandle, cmd: &mut Command) {
    cmd.env_remove("PYTHONPATH")
        .env_remove("PYTHONHOME")
        .env("PYTHONNOUSERSITE", "1")
        .env("PYTHONDONTWRITEBYTECODE", "1");
    if let Some(dir) = diarization_model_dir(app) {
        cmd.env("TRACET_DIARIZATION_MODEL", dir);
    }
}

/// Find an interpreter the ML stack supports. whisperX needs 3.10 to 3.13
/// (3.14 breaks it; Apple's /usr/bin/python3 is 3.9), so a bare `python3`
/// is only used when its version is in range.
pub async fn find_python_interpreter() -> Result<String, String> {
    for candidate in ["python3.12", "python3.13", "python3.11", "python3.10", "python3"] {
        let Ok(output) = Command::new(candidate)
            .args(["-c", "import sys; print(sys.version_info[0], sys.version_info[1])"])
            .output()
            .await
        else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let version = String::from_utf8_lossy(&output.stdout);
        let mut parts = version.split_whitespace().map(|s| s.parse::<u32>());
        if let (Some(Ok(3)), Some(Ok(minor))) = (parts.next(), parts.next()) {
            if (10..=13).contains(&minor) {
                return Ok(candidate.to_string());
            }
        }
    }
    Err("No supported Python found. Tracet needs Python 3.10 to 3.13 (3.12 recommended): brew install python@3.12".to_string())
}

/// Dev only: create the venv (if missing) and install the sidecar's
/// dependencies. Release builds already carry everything.
pub async fn setup_venv(app: &AppHandle) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return if is_runtime_ready(app) {
            Ok("Bundled Python runtime is ready".to_string())
        } else {
            Err("The bundled Python runtime is missing. Please reinstall Tracet.".to_string())
        };
    }
    let requirements = script_dir(app)?.join("requirements.txt");
    let venv_path = venv_dir(app)?;

    if !venv_path.join("bin").join("python3").exists() {
        let interpreter = find_python_interpreter().await?;
        if let Some(parent) = venv_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
        }
        run(&interpreter, &["-m", "venv", &venv_path.to_string_lossy()]).await?;
    }

    let pip = venv_path.join("bin").join("pip");
    run(
        &pip.to_string_lossy(),
        &["install", "-r", &requirements.to_string_lossy()],
    )
    .await?;

    Ok("Python environment set up successfully".to_string())
}

async fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("Failed to run {}: {}", cmd, e))?;
    if !output.status.success() {
        return Err(format!(
            "{} failed: {}",
            cmd,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// Tauri command to set up the Python sidecar from the frontend.
#[tauri::command]
pub async fn setup_python_sidecar(app: AppHandle) -> Result<String, String> {
    setup_venv(&app).await
}

/// Tauri command to check if the Python sidecar is ready.
#[tauri::command]
pub fn check_python_sidecar(app: AppHandle) -> Result<bool, String> {
    Ok(is_runtime_ready(&app))
}
