// Advanced filename operations with ML-based similarity detection and file organization
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Checks if a file should be skipped based on common system file patterns
pub fn should_skip_file(filename: &str) -> bool {
    let skip_patterns = [
        ".DS_Store",
        "Thumbs.db",
        ".git",
        ".gitignore",
        "desktop.ini",
        ".localized",
        "~$",
    ];

    skip_patterns
        .iter()
        .any(|pattern| filename.contains(pattern))
}

// ML-Based Similarity Detection

/// Configuration for similarity detection
#[derive(Debug, Clone)]
pub struct SimilarityConfig {
    #[allow(dead_code)]
    /// Threshold for Levenshtein similarity (0.0 to 1.0)
    pub levenshtein_threshold: f64,
    #[allow(dead_code)]
    /// Threshold for Jaccard similarity (0.0 to 1.0)
    pub jaccard_threshold: f64,

    /// Weight for Levenshtein distance (0.0 to 1.0)
    pub levenshtein_weight: f64,

    /// Weight for Jaccard similarity (0.0 to 1.0)
    pub jaccard_weight: f64,

    /// Minimum similarity score to consider files related (0.0 to 1.0)
    pub min_similarity_score: f64,
}

impl Default for SimilarityConfig {
    fn default() -> Self {
        Self {
            levenshtein_threshold: 0.7,
            jaccard_threshold: 0.5,
            levenshtein_weight: 0.6,
            jaccard_weight: 0.4,
            min_similarity_score: 0.65,
        }
    }
}

/// Represents a group of similar files
#[derive(Debug, Clone)]
pub struct FileGroup {
    pub representative_name: String,
    pub files: Vec<String>,
    pub avg_similarity: f64,
}

/// Result of organizing files by similarity
#[derive(Debug)]
pub struct OrganizeResult {
    pub files_moved: usize,
    pub folders_created: usize,
    pub files_skipped: usize,
    pub skipped_details: Vec<SkippedFile>,
    pub errors: Vec<String>,
}

/// Information about a skipped file
#[derive(Debug, Clone)]
pub struct SkippedFile {
    pub filename: String,
    pub reason: SkipReason,
}

#[derive(Debug, Clone)]
pub enum SkipReason {
    SingleFile,       // Only one file in its group
    SystemFile,       // System file pattern detected
    AlreadyOrganized, // Already in a subfolder
}

/// Calculates Levenshtein distance between two strings
/// This measures the minimum number of single-character edits needed
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    // Initialize first row and column
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    // Fill the matrix
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };

            matrix[i][j] = *[
                matrix[i - 1][j] + 1,        // deletion
                matrix[i][j - 1] + 1,        // insertion
                matrix[i - 1][j - 1] + cost, // substitution
            ]
            .iter()
            .min()
            .unwrap();
        }
    }

    matrix[len1][len2]
}

/// Calculates normalized Levenshtein similarity (0.0 to 1.0)
/// Higher values mean more similar
pub fn levenshtein_similarity(s1: &str, s2: &str) -> f64 {
    let distance = levenshtein_distance(s1, s2);
    let max_len = s1.len().max(s2.len()) as f64;

    if max_len == 0.0 {
        return 1.0;
    }

    1.0 - (distance as f64 / max_len)
}

