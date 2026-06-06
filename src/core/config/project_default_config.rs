use crate::core::app_types::ProjectType;

pub struct ProjectDefaultConfig {
    pub install_commands: Vec<&'static str>,
    pub build_commands: Vec<&'static str>,
    pub run_command: Option<Vec<&'static str>>,
    pub deploy_artifacts: Vec<&'static str>,
    pub dist_dir: &'static str,
    pub root_dir: &'static str,
}

pub fn node_default_config() -> ProjectDefaultConfig {
    ProjectDefaultConfig {
        install_commands: vec!["npm install"],
        build_commands: vec!["npx tsc"],
        run_command: Some(vec!["node", "index.js"]),
        deploy_artifacts: vec!["dist", "package.json", "package-lock.json"],
        dist_dir: "dist",
        root_dir: ".",
    }
}

pub fn react_default_config() -> ProjectDefaultConfig {
    ProjectDefaultConfig {
        install_commands: vec!["npm install"],
        build_commands: vec!["npm run build"],
        run_command: None,
        deploy_artifacts: vec!["dist"],
        dist_dir: "dist",
        root_dir: ".",
    }
}

pub fn html_default_config() -> ProjectDefaultConfig {
    ProjectDefaultConfig {
        install_commands: vec![],
        build_commands: vec![],
        run_command: Some(vec!["npx", "serve", "."]),
        deploy_artifacts: vec!["."],
        dist_dir: ".",
        root_dir: ".",
    }
}

pub fn get_default_config(project_type: &ProjectType) -> ProjectDefaultConfig {
    match project_type {
        ProjectType::Node => node_default_config(),
        ProjectType::React => react_default_config(),
        ProjectType::Html => html_default_config(),
        ProjectType::Unknown => panic!("Unknown project type"),
        _ => panic!("Unknown project type"),
    }
}
