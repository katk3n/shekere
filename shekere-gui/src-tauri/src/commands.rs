use crate::file_tree::{get_file_tree, FileTree};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::command;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Preview error: {0}")]
    Preview(String),
}

impl Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

type CommandResult<T> = std::result::Result<T, CommandError>;

#[command]
pub async fn get_directory_tree(path: String) -> CommandResult<FileTree> {
    let path = Path::new(&path);

    if !path.exists() {
        return Err(CommandError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Directory '{}' does not exist", path.display()),
        )));
    }

    if !path.is_dir() {
        return Err(CommandError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Path '{}' is not a directory", path.display()),
        )));
    }

    get_file_tree(path).map_err(CommandError::Io)
}

#[command]
pub async fn load_toml_config(path: String) -> CommandResult<shekere_core::Config> {
    let path = Path::new(&path);

    if !path.exists() {
        return Err(CommandError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Configuration file '{}' does not exist", path.display()),
        )));
    }

    let content = std::fs::read_to_string(path)?;
    let config: shekere_core::Config = toml::from_str(&content)?;

    Ok(config)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreviewHandle {
    pub id: String,
    pub status: String,
}

#[command]
pub async fn start_preview(_config: shekere_core::Config) -> CommandResult<PreviewHandle> {
    // TODO: Implement actual preview start logic
    // For now, return a placeholder handle
    Ok(PreviewHandle {
        id: "preview_001".to_string(),
        status: "starting".to_string(),
    })
}

#[command]
pub async fn stop_preview() -> CommandResult<()> {
    // TODO: Implement actual preview stop logic
    // For now, just return success
    Ok(())
}
