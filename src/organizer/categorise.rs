//  Copyright (C) 2026 Dawood Khan
//  SPDX-License-Identifier: GPL-3.0-or-later

// Maintainer Dawood (Nurysso) contact - nurysso [at] proton.me

/*
 * This file is responsible for executing category(file extension) based file organizing for
 * tyr(https:github.com/Nurysso/tyr) project
*/

use crate::organizer::tui::{self, App, Flow, Job, Progress};
use crate::organizer::utils::{LazyDirManager, LogLevel, SafeLogger, is_hidden};
use ratatui::crossterm::event::KeyCode;
use ratatui::style::Color;
use ratatui::{Frame, layout::Constraint, layout::Rect};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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

            let prefix = if dry_run { "[DRY RUN] " } else { "" };
            self.logger.log(
                LogLevel::Success,
                format!("{prefix}{} \u{2192} {}", filename, folder_name),
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
    Organizing(Job<io::Result<OrganizeResult>>, bool),
    Complete(OrganizeResult, bool),
    Failed(String),
}

pub struct TuiApp {
    organizer: Arc<FileOrganizer>,
    base_path: PathBuf,
    state: AppState,
    tick: usize,
}

impl TuiApp {
    pub fn new(config: FileOrganizerConfig, base_path: PathBuf) -> Self {
        Self {
            organizer: Arc::new(FileOrganizer::new(config)),
            base_path,
            state: AppState::Ready,
            tick: 0,
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
        let organizer = Arc::clone(&self.organizer);
        let base_path = self.base_path.clone();
        let job = Job::spawn(move |tx| {
            let _ = tx.send(Progress::Update("Sorting files by extension...".into()));
            organizer.organize_directory(&base_path, dry_run)
        });
        self.state = AppState::Organizing(job, dry_run);
    }

    fn draw_ready(&self, f: &mut Frame, cols: [Rect; 3]) {
        let cat_count = self.organizer.config.categories.len();
        let mut folder_names: Vec<String> = self
            .organizer
            .config
            .categories
            .iter()
            .map(|(k, c)| c.folder_name.clone().unwrap_or_else(|| k.clone()))
            .collect();
        folder_names.sort();
        let mut left = vec![
            tui::blank(),
            tui::section_label("target"),
            tui::path_line(&self.base_path.display().to_string()),
            tui::blank(),
            tui::section_label("destination folders"),
        ];
        left.extend(folder_names.into_iter().map(tui::bullet));
        left.push(tui::bullet("Extras (unmatched)"));
        f.render_widget(tui::paragraph(left, "scan target"), cols[0]);

        let mid = vec![
            tui::blank(),
            tui::header("ready to organize", tui::OK),
            tui::blank(),
            tui::bullet("groups files by extension into folders"),
            tui::bullet(format!("{cat_count} categories configured")),
            tui::bullet("creates destination dirs on demand"),
            tui::bullet("renames on collision (file_1, file_2 …)"),
            tui::blank(),
            tui::header("[d] preview first, then [s] to apply", tui::MUTED),
        ];
        f.render_widget(tui::paragraph(mid, "overview"), cols[1]);

        let mut right = vec![
            tui::blank(),
            tui::stat(
                "hidden files",
                if self.organizer.config.skip_hidden_files { "skip" } else { "include" },
                tui::MUTED,
            ),
            tui::blank(),
            tui::section_label("skip patterns"),
        ];
        right.extend(self.organizer.config.skip_patterns.iter().cloned().map(tui::bullet));
        f.render_widget(tui::paragraph(right, "options"), cols[2]);
    }

    fn draw_complete(&self, f: &mut Frame, cols: [Rect; 3], result: &OrganizeResult, dry_run: bool) {
        let (title, moved_label) = if dry_run {
            ("dry run — nothing moved yet", "would move")
        } else {
            ("✓ organization complete", "moved")
        };
        let mut left = vec![
            tui::blank(),
            tui::header(title, if dry_run { tui::WARN } else { tui::OK }),
            tui::blank(),
            tui::stat(moved_label, result.files_organized.to_string(), tui::OK),
            tui::stat("skipped", result.files_skipped.to_string(), tui::WARN),
            tui::stat("failed", result.files_failed.to_string(), tui::ERR),
        ];
        if dry_run {
            left.push(tui::blank());
            left.push(tui::header("press [s] to apply these moves", tui::INFO));
        }
        f.render_widget(tui::paragraph(left, "results"), cols[0]);

        let mut entries: Vec<(String, usize, Color)> = result
            .category_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v, tui::INFO))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        tui::bar_chart(f, cols[1], "by category", &entries);

