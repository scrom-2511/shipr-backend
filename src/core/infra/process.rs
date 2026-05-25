use crate::app_errors::AppError;
use std::process::Command;

pub fn run_script(script: Vec<&str>, dir: &str) -> Result<(), AppError> {
    for cmd in script {
        Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .current_dir(dir)
            .output()
            .map_err(|e| AppError::StartingFirecrackerFailed(e.to_string()))?;
    }

    Ok(())
}

pub fn run_script_bg(script: Vec<&str>, dir: &str) -> Result<(), AppError> {
    for cmd in script {
        Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .current_dir(dir)
            .spawn()
            .map_err(|e| AppError::StartingFirecrackerFailed(e.to_string()))?;
    }

    Ok(())
}
