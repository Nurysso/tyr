// organize files based on extension
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// Configuration Structures

#[derive(Debug, Deserialize, Serialize)]
pub struct FileOrganizerConfig {
    #[serde(default)]
    pub categories: HashMap<String, CategoryConfig>,

    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    #[serde(default)]
    pub skip_patterns: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CategoryConfig {
    pub extensions: Vec<String>,

    #[serde(default)]
    pub folder_name: Option<String>,
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

// Config Loading

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

// Lazy Directory Manager (Reduces syscalls)

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

        // Check if exists before creating
        if !path.exists() {
            fs::create_dir_all(path)?;
        }

        cache.insert(path.to_path_buf(), true);
        Ok(())
    }
}

// Error-Safe Logger

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

    // pub fn clear(&self) {
    //     if let Ok(mut entries) = self.log_entries.lock() {
    //         entries.clear();
    //     }
    // }
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

        // Collect all file entries
        let entries: Vec<_> = fs::read_dir(base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .collect();

        let category_counts = Arc::new(Mutex::new(HashMap::new()));
        let files_organized = Arc::new(Mutex::new(0usize));
        let files_skipped = Arc::new(Mutex::new(0usize));
        let files_failed = Arc::new(Mutex::new(0usize));

        // Process files in parallel for speed
        entries.par_iter().for_each(|entry| {
            let file_path = entry.path();
            let filename = match file_path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => {
                    *files_skipped.lock().unwrap() += 1;
                    return;
                }
            };

            // Skip system files
            if self.should_skip_file(filename) {
                self.logger
                    .log(LogLevel::Info, format!("Skipped: {}", filename), None);
                *files_skipped.lock().unwrap() += 1;
                return;
            }

            // Get extension and category
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

            // Handle naming conflicts
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
                // Lazy create directory
                if let Err(e) = self.dir_manager.ensure_dir_exists(&target_dir) {
                    self.logger.log(
                        LogLevel::Error,
                        format!("Failed to create dir: {}", folder_name),
                        Some(e.to_string()),
                    );
                    *files_failed.lock().unwrap() += 1;
                    return;
                }

                // Move file
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
                format!("{} → {}", filename, folder_name),
                None,
            );

            *files_organized.lock().unwrap() += 1;
            *category_counts
                .lock()
                .unwrap()
                .entry(folder_name)
                .or_insert(0) += 1;
        });

        // Fix: Extract values before creating the result to avoid borrow issues
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
            .any(|pattern| filename.contains(pattern))
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

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Terminal,
};
use std::io::stdout;
use std::time::Duration;

pub struct TuiApp {
    organizer: FileOrganizer,
    base_path: PathBuf,
    state: AppState,
}

enum AppState {
    Ready,
    Organizing,
    Complete(OrganizeResult),
}

impl TuiApp {
    pub fn new(config: FileOrganizerConfig, base_path: PathBuf) -> Self {
        Self {
            organizer: FileOrganizer::new(config),
            base_path,
            state: AppState::Ready,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.draw_ui(f))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('s') => {
                            if matches!(self.state, AppState::Ready) {
                                self.start_organization(false)?;
                            }
                        }
                        KeyCode::Char('d') => {
                            if matches!(self.state, AppState::Ready) {
                                self.start_organization(true)?;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn start_organization(&mut self, dry_run: bool) -> io::Result<()> {
        self.state = AppState::Organizing;
        let result = self
            .organizer
            .organize_directory(&self.base_path, dry_run)?;
        self.state = AppState::Complete(result);
        Ok(())
    }

    fn draw_ui(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(5),
                Constraint::Length(3),
            ])
            .split(f.size());

        // Title
        let title = Paragraph::new(" Kondo - Extension-Based File Organizer")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Main content
        match &self.state {
            AppState::Ready => self.draw_ready_state(f, chunks[1]),
            AppState::Organizing => self.draw_organizing_state(f, chunks[1]),
            AppState::Complete(result) => self.draw_complete_state(f, chunks[1], result),
        }

        // Logs
        self.draw_logs(f, chunks[2]);

        // Controls
        self.draw_controls(f, chunks[3]);
    }

    fn draw_ready_state(&self, f: &mut ratatui::Frame, area: Rect) {
        let categories = self.organizer.config.categories.len();

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                " Ready to Organize Files",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("Directory: "),
                Span::styled(
                    self.base_path.display().to_string(),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "How it works:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from("  • Organizes files by extension into folders"),
            Line::from(format!(
                "  • {} categories configured (Images, Videos, Documents, etc.)",
                categories
            )),
            Line::from("  • Creates folders only when needed"),
            Line::from("  • Handles naming conflicts automatically"),
            Line::from(""),
            Line::from(Span::styled(
                " Press 's' to start organizing",
                Style::default().fg(Color::Green),
            )),
            Line::from(Span::styled(
                " Press 'd' for dry run (preview only)",
                Style::default().fg(Color::Yellow),
            )),
        ];

        let widget =
            Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(" Status "));
        f.render_widget(widget, area);
    }

