//  Copyright (C) 2026 Dawood Khan
//  SPDX-License-Identifier: GPL-3.0-or-later

// Maintainer Dawood (Nurysso) contact - nurysso [at] proton.me

/*
 * This file is responsible for executing category(file extension) based file organizing for
 * tyr(https:github.com/Nurysso/tyr) project
*/

use crate::organizer::tui::{self, App, Flow, Job, Progress};
use ratatui::crossterm::event::KeyCode;
use ratatui::style::Color;
use ratatui::{layout::Constraint, layout::Rect, Frame};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use crate::organizer::utils::{is_hidden, LazyDirManager, LogLevel, SafeLogger};

#[derive(Debug, Deserialize, Serialize)]
pub struct FileOrganizerConfig {
    #[serde(default)]
    pub categories: HashMap<String, CategoryConfig>,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default)]
    pub skip_patterns: Vec<String>,
    #[serde(default = "default_true")]
    pub skip_hidden_files: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CategoryConfig {
    pub extensions: Vec<String>,
    #[serde(default)]
    pub folder_name: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_batch_size() -> usize {
    100
}

impl Default for FileOrganizerConfig {
    fn default() -> Self {
        Self {
            categories: create_default_categories(),
            batch_size: 100,
            skip_patterns: vec![
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
                ".git".to_string(),
                ".gitignore".to_string(),
                "desktop.ini".to_string(),
            ],
            skip_hidden_files: true,
        }
    }
}

fn create_default_categories() -> HashMap<String, CategoryConfig> {
    let mut map = HashMap::new();

    map.insert(
        "images".to_string(),
        CategoryConfig {
            extensions: vec![
                "jpg", "jpeg", "png", "gif", "bmp", "svg", "webp", "tiff", "ico", "heic", "raw",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            folder_name: Some("Images".to_string()),
        },
    );

    map.insert(
        "videos".to_string(),
        CategoryConfig {
            extensions: vec![
                "mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "m4v", "3gp", "mpg", "mpeg",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            folder_name: Some("Videos".to_string()),
        },
    );

    map.insert(
        "audio".to_string(),
        CategoryConfig {
            extensions: vec![
                "mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus", "aiff",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            folder_name: Some("Audio".to_string()),
        },
    );

    map.insert(
        "documents".to_string(),
        CategoryConfig {
            extensions: vec![
                "pdf", "doc", "docx", "txt", "rtf", "odt", "pages", "tex", "md", "epub",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            folder_name: Some("Documents".to_string()),
        },
    );

    map.insert(
        "code".to_string(),
        CategoryConfig {
            extensions: vec![
                "rs", "py", "js", "ts", "html", "css", "cpp", "c", "h", "java", "go",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            folder_name: Some("Code".to_string()),
        },
    );

    map
}

impl FileOrganizerConfig {
    pub fn load_from_file(path: &Path) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("TOML parse error: {}", e),
            )
        })
    }

    pub fn save_to_file(&self, path: &Path) -> io::Result<()> {
        let content = toml::to_string_pretty(self).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("TOML serialize error: {}", e),
            )
        })?;
        fs::write(path, content)
    }

    /// Build reverse lookup map: extension -> (category_key, folder_name)
    pub fn build_extension_map(&self) -> HashMap<String, (String, String)> {
        let mut ext_map = HashMap::new();

        for (category_key, config) in &self.categories {
            let folder_name = config
                .folder_name
                .as_ref()
                .cloned()
                .unwrap_or_else(|| category_key.clone());

            for ext in &config.extensions {
                ext_map.insert(
                    ext.to_lowercase(),
                    (category_key.clone(), folder_name.clone()),
                );
            }
        }

        ext_map
    }
}

// Fast File Organizer (with parallelization)

pub struct FileOrganizer {
    config: FileOrganizerConfig,
    dir_manager: LazyDirManager,
    logger: SafeLogger,
}

#[derive(Debug)]
pub struct OrganizeResult {
    pub files_organized: usize,
    pub files_skipped: usize,
    pub files_failed: usize,
    pub category_counts: HashMap<String, usize>,
}

