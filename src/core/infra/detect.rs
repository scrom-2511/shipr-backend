use std::{fs, path::Path};

use crate::app::models::ProjectType;

pub fn detect_project_type(path: &str) -> ProjectType {
    println!("path is: {:?}", path);
    let html_path = format!("{}/index.html", path);
    let package_json_path = format!("{}/package.json", path);
    let cargo_toml_path = format!("{}/Cargo.toml", path);

    if Path::new(&package_json_path).exists() {
        let package_json_str = fs::read_to_string(&package_json_path).unwrap();
        let package_json = serde_json::from_str::<serde_json::Value>(&package_json_str).unwrap();

        if package_json["dependencies"].get("react").is_some() {
            return ProjectType::React;
        }

        if package_json["dependencies"].get("express").is_some() {
            return ProjectType::Node;
        }
    }

    if Path::new(&cargo_toml_path).exists() {
        return ProjectType::Rust;
    }

    if Path::new(&html_path).exists() {
        return ProjectType::Html;
    }

    ProjectType::Unknown
}
