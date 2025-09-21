use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub file_type: Option<String>,
    pub children: Option<Vec<FileNode>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileTree {
    pub root: FileNode,
    pub total_files: usize,
    pub total_directories: usize,
}

pub fn get_file_tree(root_path: &Path) -> Result<FileTree, std::io::Error> {
    let mut total_files = 0;
    let mut total_directories = 0;

    let root_node = build_file_node(root_path, &mut total_files, &mut total_directories)?;

    Ok(FileTree {
        root: root_node,
        total_files,
        total_directories,
    })
}

fn build_file_node(
    path: &Path,
    total_files: &mut usize,
    total_directories: &mut usize,
) -> Result<FileNode, std::io::Error> {
    let metadata = fs::metadata(path)?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let path_str = path.to_string_lossy().to_string();

    if metadata.is_dir() {
        *total_directories += 1;
        let mut children = Vec::new();

        // Read directory contents
        let entries = fs::read_dir(path)?;
        for entry in entries {
            let entry = entry?;
            let entry_path = entry.path();

            // Skip hidden files and directories (starting with '.')
            if let Some(file_name) = entry_path.file_name() {
                if let Some(name_str) = file_name.to_str() {
                    if name_str.starts_with('.') {
                        continue;
                    }
                }
            }

            // Recursively build child nodes
            match build_file_node(&entry_path, total_files, total_directories) {
                Ok(child_node) => children.push(child_node),
                Err(_) => continue, // Skip files we can't read
            }
        }

        // Sort children: directories first, then files, all alphabetically
        children.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        Ok(FileNode {
            name,
            path: path_str,
            is_directory: true,
            file_type: None,
            children: Some(children),
        })
    } else {
        *total_files += 1;
        let file_type = get_file_type(path);

        Ok(FileNode {
            name,
            path: path_str,
            is_directory: false,
            file_type,
            children: None,
        })
    }
}

fn get_file_type(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext.to_lowercase().as_str() {
            "toml" => "config",
            "wgsl" => "shader",
            "glsl" => "shader",
            "frag" => "shader",
            "vert" => "shader",
            "rs" => "rust",
            "js" => "javascript",
            "ts" => "typescript",
            "json" => "json",
            "md" => "markdown",
            "txt" => "text",
            _ => "unknown",
        })
        .map(|s| s.to_string())
}