    fn draw_organizing_state(&self, f: &mut ratatui::Frame, area: Rect) {
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Organizing Files "),
            )
            .gauge_style(Style::default().fg(Color::Cyan))
            .label(" Sorting files by extension...")
            .percent(50);
        f.render_widget(gauge, area);
    }

    fn draw_complete_state(&self, f: &mut ratatui::Frame, area: Rect, result: &OrganizeResult) {
        let mut lines = vec![
            Line::from(Span::styled(
                "✦ Organization Complete!",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw(" Files organized: "),
                Span::styled(
                    result.files_organized.to_string(),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw(" Files skipped: "),
                Span::styled(
                    result.files_skipped.to_string(),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(vec![
                Span::raw(" Files failed: "),
                Span::styled(
                    result.files_failed.to_string(),
                    Style::default().fg(Color::Red),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                " Category Summary:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        // Sort categories by count for better display
        let mut sorted_categories: Vec<_> = result.category_counts.iter().collect();
        sorted_categories.sort_by(|a, b| b.1.cmp(a.1));

        for (category, count) in sorted_categories.iter().take(10) {
            let icon = match category.as_str() {
                name if name.contains("Image") => "",
                name if name.contains("Video") => "",
                name if name.contains("Audio") => "",
                name if name.contains("Document") => "",
                name if name.contains("Code") => "",
                name if name.contains("Archive") => "",
                _ => "",
            };

            lines.push(Line::from(vec![
                Span::raw(format!("  {} ", icon)),
                Span::styled(format!("{:15}", category), Style::default().fg(Color::Cyan)),
                Span::raw(" → "),
                Span::styled(
                    format!("{} files", count),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        if sorted_categories.len() > 10 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("... and {} more categories", sorted_categories.len() - 10),
                Style::default().fg(Color::Gray),
            )));
        }

        let widget =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Results "));
        f.render_widget(widget, area);
    }

    fn draw_logs(&self, f: &mut ratatui::Frame, area: Rect) {
        let logs = self.organizer.get_logger().get_logs();
        let items: Vec<ListItem> = logs
            .iter()
            .rev()
            .take(3)
            .map(|log| {
                let (style, icon) = match log.level {
                    LogLevel::Success => (Style::default().fg(Color::Green), "✓"),
                    LogLevel::Error => (Style::default().fg(Color::Red), "✗"),
                    LogLevel::Warning => (Style::default().fg(Color::Yellow), "⚠"),
                    LogLevel::Info => (Style::default().fg(Color::Gray), "ℹ"),
                };
                ListItem::new(format!("{} {}", icon, log.message)).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Recent Activity "),
        );
        f.render_widget(list, area);
    }

    fn draw_controls(&self, f: &mut ratatui::Frame, area: Rect) {
        let controls = match &self.state {
            AppState::Ready => " 's' Start | 'd' Dry Run | 'q' Quit",
            AppState::Organizing => " Organizing... Please wait",
            AppState::Complete(_) => " 'q' Quit (or press any key to exit)",
        };

        let widget = Paragraph::new(controls)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(widget, area);
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
