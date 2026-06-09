use crate::core::app_types::ProjectType;

#[derive(Debug)]
pub struct ProjectDefaultConfig {
    pub install_commands: Vec<&'static str>,
    pub build_commands: Vec<&'static str>,
    pub run_command: Option<Vec<&'static str>>,
    pub deploy_artifacts: Vec<&'static str>,
    pub root_dir: &'static str,
}

pub fn node_default_config() -> ProjectDefaultConfig {
    ProjectDefaultConfig {
        install_commands: vec!["npm install"],
        build_commands: vec!["npx tsc"],
        run_command: Some(vec!["node dist/index.js"]),
        deploy_artifacts: vec!["dist", "package.json", "package-lock.json"],
        root_dir: ".",
    }
}

pub fn react_default_config() -> ProjectDefaultConfig {
    ProjectDefaultConfig {
        install_commands: vec!["npm install"],
        build_commands: vec!["npm run build"],
        run_command: Some(vec!["npx serve dist"]),
        deploy_artifacts: vec!["dist", "package.json", "package-lock.json"],
        root_dir: ".",
    }
}

pub fn html_default_config() -> ProjectDefaultConfig {
    ProjectDefaultConfig {
        install_commands: vec![],
        build_commands: vec![],
        run_command: Some(vec!["npx serve dist"]),
        deploy_artifacts: vec!["."],
        root_dir: ".",
    }
}

pub fn rust_default_config() -> ProjectDefaultConfig {
    ProjectDefaultConfig {
        install_commands: vec!["echo 'no install commands'"],
        build_commands: vec!["cargo build"],
        run_command: Some(vec![
            "export PROJECT_NAME=$(awk -F'\"' '/^name =/ {print $2}' Cargo.toml)",
            "./target/release/$PROJECT_NAME",
        ]),
        deploy_artifacts: vec!["target/release", "Cargo.toml", "Cargo.lock"],
        root_dir: ".",
    }
}

pub fn get_default_config(project_type: &ProjectType) -> ProjectDefaultConfig {
    match project_type {
        ProjectType::Node => node_default_config(),
        ProjectType::React => react_default_config(),
        ProjectType::Html => html_default_config(),
        ProjectType::Rust => rust_default_config(),
        ProjectType::Next => next_default_config(),
        ProjectType::Unknown => panic!("Unknown project type"),
    }
}