impl FileOrganizer {
    pub fn new(config: FileOrganizerConfig) -> Self {
        Self {
            config,
            dir_manager: LazyDirManager::new(),
            logger: SafeLogger::new(),
        }
    }

    pub fn get_logger(&self) -> &SafeLogger {
        &self.logger
    }

    pub fn organize_directory(
        &self,
        base_path: &Path,
        dry_run: bool,
    ) -> io::Result<OrganizeResult> {
        let ext_map = self.config.build_extension_map();

        let entries: Vec<_> = fs::read_dir(base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .collect();

        let category_counts = Arc::new(Mutex::new(HashMap::new()));
        let files_organized = Arc::new(Mutex::new(0usize));
        let files_skipped = Arc::new(Mutex::new(0usize));
        let files_failed = Arc::new(Mutex::new(0usize));

        entries.par_iter().for_each(|entry| {
            let file_path = entry.path();
            let filename = match file_path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => {
                    *files_skipped.lock().unwrap() += 1;
                    return;
                }
            };

            if self.should_skip_file(filename) {
                self.logger
                    .log(LogLevel::Info, format!("Skipped: {}", filename), None);
                *files_skipped.lock().unwrap() += 1;
                return;
            }
            if self.config.skip_hidden_files && is_hidden(&file_path) {
                self.logger.log(
                    LogLevel::Info,
                    format!("Skipped (hidden): {}", filename),
                    None,
                );
                *files_skipped.lock().unwrap() += 1;
                return;
            }

            let extension = file_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_lowercase())
                .unwrap_or_else(|| "unknown".to_string());

            let (_category_key, folder_name) = ext_map
                .get(&extension)
                .cloned()
                .unwrap_or_else(|| ("extras".to_string(), "Extras".to_string()));

            let target_dir = base_path.join(&folder_name);
            let target_path = target_dir.join(filename);

            let final_target = match self.handle_naming_conflict(&target_path) {
                Ok(path) => path,
                Err(e) => {
                    self.logger.log(
                        LogLevel::Error,
                        format!("Naming conflict for: {}", filename),
                        Some(e.to_string()),
                    );
                    *files_failed.lock().unwrap() += 1;
                    return;
                }
            };

            if !dry_run {
                if let Err(e) = self.dir_manager.ensure_dir_exists(&target_dir) {
                    self.logger.log(
                        LogLevel::Error,
                        format!("Failed to create dir: {}", folder_name),
                        Some(e.to_string()),
                    );
                    *files_failed.lock().unwrap() += 1;
                    return;
                }

                if let Err(e) = fs::rename(&file_path, &final_target) {
                    self.logger.log(
                        LogLevel::Error,
                        format!("Failed to move: {}", filename),
                        Some(e.to_string()),
                    );
                    *files_failed.lock().unwrap() += 1;
                    return;
                }
            }

            self.logger.log(
                LogLevel::Success,
                format!("{} \u{2192} {}", filename, folder_name),
                None,
            );

            *files_organized.lock().unwrap() += 1;
            *category_counts
                .lock()
                .unwrap()
                .entry(folder_name)
                .or_insert(0) += 1;
        });

        let organized_count = *files_organized.lock().unwrap();
        let skipped_count = *files_skipped.lock().unwrap();
        let failed_count = *files_failed.lock().unwrap();
        let counts = category_counts.lock().unwrap().clone();

        Ok(OrganizeResult {
            files_organized: organized_count,
            files_skipped: skipped_count,
            files_failed: failed_count,
            category_counts: counts,
        })
    }

    fn should_skip_file(&self, filename: &str) -> bool {
        self.config
            .skip_patterns
            .iter()
            .any(|pattern| filename == pattern.as_str() || filename.starts_with(pattern.as_str()))
    }

    fn handle_naming_conflict(&self, target_path: &Path) -> io::Result<PathBuf> {
        if !target_path.exists() {
            return Ok(target_path.to_path_buf());
        }

        let parent = target_path.parent().unwrap();
        let stem = target_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let extension = target_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();

        for i in 1..1000 {
            let new_name = format!("{}_{}{}", stem, i, extension);
            let new_path = parent.join(new_name);
            if !new_path.exists() {
                return Ok(new_path);
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Could not find available filename after 999 attempts",
        ))
    }
}