/// Tokenizes a filename into meaningful parts - IMPROVED VERSION
/// Now preserves meaningful phrases and handles common patterns better
fn tokenize_filename(filename: &str) -> HashSet<String> {
    let mut tokens = HashSet::new();

    // Remove extension if present
    let name = filename.rsplit_once('.').map(|(n, _)| n).unwrap_or(filename);
    let name_lower = name.to_lowercase();

    // First, add the full name without delimiters as a token (helps with phrases)
    let clean_full = name_lower
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>();

    if !clean_full.trim().is_empty() {
        tokens.insert(clean_full.trim().to_string());
    }

    // Extract common phrases that should stay together
    let phrases = [
        "whatsapp chat",
        "whatsapp chats",
        "whatsapp image",
        "screenshot",
        "screen shot",
        "chatgpt",
        "img_",
        "photo",
        "picture",
        "document",
        "download",
    ];

    for phrase in &phrases {
        if name_lower.contains(phrase) {
            tokens.insert(phrase.to_string());
        }
    }

    // Now tokenize individual words
    let delimiters = ['-', '_', '.', '(', ')', '[', ']', '{', '}'];
    let mut current_token = String::new();

    for c in name.chars() {
        if delimiters.contains(&c) {
            if !current_token.is_empty() {
                let token_lower = current_token.to_lowercase();
                if token_lower.len() > 1 && !token_lower.chars().all(|c| c.is_numeric()) {
                    tokens.insert(token_lower);
                }
                current_token.clear();
            }
        } else if c == ' ' {
            // For spaces, we want to preserve word boundaries but also create word tokens
            if !current_token.is_empty() {
                let token_lower = current_token.to_lowercase();
                if token_lower.len() > 1 && !token_lower.chars().all(|c| c.is_numeric()) {
                    tokens.insert(token_lower);
                }
                current_token.clear();
            }
        } else {
            current_token.push(c);
        }
    }

    if !current_token.is_empty() {
        let token_lower = current_token.to_lowercase();
        if token_lower.len() > 1 && !token_lower.chars().all(|c| c.is_numeric()) {
            tokens.insert(token_lower);
        }
    }

    // Also extract bigrams (two-word combinations) for better matching
    let words: Vec<String> = name
        .split(|c: char| !c.is_alphanumeric() && c != ' ')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_lowercase())
        .collect();

    for window in words.windows(2) {
        let bigram = format!("{} {}", window[0], window[1]);
        tokens.insert(bigram);
    }

    tokens
}

/// Calculates Jaccard similarity between two strings based on token sets
/// This measures overlap of words/tokens in the filenames
pub fn jaccard_similarity(s1: &str, s2: &str) -> f64 {
    let tokens1 = tokenize_filename(s1);
    let tokens2 = tokenize_filename(s2);

    if tokens1.is_empty() && tokens2.is_empty() {
        return 1.0;
    }

    let intersection: HashSet<_> = tokens1.intersection(&tokens2).collect();
    let union: HashSet<_> = tokens1.union(&tokens2).collect();

    if union.is_empty() {
        return 0.0;
    }

    intersection.len() as f64 / union.len() as f64
}

/// Calculates combined similarity score using both metrics
pub fn combined_similarity(s1: &str, s2: &str, config: &SimilarityConfig) -> f64 {
    let lev_sim = levenshtein_similarity(s1, s2);
    let jac_sim = jaccard_similarity(s1, s2);

    (lev_sim * config.levenshtein_weight) + (jac_sim * config.jaccard_weight)
}

/// Groups similar files together using clustering
pub fn group_similar_files(filenames: &[String], config: &SimilarityConfig) -> Vec<FileGroup> {
    if filenames.is_empty() {
        return Vec::new();
    }

    let mut groups: Vec<FileGroup> = Vec::new();
    let mut assigned: HashSet<usize> = HashSet::new();

    for i in 0..filenames.len() {
        if assigned.contains(&i) {
            continue;
        }

        let mut group_files = vec![filenames[i].clone()];
        let mut similarities = Vec::new();
        assigned.insert(i);

        // Find all files similar to this one
        for j in (i + 1)..filenames.len() {
            if assigned.contains(&j) {
                continue;
            }

            let similarity = combined_similarity(&filenames[i], &filenames[j], config);

            if similarity >= config.min_similarity_score {
                group_files.push(filenames[j].clone());
                similarities.push(similarity);
                assigned.insert(j);
            }
        }

        let avg_similarity = if similarities.is_empty() {
            1.0
        } else {
            similarities.iter().sum::<f64>() / similarities.len() as f64
        };

        groups.push(FileGroup {
            representative_name: extract_common_prefix(&group_files),
            files: group_files,
            avg_similarity,
        });
    }

    groups
}

