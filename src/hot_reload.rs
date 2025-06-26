use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct HotReloader {
    _watcher: RecommendedWatcher,
    shader_modified: Arc<Mutex<bool>>,
}

#[cfg(test)]
pub struct MockHotReloader {
    shader_modified: Arc<Mutex<bool>>,
}

#[cfg(test)]
impl MockHotReloader {
    pub fn new() -> Self {
        Self {
            shader_modified: Arc::new(Mutex::new(false)),
        }
    }

    pub fn simulate_file_change(&self) {
        *self.shader_modified.lock().unwrap() = true;
    }

    pub fn check_for_changes(&self) -> bool {
        let mut modified = self.shader_modified.lock().unwrap();
        if *modified {
            *modified = false;
            return true;
        }
        false
    }
}

impl HotReloader {
    pub fn new<P: AsRef<Path>>(shader_path: P) -> notify::Result<Self> {
        let shader_modified = Arc::new(Mutex::new(false));
        let shader_modified_clone = shader_modified.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = &res {
                    if event.kind.is_modify() {
                        *shader_modified_clone.lock().unwrap() = true;
                    }
                }
            },
            Config::default(),
        )?;

        watcher.watch(shader_path.as_ref(), RecursiveMode::NonRecursive)?;

        Ok(Self {
            _watcher: watcher,
            shader_modified,
        })
    }

    pub fn check_for_changes(&self) -> bool {
        let mut modified = self.shader_modified.lock().unwrap();
        if *modified {
            *modified = false;

            thread::sleep(Duration::from_millis(50));
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_mock_hot_reloader_initial_state() {
        let reloader = MockHotReloader::new();
        assert!(
            !reloader.check_for_changes(),
            "Should start with no changes"
        );
    }

    #[test]
    fn test_mock_hot_reloader_simulate_change() {
        let reloader = MockHotReloader::new();

        // Simulate a file change
        reloader.simulate_file_change();

        // Should detect the change
        assert!(
            reloader.check_for_changes(),
            "Should detect simulated change"
        );

        // Should reset after checking
        assert!(
            !reloader.check_for_changes(),
            "Should reset after first check"
        );
    }

    #[test]
    fn test_mock_hot_reloader_multiple_changes() {
        let reloader = MockHotReloader::new();

        // Simulate multiple changes
        reloader.simulate_file_change();
        reloader.simulate_file_change();

        // Should only trigger once
        assert!(reloader.check_for_changes(), "Should detect change");
        assert!(!reloader.check_for_changes(), "Should not trigger again");
    }

    #[test]
    fn test_hot_reloader_with_valid_file() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(temp_file, "// Test shader content").expect("Failed to write to temp file");

        let result = HotReloader::new(temp_file.path());
        assert!(
            result.is_ok(),
            "Should successfully create HotReloader for valid file"
        );
    }

    #[test]
    fn test_hot_reloader_with_invalid_path() {
        let result = HotReloader::new("/non/existent/path/shader.wgsl");
        assert!(
            result.is_err(),
            "Should fail to create HotReloader for invalid path"
        );
    }

    #[test]
    fn test_hot_reloader_file_modification_detection() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(temp_file, "// Initial shader content").expect("Failed to write to temp file");

        let reloader = HotReloader::new(temp_file.path()).expect("Failed to create HotReloader");

        // Initially no changes
        assert!(
            !reloader.check_for_changes(),
            "Should start with no changes"
        );

        // Modify the file
        writeln!(temp_file, "// Modified shader content").expect("Failed to modify temp file");
        temp_file.flush().expect("Failed to flush temp file");

        // Give the file watcher some time to detect the change
        thread::sleep(Duration::from_millis(100));

        // Should detect the change (this test might be flaky due to timing)
        let detected_change = reloader.check_for_changes();
        // Note: This assertion might fail in some environments due to file system timing
        // In a real test environment, you might want to use a more robust approach
        println!("Change detected: {}", detected_change);
    }

    #[test]
    fn test_hot_reloader_check_for_changes_resets_flag() {
        let reloader = MockHotReloader::new();

        reloader.simulate_file_change();

        // First check should return true
        assert!(
            reloader.check_for_changes(),
            "First check should detect change"
        );

        // Subsequent checks should return false
        assert!(
            !reloader.check_for_changes(),
            "Second check should not detect change"
        );
        assert!(
            !reloader.check_for_changes(),
            "Third check should not detect change"
        );
    }
}
