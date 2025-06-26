use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct HotReloader {
    _watcher: RecommendedWatcher,
    shader_modified: Arc<Mutex<bool>>,
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