        // newest entries first; the logger keeps both the dry run and the real run
        let logs = self.organizer.get_logger().get_logs();
        let rows = cols[2].height.saturating_sub(2) as usize;
        let shown = logs.iter().rev().take(rows).map(|e| {
            let lvl = match e.level {
                LogLevel::Success => tui::Level::Ok,
                LogLevel::Error => tui::Level::Err,
                LogLevel::Warning => tui::Level::Warn,
                LogLevel::Info => tui::Level::Info,
            };
            (lvl, e.message.as_str())
        });
        f.render_widget(tui::log_list("activity", shown), cols[2]);
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

        self.state = AppState::Complete(result, false);
        Ok(())
    }
}

impl App for TuiApp {
    fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        if let AppState::Organizing(job, dry_run) = &mut self.state {
            let dry_run = *dry_run;
            match job.poll() {
                Some(Ok(result)) => self.state = AppState::Complete(result, dry_run),
                Some(Err(e)) => self.state = AppState::Failed(e.to_string()),
                None => {}
            }
        }
    }

    fn on_key(&mut self, key: KeyCode) -> Flow {
        match (&self.state, key) {
            (AppState::Organizing(..), _) => Flow::Continue,
            (_, KeyCode::Char('q') | KeyCode::Esc) => Flow::Quit,
            (AppState::Ready | AppState::Complete(_, true), KeyCode::Char('s')) => {
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

        let (status, status_color, bindings): (&str, Color, &[(&str, &str)]) = match &self.state {
            AppState::Ready => (
                "ready",
                tui::OK,
                &[("d", "dry run"), ("s", "organize"), ("q", "quit")],
            ),
            AppState::Organizing(..) => ("running", tui::INFO, &[("…", "organizing — please wait")]),
            AppState::Complete(_, true) => (
                "preview",
                tui::WARN,
                &[("s", "apply moves"), ("q", "quit")],
            ),
            AppState::Complete(_, false) => ("done", tui::OK, &[("q", "quit")]),
            AppState::Failed(_) => ("error", tui::ERR, &[("q", "quit")]),
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
            AppState::Organizing(job, _) => {
                tui::busy_gauge(f, cols[1], "progress", &job.message, tui::INFO, self.tick)
            }
            AppState::Complete(result, dry_run) => self.draw_complete(f, cols, result, *dry_run),
            AppState::Failed(e) => f.render_widget(
                tui::paragraph(vec![tui::blank(), tui::header(format!("✗ {e}"), tui::ERR)], "error"),
                cols[1],
            ),
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

    #[test]
    fn dry_run_moves_nothing_then_real_run_applies() {
        let dir = std::env::temp_dir().join(format!("tyr-cat-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.jpg"), "").unwrap();
        fs::write(dir.join("b.zzz"), "").unwrap();
        let org = FileOrganizer::new(FileOrganizerConfig::default());

        let dry = org.organize_directory(&dir, true).unwrap();
        assert_eq!(dry.files_organized, 2);
        assert!(dir.join("a.jpg").exists() && !dir.join("Images").exists());

        let real = org.organize_directory(&dir, false).unwrap();
        assert_eq!(real.files_organized, 2);
        assert!(dir.join("Images/a.jpg").exists() && dir.join("Extras/b.zzz").exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
