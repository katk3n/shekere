use std::path::Path;

const EMBEDDED_SHADER_DEFS: &str = include_str!("../shaders/common.wgsl");

#[derive(Debug, thiserror::Error)]
pub enum PreprocessorError {
    #[error("Failed to read file: {file_path} - {source}")]
    FileRead {
        file_path: String,
        source: std::io::Error,
    },
}

pub struct ShaderPreprocessor;

impl ShaderPreprocessor {
    pub fn new(_base_dir: &Path) -> Self {
        Self
    }

    pub fn process_file_with_embedded_defs(
        &self,
        file_path: &Path,
    ) -> Result<String, PreprocessorError> {
        // Read user shader file
        let user_content =
            std::fs::read_to_string(file_path).map_err(|e| PreprocessorError::FileRead {
                file_path: file_path.to_string_lossy().to_string(),
                source: e,
            })?;

        // Prepend embedded definitions to user shader
        let mut final_content = String::new();
        final_content.push_str(EMBEDDED_SHADER_DEFS);
        final_content.push_str("\n\n// === USER SHADER CODE ===\n\n");
        final_content.push_str(&user_content);

        Ok(final_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let file_path = dir.join(name);
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file_path
    }

    #[test]
    fn test_process_file_with_embedded_defs() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        create_test_file(
            dir_path,
            "user.wgsl",
            "@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n    return vec4(1.0);\n}",
        );

        let preprocessor = ShaderPreprocessor::new(dir_path);
        let result = preprocessor
            .process_file_with_embedded_defs(&dir_path.join("user.wgsl"))
            .unwrap();

        // Should contain embedded definitions
        assert!(result.contains("struct WindowUniform"));
        assert!(result.contains("struct TimeUniform"));
        assert!(result.contains("fn NormalizedCoords"));
        assert!(result.contains("fn ToLinearRgb"));

        // Should contain user code
        assert!(result.contains("@fragment"));
        assert!(result.contains("fs_main"));

        // Should have the structure with embedded defs first, then user code
        let embedded_pos = result.find("struct WindowUniform").unwrap();
        let user_pos = result.find("@fragment").unwrap();
        assert!(
            embedded_pos < user_pos,
            "Embedded definitions should come before user code"
        );
    }

    #[test]
    fn test_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        let preprocessor = ShaderPreprocessor::new(dir_path);
        let result =
            preprocessor.process_file_with_embedded_defs(&dir_path.join("nonexistent.wgsl"));

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PreprocessorError::FileRead { .. }
        ));
    }

    #[test]
    fn test_embedded_defs_content() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        create_test_file(dir_path, "simple.wgsl", "// Simple shader");

        let preprocessor = ShaderPreprocessor::new(dir_path);
        let result = preprocessor
            .process_file_with_embedded_defs(&dir_path.join("simple.wgsl"))
            .unwrap();

        // Check that all expected embedded definitions are present
        assert!(result.contains("struct WindowUniform"));
        assert!(result.contains("struct TimeUniform"));
        assert!(result.contains("struct MouseUniform"));
        assert!(result.contains("struct VertexOutput"));
        assert!(result.contains("@group(0) @binding(0) var<uniform> Window"));
        assert!(result.contains("@group(0) @binding(1) var<uniform> Time"));
        assert!(result.contains("@group(1) @binding(0) var<uniform> Mouse"));
        assert!(result.contains("fn ToLinearRgb"));
        assert!(result.contains("fn NormalizedCoords"));
    }
}
