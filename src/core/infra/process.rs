use crate::app_errors::AppError;

pub async fn run_script(script: Vec<&str>, dir: &str) -> Result<(), AppError> {
    for cmd in script {
        tokio::process::Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .current_dir(dir)
            .output()
            .await
            .map_err(|e| AppError::StartingFirecrackerFailed(e.to_string()))?;
    }

    Ok(())
}

pub fn run_script_bg(script: Vec<&str>, dir: &str) -> Result<(), AppError> {
    for cmd in script {
        std::process::Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .current_dir(dir)
            .spawn()
            .map_err(|e| AppError::StartingFirecrackerFailed(e.to_string()))?;
    }

    Ok(())
}