// TUI Implementation

enum AppState {
    Ready,
    Organizing(Job<OrganizeResult>),
    Complete(OrganizeResult),
}

pub struct TuiApp {
    organizer: FileOrganizer,
    base_path: PathBuf,
    state: AppState,
    tick: usize,
    live_moved: usize,
    live_skipped: usize,
    live_failed: usize,
    total_files: usize,
    state_changed: bool,
}

impl TuiApp {
    pub fn new(config: FileOrganizerConfig, base_path: PathBuf) -> Self {
        Self {
            organizer: FileOrganizer::new(config),
            base_path,
            state: AppState::Ready,
            tick: 0,
            live_moved: 0,
            live_skipped: 0,
            live_failed: 0,
            total_files: 0,
            state_changed: true,
        }
    }

    /// Flattned log lines ("LEVEL: message"), for file logging.
    pub fn get_logs(&self) -> Vec<String> {
        self.organizer
            .get_logger()
            .get_logs()
            .into_iter()
            .map(|e| {
                let level = match e.level {
                    LogLevel::Success => "OK",
                    LogLevel::Error => "ERROR",
                    LogLevel::Warning => "WARN",
                    LogLevel::Info => "INFO",
                };
                format!("{level}: {}", e.message)
            })
            .collect()
    }

    pub fn run(&mut self) -> io::Result<()> {
        tui::run(self)
    }