/// Extracts common prefix from a group of filenames
fn extract_common_prefix(filenames: &[String]) -> String {
    if filenames.is_empty() {
        return String::new();
    }

    if filenames.len() == 1 {
        return filenames[0].clone();
    }

    let first = &filenames[0];
    let mut prefix = String::new();

    for (i, c) in first.chars().enumerate() {
        let all_match = filenames.iter().all(|f| f.chars().nth(i) == Some(c));

        if all_match {
            prefix.push(c);
        } else {
            break;
        }
    }

    // Clean up the prefix (remove trailing delimiters)
    prefix
        .trim_end_matches(['-', '_', ' ', '.', '(', '[', '{'].as_ref())
        .to_string()
}

/// Suggests a smart, memorable folder name for a group of similar files
pub fn suggest_folder_name(group: &FileGroup) -> String {
    let name = if group.representative_name.is_empty() {
        "SimilarFiles"
    } else {
        &group.representative_name
    };

    // Apply smart naming rules
    let cleaned = smart_folder_naming(name);

    if cleaned.is_empty() {
        "SimilarFiles".to_string()
    } else {
        cleaned
    }
}

/// Applies intelligent naming rules to create memorable folder names
fn smart_folder_naming(name: &str) -> String {
    let lower = name.to_lowercase();

    // Pattern-based naming rules
    let patterns = [
        // WhatsApp
        ("whatsapp chat", "WhatsAppChats"),
        ("whatsapp image", "WhatsAppImages"),
        ("whatsapp", "WhatsApp"),
        // Screenshots
        ("screenshot", "Screenshots"),
        ("screen shot", "Screenshots"),
        ("screencapture", "Screenshots"),
        // ChatGPT images
        ("chatgpt", "ChatGPTImages"),
        // Common document patterns
        ("document", "Documents"),
        ("report", "Reports"),
        ("invoice", "Invoices"),
        ("receipt", "Receipts"),
        // Image types
        ("img_", "Images"),
        ("dsc", "CameraPhotos"),
        ("dcim", "CameraPhotos"),
        ("photo", "Photos"),
        ("pic", "Pictures"),
        // Videos
        ("vid_", "Videos"),
        ("video", "Videos"),
        ("mov_", "Videos"),
        // Downloads
        ("download", "Downloads"),
        // Archives
        ("backup", "Backups"),
        ("archive", "Archives"),
    ];

    // Check patterns
    for (pattern, replacement) in &patterns {
        if lower.contains(pattern) {
            return replacement.to_string();
        }
    }

    // Extract meaningful base name
    let mut result = name
        .split(|c: char| !c.is_alphanumeric() && c != ' ')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("");

    // Remove dates (YYYY-MM-DD, YYYY_MM_DD, etc.)
    result = remove_date_patterns(&result);

    // Remove version numbers (v1, v2, etc.)
    result = remove_version_patterns(&result);

    // Remove numbered suffixes like (1), (2), _1, _2
    result = remove_number_suffixes(&result);

    // Capitalize first letter and format
    if !result.is_empty() {
        let mut chars = result.chars();
        result = match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => result,
        };
    }

    // Ensure it's a valid folder name (Windows and Unix compatible)
    result
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "")
        .trim()
        .to_string()
}

fn remove_date_patterns(s: &str) -> String {
    let mut cleaned = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check for date-like patterns (4 digits followed by more digits)
        if i + 7 < chars.len() && chars[i..i + 4].iter().all(|c| c.is_numeric()) {
            // Skip potential date
            i += 8;
            while i < chars.len() && (chars[i].is_numeric() || chars[i] == '-' || chars[i] == '_') {
                i += 1;
            }
        } else {
            cleaned.push(chars[i]);
            i += 1;
        }
    }

    cleaned
}

fn remove_version_patterns(s: &str) -> String {
    let mut result = s.to_string();

    // Remove patterns like "_v1", "-v2", "v3"
    if let Some(pos) = result.rfind("v") {
        if pos > 0 {
            let after = &result[pos + 1..];
            if after
                .chars()
                .all(|c| c.is_numeric() || c == '.' || c == '_' || c == '-')
            {
                result = result[..pos].to_string();
            }
        }
    }

    result
}

fn remove_number_suffixes(s: &str) -> String {
    let mut result = s.to_string();

    // Remove trailing numbers and delimiters
    while !result.is_empty() {
        let last = result.chars().last().unwrap();
        if last.is_numeric() || last == '_' || last == '-' || last == ' ' {
            result.pop();
        } else {
            break;
        }
    }

    result
}

