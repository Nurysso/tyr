use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub fn is_hidden(path: &Path) -> bool {
    #[cfg(unix)]
    {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("."))
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        fs::metadata(path)
            .map(|meta| meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

pub struct LazyDirManager {
    created_dirs: Arc<Mutex<HashMap<PathBuf, bool>>>,
}

impl LazyDirManager {
    pub fn new() -> Self {
        Self {
            created_dirs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Only creates directory if not already created in this session
    pub fn ensure_dir_exists(&self, path: &Path) -> io::Result<()> {
        let mut cache = self.created_dirs.lock().unwrap();

        if cache.contains_key(path) {
            return Ok(());
        }

        if !path.exists() {
            fs::create_dir_all(path)?;
        }

        cache.insert(path.to_path_buf(), true);
        Ok(())
    }
    /// True if the paths has already been created/verified in this session
    pub fn is_known(&self, path: &Path) -> bool {
        self.created_dirs.lock().unwrap().contains_key(path)
    }
}

pub struct SafeLogger {
    log_entries: Arc<Mutex<Vec<LogEntry>>>,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    #[allow(dead_code)]
    pub details: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Info,
    Success,
    #[allow(dead_code)]
    Warning,
    Error,
}

impl SafeLogger {
    pub fn new() -> Self {
        Self {
            log_entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn log(&self, level: LogLevel, message: String, details: Option<String>) {
        if let Ok(mut entries) = self.log_entries.lock() {
            entries.push(LogEntry {
                level,
                message,
                details,
            });
        }
    }

    pub fn get_logs(&self) -> Vec<LogEntry> {
        self.log_entries
            .lock()
            .map(|e| e.clone())
            .unwrap_or_default()
    }
}