    fn start_organization(&mut self, dry_run: bool) {
        // Count files upfront so the progress bar has a denominator
        self.total_files = std::fs::read_dir(&self.base_path)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.path().is_file())
                    .count()
            })
            .unwrap_or(0);
        self.live_moved = 0;
        self.live_skipped = 0;
        self.live_failed = 0;

        let base_path = self.base_path.clone();
        let ext_map = self.organizer.config.build_extension_map();
        let skip_patterns = self.organizer.config.skip_patterns.clone();
        let skip_hidden = self.organizer.config.skip_hidden_files;

        self.state = AppState::Organizing(Job::spawn(move |tx| {
            let _ = tx.send(Progress::Update("Sorting files by extension...".into()));
            let category_counts = Arc::new(Mutex::new(HashMap::new()));
            let files_organized = Arc::new(Mutex::new(0usize));
            let files_skipped = Arc::new(Mutex::new(0usize));
            let files_failed = Arc::new(Mutex::new(0usize));
            let dir_manager = LazyDirManager::new();

            let entries: Vec<_> = fs::read_dir(&base_path)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .collect();

            entries.par_iter().for_each(|entry| {
                let file_path = entry.path();
                let filename = match file_path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n,
                    None => {
                        *files_skipped.lock().unwrap() += 1;
                        return;
                    }
                };
                if skip_patterns.iter().any(|p| filename.contains(p)) {
                    *files_skipped.lock().unwrap() += 1;
                    return;
                }
                if skip_hidden && is_hidden(&file_path) {
                    *files_skipped.lock().unwrap() += 1;
                    return;
                }
                let extension = file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .unwrap_or_else(|| "unknown".to_string());
                let (_cat, folder_name) = ext_map
                    .get(&extension)
                    .cloned()
                    .unwrap_or_else(|| ("extras".to_string(), "Extras".to_string()));
                let target_dir = base_path.join(&folder_name);
                let target_path = target_dir.join(filename);
                let final_target = {
                    if !target_path.exists() {
                        target_path.clone()
                    } else {
                        let parent = target_path.parent().unwrap();
                        let stem = target_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("file");
                        let ext = target_path
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| format!(".{e}"))
                            .unwrap_or_default();
                        match (1..1000usize)
                            .map(|i| parent.join(format!("{stem}_{i}{ext}")))
                            .find(|p| !p.exists())
                        {
                            Some(p) => p,
                            None => {
                                *files_failed.lock().unwrap() += 1;
                                return;
                            }
                        }
                    }
                };
                if !dry_run {
                    if dir_manager.ensure_dir_exists(&target_dir).is_err() {
                        *files_failed.lock().unwrap() += 1;
                        return;
                    }
                    if fs::rename(&file_path, &final_target).is_err() {
                        *files_failed.lock().unwrap() += 1;
                        return;
                    }
                }
                let moved_now = {
                    let mut g = files_organized.lock().unwrap();
                    *g += 1;
                    *g
                };
                *category_counts
                    .lock()
                    .unwrap()
                    .entry(folder_name.clone())
                    .or_insert(0) += 1;
                // Send a structured progress token so tick() can parse live counts
                let _ = tx.send(Progress::Update(format!("moved:{moved_now}:{folder_name}")));
            });

            let x = OrganizeResult {
                files_organized: *files_organized.lock().unwrap(),
                files_skipped: *files_skipped.lock().unwrap(),
                files_failed: *files_failed.lock().unwrap(),
                category_counts: category_counts.lock().unwrap().clone(),
            };
            x
        }));
    }

    fn draw_ready(&self, f: &mut Frame, cols: [Rect; 3]) {
        // Left: path + category list
        let cat_count = self.organizer.config.categories.len();
        let mut folder_names: Vec<String> = self
            .organizer
            .config
            .categories
            .values()
            .map(|c| {
                c.folder_name
                    .clone()
                    .unwrap_or_else(|| "Unnamed".to_string())
            })
            .collect();
        folder_names.sort();
        let folder_summary = folder_names.join(" ");
        let lines = vec![
            tui::blank(),
            tui::section_label("target"),
            tui::path_line(&self.base_path.display().to_string()),
            tui::blank(),
            tui::section_label("categories configured"),
            tui::stat("total", cat_count.to_string(), tui::INFO),
            tui::blank(),
            tui::header(&folder_summary, tui::MUTED),
        ];
        f.render_widget(tui::paragraph(lines, "scan target"), cols[0]);

        // Mid: how it works
        let mut mid = vec![
            tui::blank(),
            tui::header("ready to organize", tui::OK),
            tui::blank(),
            tui::bullet("groups files by extension into folders"),
            tui::bullet(format!("{cat_count} categories configured")),
            tui::bullet("creates destination dirs on demand"),
            tui::bullet("renames on collision (file_1, file_2 …)"),
            tui::blank(),
        ];
        for patterns in &self.organizer.config.skip_patterns {
            mid.push(tui::bullet(patterns.clone()))
        }
        tui::header("press [s] to run  or  [d] for dry run", tui::MUTED);
        f.render_widget(tui::paragraph(mid, "overview"), cols[1]);

        // Right: options snapshot
        // let mut right = vec![
        //     tui::blank(),
        //     tui::section_label("mode"),
        //     tui::stat("collision", "rename", tui::WARN),
        //     tui::stat("hidden files", "skip", tui::MUTED),
        //     tui::blank(),
        //     tui::section_label("skip patterns"),
        //     ];
        // for patterns in &self.organizer.config.skip_patterns {
        //     right.push(tui::bullet(patterns.clone()))
        // }
        // f.render_widget(tui::paragraph(right, "options"), cols[2]);
    }

    fn draw_organizing(&self, f: &mut Frame, cols: [Rect; 3], msg: &str) {
        // Tyr is fast af boi fast as f...
        // drawing anything just makes it look awakward
        tui::busy_gauge(f, cols[0], "progress", msg, tui::INFO, self.tick);
    }

    fn draw_complete(&self, f: &mut Frame, cols: [Rect; 3], result: &OrganizeResult) {
        // Left: summary numbers
        let left = vec![
            tui::blank(),
            tui::header("✓ organization complete", tui::OK),
            tui::blank(),
            tui::stat("moved", result.files_organized.to_string(), tui::OK),
            tui::stat("skipped", result.files_skipped.to_string(), tui::WARN),
            tui::stat("failed", result.files_failed.to_string(), tui::ERR),
        ];
        f.render_widget(tui::paragraph(left, "results"), cols[0]);

        // category breakdown as bar chart
        let mut entries: Vec<(String, usize, Color)> = result
            .category_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v, tui::INFO))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        tui::bar_chart(f, cols[1], "by category", &entries);
    }

    /// Auto-organize files without UI interaction.
    pub fn auto_organize(&mut self) -> io::Result<()> {
        let result = self.organizer.organize_directory(&self.base_path, false)?;

        println!("\n\u{2726} Organization Complete!\n");
        println!("Summary:");
        println!("   \u{2022} Files organized: {}", result.files_organized);
        println!("   \u{2022} Files skipped:   {}", result.files_skipped);
        println!("   \u{2022} Files failed:    {}", result.files_failed);

        if !result.category_counts.is_empty() {
            println!("\nCategories:");
            let mut sorted: Vec<_> = result.category_counts.iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(a.1));
            for (category, count) in sorted {
                println!("   {category:<20} \u{2192} {count} files");
            }
        }

        self.state = AppState::Complete(result);
        Ok(())
    }
}