/// Organizes files by similarity, moving them into appropriate folders
pub fn organize_by_similarity(
    base_path: &Path,
    config: &SimilarityConfig,
    move_skipped: bool,
    logger: &mut dyn FnMut(&str),
) -> io::Result<OrganizeResult> {
    logger(&format!(
        "Starting organization in: {}",
        base_path.display()
    ));

    // Read all files
    let entries: Vec<_> = fs::read_dir(base_path)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();

    logger(&format!("Found {} files to process", entries.len()));

    let filenames: Vec<String> = entries
        .iter()
        .filter_map(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .collect();

    // Group files
    logger("Analyzing file similarities...");
    let groups = group_similar_files(&filenames, config);
    logger(&format!("Identified {} file groups", groups.len()));

    let mut files_moved = 0;
    let mut folders_created = 0;
    let mut files_skipped = 0;
    let mut skipped_details = Vec::new();
    let mut errors = Vec::new();

    // Prepare skip folder if needed
    let skip_folder = if move_skipped {
        let skip_dir = base_path.join("tyr-skip");
        if !skip_dir.exists() {
            match fs::create_dir(&skip_dir) {
                Ok(_) => {
                    logger(&format!("Created skip folder: {}", skip_dir.display()));
                    Some(skip_dir)
                }
                Err(e) => {
                    let err_msg = format!("Failed to create skip folder: {}", e);
                    logger(&err_msg);
                    errors.push(err_msg);
                    None
                }
            }
        } else {
            Some(skip_dir)
        }
    } else {
        None
    };

    // Process each group
    for group in groups {
        // Handle single files
        if group.files.len() < 2 {
            for filename in &group.files {
                // Check if it's a system file
                if should_skip_file(filename) {
                    skipped_details.push(SkippedFile {
                        filename: filename.clone(),
                        reason: SkipReason::SystemFile,
                    });
                    logger(&format!("Skipped system file: {}", filename));
                } else {
                    skipped_details.push(SkippedFile {
                        filename: filename.clone(),
                        reason: SkipReason::SingleFile,
                    });
                    logger(&format!("Skipped single file: {}", filename));
                }
                files_skipped += 1;

                // Move to skip folder if enabled
                if let Some(ref skip_dir) = skip_folder {
                    let source = base_path.join(filename);
                    let dest = skip_dir.join(filename);

                    if let Err(e) = fs::rename(&source, &dest) {
                        let err_msg =
                            format!("Failed to move '{}' to skip folder: {}", filename, e);
                        logger(&err_msg);
                        errors.push(err_msg);
                    } else {
                        logger(&format!("Moved to skip folder: {}", filename));
                    }
                }
            }
            continue;
        }

        let folder_name = suggest_folder_name(&group);
        let target_dir = base_path.join(&folder_name);

        // Create folder if it doesn't exist
        if !target_dir.exists() {
            match fs::create_dir(&target_dir) {
                Ok(_) => {
                    folders_created += 1;
                    logger(&format!("Created folder: {}", folder_name));
                }
                Err(e) => {
                    let err_msg = format!("Failed to create folder '{}': {}", folder_name, e);
                    logger(&err_msg);
                    errors.push(err_msg);
                    continue;
                }
            }
        }

        // Move files into the folder
        for filename in &group.files {
            let source = base_path.join(filename);
            let dest = target_dir.join(filename);

            // Handle naming conflicts
            let final_dest = if dest.exists() {
                match handle_naming_conflict(&dest) {
                    Ok(path) => path,
                    Err(e) => {
                        let err_msg = format!("Naming conflict for '{}': {}", filename, e);
                        logger(&err_msg);
                        errors.push(err_msg);
                        continue;
                    }
                }
            } else {
                dest
            };

            match fs::rename(&source, &final_dest) {
                Ok(_) => {
                    files_moved += 1;
                    logger(&format!("Moved: {} -> {}", filename, folder_name));
                }
                Err(e) => {
                    let err_msg = format!("Failed to move '{}': {}", filename, e);
                    logger(&err_msg);
                    errors.push(err_msg);
                }
            }
        }
    }

    logger(&format!(
        "Organization complete: {} files moved, {} folders created, {} files skipped",
        files_moved, folders_created, files_skipped
    ));

    Ok(OrganizeResult {
        files_moved,
        folders_created,
        files_skipped,
        skipped_details,
        errors,
    })
}

/// Handles naming conflicts by appending a number
fn handle_naming_conflict(path: &Path) -> io::Result<PathBuf> {
    let parent = path.parent().unwrap();
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let extension = path
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
        "Could not find available filename",
    ))
}

