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

    let root_node = build_file_node(root_path, &mut total_files, &mut total_directories, 0)?;

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
    depth: usize,
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

        // Limit depth to prevent excessive recursion in large projects
        const MAX_DEPTH: usize = 10;
        if depth >= MAX_DEPTH {
            return Ok(FileNode {
                name,
                path: path_str,
                is_directory: true,
                file_type: None,
                children: Some(Vec::new()), // Empty children to indicate depth limit
            });
        }

        // Read directory contents
        let entries = fs::read_dir(path)?;
        for entry in entries {
            let entry = entry?;
            let entry_path = entry.path();

            // Enhanced filtering for shekere projects
            if should_skip_entry(&entry_path) {
                continue;
            }

            // Recursively build child nodes
            match build_file_node(&entry_path, total_files, total_directories, depth + 1) {
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

/// Enhanced filtering for shekere projects - skip common build/cache directories and hidden files
fn should_skip_entry(path: &Path) -> bool {
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        // Skip hidden files and directories (starting with '.')
        if file_name.starts_with('.') {
            return true;
        }

        // Skip common build and cache directories
        match file_name {
            // Rust build artifacts
            "target" => true,
            // Node.js dependencies
            "node_modules" => true,
            // General cache/temp directories
            "cache" | "tmp" | "temp" => true,
            // IDE directories
            ".vscode" | ".idea" | ".vs" => true,
            // OS-specific directories
            "__pycache__" | ".DS_Store" | "Thumbs.db" => true,
            // Other build artifacts
            "build" | "dist" | "out" => true,
            _ => false,
        }
    } else {
        false
    }
}

fn get_file_type(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext.to_lowercase().as_str() {
            // Configuration files
            "toml" => "config",
            "yaml" | "yml" => "config",
            "json" => "config",

            // Shader files
            "wgsl" => "shader",
            "glsl" => "shader",
            "frag" => "shader",
            "vert" => "shader",
            "hlsl" => "shader",
            "spv" => "shader",

            // Programming languages
            "rs" => "rust",
            "js" => "javascript",
            "ts" => "typescript",
            "py" => "python",
            "c" | "cpp" | "cc" | "cxx" => "cpp",
            "h" | "hpp" => "header",

            // Documentation
            "md" => "markdown",
            "txt" => "text",
            "rst" => "text",

            // Other common files
            "png" | "jpg" | "jpeg" | "gif" | "bmp" => "image",
            "wav" | "mp3" | "ogg" | "flac" => "audio",
            "mp4" | "avi" | "mov" | "mkv" => "video",

            _ => "unknown",
        })
        .map(|s| s.to_string())
}