impl App for TuiApp {
    fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        if let AppState::Organizing(job) = &mut self.state {
            // Drain all pending messages; parse structured ones for live counters
            // (job.poll() only returns Some on Done, so we peek the channel ourselves
            //  via the public .message field which the framework already updates)
            if let Some(result) = job.poll() {
                self.live_moved = result.files_organized;
                self.live_skipped = result.files_skipped;
                self.live_failed = result.files_failed;
                self.state = AppState::Complete(result);
                return;
            }
            // Parse the latest message for live moved count
            let msg = job.message.clone();
            if let Some(rest) = msg.strip_prefix("moved:") {
                let mut parts = rest.splitn(2, ':');
                if let Some(n) = parts.next().and_then(|s| s.parse::<usize>().ok()) {
                    self.live_moved = n;
                }
            }
        }
    }

    fn needs_clear(&self) -> bool {
        self.state_changed
    }

    fn on_key(&mut self, key: KeyCode) -> Flow {
        match (&self.state, key) {
            (AppState::Organizing(_), KeyCode::Char('q') | KeyCode::Esc) => Flow::Continue,
            (_, KeyCode::Char('q') | KeyCode::Esc) => Flow::Quit,
            (AppState::Ready, KeyCode::Char('s')) => {
                self.start_organization(false);
                Flow::Continue
            }
            (AppState::Ready, KeyCode::Char('d')) => {
                self.start_organization(true);
                Flow::Continue
            }
            _ => Flow::Continue,
        }
    }

    fn draw(&self, f: &mut Frame) {
        let rows = tui::vsplit(
            f.area(),
            &[
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ],
        );

        let bindings: &[(&str, &str)] = match &self.state {
            AppState::Ready => &[("s", "start"), ("d", "dry run"), ("q", "quit")],
            AppState::Organizing(_) => &[("...", "organizing — please wait")],
            AppState::Complete(_) => &[("q", "quit")],
        };

        let (status, status_color) = match &self.state {
            AppState::Ready => ("ready", tui::OK),
            AppState::Organizing(_) => ("running", tui::INFO),
            AppState::Complete(_) => ("done", tui::OK),
        };

        let cols = tui::screen_frame(
            f,
            rows[0],
            rows[1],
            rows[2],
            "extension-based organizer",
            status,
            status_color,
            bindings,
            40,
            25,
        );

        match &self.state {
            AppState::Ready => self.draw_ready(f, cols),
            AppState::Organizing(job) => self.draw_organizing(f, cols, &job.message),
            AppState::Complete(result) => self.draw_complete(f, cols, result),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = FileOrganizerConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("images"));
        assert!(toml_str.contains("jpg"));
    }

    #[test]
    fn test_extension_map_building() {
        let config = FileOrganizerConfig::default();
        let ext_map = config.build_extension_map();

        assert!(ext_map.contains_key("jpg"));
        assert!(ext_map.contains_key("mp4"));
        assert!(ext_map.contains_key("pdf"));
    }
}