// TUI Implementation for Filename Organization

use std::sync::{Arc, Mutex};
use std::time::Duration;

/// TUI App for filename-based organization
pub struct FilenameTuiApp {
    base_path: PathBuf,
    config: SimilarityConfig,
    state: FilenameAppState,
    move_skipped_to_folder: bool,
    groups: Vec<FileGroup>,
    scroll_offset: usize,
    log_messages: Arc<Mutex<Vec<String>>>,
}

enum FilenameAppState {
    Ready,
    Analyzing,
    ReviewGroups,
    Organizing,
    Complete(OrganizeResult),
}

impl FilenameTuiApp {
    pub fn new(base_path: PathBuf, config: SimilarityConfig) -> Self {
        Self {
            base_path,
            config,
            state: FilenameAppState::Ready,
            move_skipped_to_folder: false,
            groups: Vec::new(),
            scroll_offset: 0,
            log_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn log(&self, message: &str) {
        if let Ok(mut logs) = self.log_messages.lock() {
            logs.push(message.to_string());
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        // Import crossterm types
        use crossterm::{
            event::{DisableMouseCapture, EnableMouseCapture},
            execute,
            terminal::{
                disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
            },
        };
        use std::io::stdout;

        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        // Import ratatui types
        use ratatui::{backend::CrosstermBackend, Terminal};

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
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> io::Result<()> {
        use crossterm::event::{self, Event, KeyCode};

        loop {
            terminal.draw(|f| self.draw_ui(f))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            // Allow quitting from any state
                            if matches!(self.state, FilenameAppState::Organizing) {
                                // Don't quit while organizing
                                continue;
                            }
                            self.log("User requested quit");
                            break;
                        }
                        KeyCode::Char('a') => {
                            if matches!(self.state, FilenameAppState::Ready) {
                                self.analyze_files()?;
                            }
                        }
                        KeyCode::Char('s') => {
                            if matches!(self.state, FilenameAppState::ReviewGroups) {
                                self.start_organization()?;
                            }
                        }
                        KeyCode::Char('k') => {
                            if matches!(self.state, FilenameAppState::ReviewGroups) {
                                self.move_skipped_to_folder = !self.move_skipped_to_folder;
                                self.log(&format!(
                                    "Toggle skip folder: {}",
                                    self.move_skipped_to_folder
                                ));
                            }
                        }
                        KeyCode::Char('r') => {
                            if matches!(self.state, FilenameAppState::Complete(_)) {
                                self.state = FilenameAppState::Ready;
                                self.groups.clear();
                                self.scroll_offset = 0;
                                self.log("Reset to ready state");
                            }
                        }
                        KeyCode::Up => {
                            if self.scroll_offset > 0 {
                                self.scroll_offset -= 1;
                            }
                        }
                        KeyCode::Down => {
                            self.scroll_offset += 1;
                        }
                        KeyCode::PageUp => {
                            self.scroll_offset = self.scroll_offset.saturating_sub(10);
                        }
                        KeyCode::PageDown => {
                            self.scroll_offset += 10;
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn analyze_files(&mut self) -> io::Result<()> {
        self.state = FilenameAppState::Analyzing;
        self.log("Starting file analysis");

        // Read directory
        let entries: Vec<_> = fs::read_dir(&self.base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .collect();

        self.log(&format!("Found {} files", entries.len()));

        let filenames: Vec<String> = entries
            .iter()
            .filter_map(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })
            .collect();

        self.groups = group_similar_files(&filenames, &self.config);
        self.log(&format!("Grouped into {} clusters", self.groups.len()));
        self.state = FilenameAppState::ReviewGroups;
        self.scroll_offset = 0;
        Ok(())
    }

    fn start_organization(&mut self) -> io::Result<()> {
        self.state = FilenameAppState::Organizing;
        self.log("Starting organization");

        let log_messages = Arc::clone(&self.log_messages);
        let mut logger = |msg: &str| {
            if let Ok(mut logs) = log_messages.lock() {
                logs.push(msg.to_string());
            }
        };

        // Perform organization
        let result = organize_by_similarity(
            &self.base_path,
            &self.config,
            self.move_skipped_to_folder,
            &mut logger,
        )?;

        self.state = FilenameAppState::Complete(result);
        self.scroll_offset = 0;
        Ok(())
    }

    fn draw_ui(&self, f: &mut ratatui::Frame) {
        use ratatui::{
            layout::{Constraint, Direction, Layout},
            style::{Color, Modifier, Style},
            widgets::{Block, Borders, Paragraph},
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(f.size());

        // Title
        let title = Paragraph::new(" Tyr - Filename Similarity Organizer")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Main content
        match &self.state {
            FilenameAppState::Ready => self.draw_ready_state(f, chunks[1]),
            FilenameAppState::Analyzing => self.draw_analyzing_state(f, chunks[1]),
            FilenameAppState::ReviewGroups => self.draw_review_state(f, chunks[1]),
            FilenameAppState::Organizing => self.draw_organizing_state(f, chunks[1]),
            FilenameAppState::Complete(result) => self.draw_complete_state(f, chunks[1], result),
        }

        // Controls
        self.draw_controls(f, chunks[2]);
    }

    fn draw_ready_state(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use ratatui::{
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Paragraph},
        };

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                " Ready to Analyze Files",
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
            Line::from(Span::styled(
                "How it works:",
                Style::default().fg(Color::Cyan),
            )),
            Line::from("  • Analyzes filename patterns using ML algorithms"),
            Line::from("  • Groups similar files (e.g., IMG_001.jpg, IMG_002.jpg)"),
            Line::from("  • Creates organized folders automatically"),
            Line::from(""),
            Line::from(Span::styled(
                " Press 'a' to begin analysis",
                Style::default().fg(Color::Green),
            )),
        ];

        let widget =
            Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(" Status "));
        f.render_widget(widget, area);
    }

    fn draw_analyzing_state(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use ratatui::{
            style::{Color, Style},
            widgets::{Block, Borders, Gauge},
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Analyzing Files "),
            )
            .gauge_style(Style::default().fg(Color::Yellow))
            .label(" Scanning and grouping files...")
            .percent(50);
        f.render_widget(gauge, area);
    }

    fn draw_review_state(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use ratatui::{
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Paragraph, Wrap},
        };

        let mut lines = vec![
            Line::from(Span::styled(
                format!(
                    "✓ Analysis Complete - Found {} file groups",
                    self.groups.len()
                ),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        let grouped_count = self.groups.iter().filter(|g| g.files.len() > 1).count();
        let single_count = self.groups.iter().filter(|g| g.files.len() == 1).count();

        lines.push(Line::from(vec![
            Span::raw(" Groups with 2+ files: "),
            Span::styled(
                grouped_count.to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw(" Single files: "),
            Span::styled(single_count.to_string(), Style::default().fg(Color::Yellow)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Preview of groups:",
            Style::default().fg(Color::Cyan),
        )));
        lines.push(Line::from(""));

        // Show groups with scroll support
        let multi_file_groups: Vec<_> = self.groups.iter().filter(|g| g.files.len() > 1).collect();
        let visible_groups = multi_file_groups.iter().skip(self.scroll_offset).take(8);

        for (i, group) in visible_groups.enumerate() {
            let folder_name = suggest_folder_name(group);
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}. ", i + 1 + self.scroll_offset),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    folder_name,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" ({} files, similarity: ", group.files.len())),
                Span::styled(
                    format!("{:.0}%", group.avg_similarity * 100.0),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(")"),
            ]));

            // Show first 2 files as examples
            for (j, file) in group.files.iter().take(2).enumerate() {
                lines.push(Line::from(format!(
                    "   {} {}",
                    if j == 0 { "├─" } else { "└─" },
                    file
                )));
            }
            if group.files.len() > 2 {
                lines.push(Line::from(format!(
                    "   ... and {} more",
                    group.files.len() - 2
                )));
            }
            lines.push(Line::from(""));
        }

        if multi_file_groups.len() > 8 + self.scroll_offset {
            lines.push(Line::from(Span::styled(
                format!(
                    "▼ {} more groups (use ↑↓ to scroll)",
                    multi_file_groups.len() - 8 - self.scroll_offset
                ),
                Style::default().fg(Color::Gray),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw(" Move skipped files to 'tyr-skip' folder: "),
            Span::styled(
                if self.move_skipped_to_folder {
                    "YES ✓"
                } else {
                    "NO"
                },
                if self.move_skipped_to_folder {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Red)
                },
            ),
        ]));

        let widget = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Review Groups "),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(widget, area);
    }

    fn draw_organizing_state(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use ratatui::{
            style::{Color, Style},
            widgets::{Block, Borders, Gauge},
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Organizing Files "),
            )
            .gauge_style(Style::default().fg(Color::Cyan))
            .label(" Moving files into folders...")
            .percent(75);
        f.render_widget(gauge, area);
    }

    fn draw_complete_state(
        &self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        result: &OrganizeResult,
    ) {
        use ratatui::{
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Paragraph, Wrap},
        };

        let mut lines = vec![
            Line::from(Span::styled(
                "✦ Organization Complete!",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw(" Folders created: "),
                Span::styled(
                    result.folders_created.to_string(),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw(" Files moved: "),
                Span::styled(
                    result.files_moved.to_string(),
                    Style::default()
                        .fg(Color::Cyan)
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
            Line::from(""),
        ];

        if !result.skipped_details.is_empty() {
            lines.push(Line::from(Span::styled(
                "Skipped Files:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            let visible_skipped = result
                .skipped_details
                .iter()
                .skip(self.scroll_offset)
                .take(8);

            for skip in visible_skipped {
                let (icon, reason_text) = match skip.reason {
                    SkipReason::SingleFile => ("", "No similar matches found"),
                    SkipReason::SystemFile => ("", "System file"),
                    SkipReason::AlreadyOrganized => ("✓", "Already organized"),
                };
                lines.push(Line::from(vec![
                    Span::raw(format!("  {} ", icon)),
                    Span::styled(&skip.filename, Style::default().fg(Color::Gray)),
                    Span::raw(" - "),
                    Span::styled(reason_text, Style::default().fg(Color::Yellow)),
                ]));
            }

            if result.skipped_details.len() > 8 + self.scroll_offset {
                lines.push(Line::from(Span::styled(
                    format!(
                        "▼ {} more (use ↑↓ to scroll)",
                        result.skipped_details.len() - 8 - self.scroll_offset
                    ),
                    Style::default().fg(Color::Gray),
                )));
            }
            lines.push(Line::from(""));
        }

        if !result.errors.is_empty() {
            lines.push(Line::from(Span::styled(
                "! Errors:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            for error in result.errors.iter().take(5) {
                lines.push(Line::from(format!("  • {}", error)));
            }
            if result.errors.len() > 5 {
                lines.push(Line::from(format!(
                    "  ... and {} more errors",
                    result.errors.len() - 5
                )));
            }
        }

        let widget = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" Results "))
            .wrap(Wrap { trim: false });
        f.render_widget(widget, area);
    }

    fn draw_controls(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use ratatui::{
            style::{Color, Style},
            widgets::{Block, Borders, Paragraph},
        };

        let controls = match &self.state {
            FilenameAppState::Ready => "'a' Analyze | 'q' Quit",
            FilenameAppState::ReviewGroups => {
                "'s' Start Organization | 'k' Toggle Skip Folder | ↑↓ Scroll | 'q' Quit"
            }
            FilenameAppState::Complete(_) => "'r' Reset | ↑↓ Scroll | 'q' Quit",
            FilenameAppState::Organizing => " Organizing... Please wait",
            _ => " Please wait...",
        };

        let widget = Paragraph::new(controls)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(widget, area);
    }

    pub fn get_logs(&self) -> Vec<String> {
        if let Ok(logs) = self.log_messages.lock() {
            logs.clone()
        } else {
            Vec::new()
        }
    }
    /// Auto-organize files without UI interaction
    /// Automatically analyzes (press 'a') then organizes (press 's')
    pub fn auto_organize(&mut self) -> io::Result<()> {
        // println!("Analyzing files...");

        // Step 1: Analyze files (equivalent to pressing 'a')
        self.analyze_files()?;

        // Display analysis results
        if matches!(self.state, FilenameAppState::ReviewGroups) {
            let grouped_count = self.groups.iter().filter(|g| g.files.len() > 1).count();
            let single_count = self.groups.iter().filter(|g| g.files.len() == 1).count();
            let total_files: usize = self.groups.iter().map(|g| g.files.len()).sum();

            println!("\n✓ Analysis Complete!");
            println!("   • Total files:        {}", total_files);
            println!("   • Groups with 2+ files: {}", grouped_count);
            println!("   • Single files:       {}", single_count);

            // Show preview of groups
            // if grouped_count > 0 {
            //     println!("\Preview of file groups:");
            //     let multi_file_groups: Vec<_> = self.groups.iter()
            //         .filter(|g| g.files.len() > 1)
            //         .collect();

            //     for (i, group) in multi_file_groups.iter().take(5).enumerate() {
            //         let folder_name = suggest_folder_name(group);
            //         println!("   {}. {} ({} files, similarity: {:.0}%)",
            //             i + 1,
            //             folder_name,
            //             group.files.len(),
            //             group.avg_similarity * 100.0
            //         );

            //         // Show first 2 files
            //         for (j, file) in group.files.iter().take(2).enumerate() {
            //             println!("      {} {}",
            //                 if j == 0 { "├─" } else { "└─" },
            //                 file
            //             );
            //         }
            //         if group.files.len() > 2 {
            //             println!("      ... and {} more", group.files.len() - 2);
            //         }
            //     }

            //     if multi_file_groups.len() > 5 {
            //         println!("   ... and {} more groups", multi_file_groups.len() - 5);
            //     }
            // }

            // println!("\nStarting organization...");
        }

        // Step 2: Start organization (equivalent to pressing 's')
        self.start_organization()?;

        // Display organization results
        if let FilenameAppState::Complete(result) = &self.state {
            println!("\n✦ Organization Complete!\n");
            println!("Summary:");
            println!("   • Folders created: {}", result.folders_created);
            println!("   • Files moved:     {}", result.files_moved);
            println!("   • Files skipped:   {}", result.files_skipped);

            // Show skipped files summary
            if !result.skipped_details.is_empty() {
                println!("\n[!]  Skipped Files:");

                let single_files = result.skipped_details.iter()
                    .filter(|s| matches!(s.reason, SkipReason::SingleFile))
                    .count();
                let system_files = result.skipped_details.iter()
                    .filter(|s| matches!(s.reason, SkipReason::SystemFile))
                    .count();
                let already_organized = result.skipped_details.iter()
                    .filter(|s| matches!(s.reason, SkipReason::AlreadyOrganized))
                    .count();

                if single_files > 0 {
                    println!("   • No similar matches: {}", single_files);
                }
                if system_files > 0 {
                    println!("   • System files:       {}", system_files);
                }
                if already_organized > 0 {
                    println!("   • Already organized:  {}", already_organized);
                }

                // Show first few skipped files
                if result.skipped_details.len() <= 5 {
                    println!("\n   Files:");
                    for skip in &result.skipped_details {
                        let reason_text = match skip.reason {
                            SkipReason::SingleFile => "no matches",
                            SkipReason::SystemFile => "system file",
                            SkipReason::AlreadyOrganized => "already organized",
                        };
                        println!("   • {} ({})", skip.filename, reason_text);
                    }
                } else {
                    println!("   (Use logs for full details)");
                }
            }

            // Show errors if any
            if !result.errors.is_empty() {
                println!("\nx Errors:");
                for (i, error) in result.errors.iter().enumerate().take(3) {
                    println!("   {}. {}", i + 1, error);
                }
                if result.errors.len() > 3 {
                    println!("   ... and {} more errors", result.errors.len() - 3);
                }
            }
        }

        Ok(())
    }
}
