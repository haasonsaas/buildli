use anyhow::{Context, Result};
use ignore::WalkBuilder;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;
use tracing::debug;

#[derive(Debug, Clone)]
pub enum WatchEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}

pub struct FileWalker {
    ignore_patterns: Vec<String>,
}

impl FileWalker {
    pub fn new() -> Self {
        Self {
            ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "build".to_string(),
                "__pycache__".to_string(),
                ".pytest_cache".to_string(),
                ".mypy_cache".to_string(),
            ],
        }
    }

    pub fn with_ignore_patterns(mut self, patterns: Vec<String>) -> Self {
        self.ignore_patterns.extend(patterns);
        self
    }

    pub fn walk(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        let walker = WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();
        
        for entry in walker {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            
            if path.is_file() && !self.should_ignore(path) {
                files.push(path.to_path_buf());
            }
        }
        
        Ok(files)
    }

    pub fn watch(&self, root: &Path) -> Result<Receiver<WatchEvent>> {
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                let watch_event = match event.kind {
                    EventKind::Create(_) => {
                        event.paths.first().map(|p| WatchEvent::Created(p.clone()))
                    }
                    EventKind::Modify(_) => {
                        event.paths.first().map(|p| WatchEvent::Modified(p.clone()))
                    }
                    EventKind::Remove(_) => {
                        event.paths.first().map(|p| WatchEvent::Deleted(p.clone()))
                    }
                    _ => None,
                };
                
                if let Some(watch_event) = watch_event {
                    let _ = tx.send(watch_event);
                }
            }
        })?;
        
        watcher.watch(root, RecursiveMode::Recursive)?;
        
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(60));
                debug!("File watcher still active");
            }
        });
        
        Ok(rx)
    }

    fn should_ignore(&self, path: &Path) -> bool {
        for pattern in &self.ignore_patterns {
            if path.to_string_lossy().contains(pattern) {
                return true;
            }
        }
        
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy();
            if matches!(ext.as_ref(), "exe" | "dll" | "so" | "dylib" | "o" | "a") {
                return true;
            }
        }
        
        false
    }
}