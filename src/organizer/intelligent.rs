use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::io::{self, stdout};
use std::time::Duration;
use regex::Regex;
use rayon::prelude::*;
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


/// Configuration for intelligent grouping
#[derive(Debug, Clone)]
pub struct IntelligentConfig {
    pub max_lines_to_read: usize,
    pub min_cluster_size: usize,
    pub max_clusters: usize,
    pub filename_similarity_weight: f64,
    pub content_similarity_weight: f64,
    pub similarity_threshold: f64,
    pub max_iterations: usize,
}

impl Default for IntelligentConfig {
    fn default() -> Self {
        Self {
            max_lines_to_read: 100,
            min_cluster_size: 2,
            max_clusters: 20,
            filename_similarity_weight: 0.3,
            content_similarity_weight: 0.7,
            similarity_threshold: 0.65,
            max_iterations: 100,
        }
    }
}

/// Represents a file with its features for clustering
#[derive(Debug, Clone)]
pub struct FileFeatures {
    pub path: PathBuf,
    pub filename_vector: Vec<f64>,
    pub content_vector: Option<Vec<f64>>,
    pub is_text: bool,
}

/// Result of clustering operation
#[derive(Debug, Clone)]
pub struct ClusterResult {
    pub groups: Vec<FileGroup>,
}

/// A group of similar files
#[derive(Debug, Clone)]
pub struct FileGroup {
    pub files: Vec<PathBuf>,
    pub suggested_name: String,
    pub centroid: Vec<f64>,
}

/// TF-IDF vocabulary and document vectors
#[derive(Debug)]
struct TfIdfModel {
    vocabulary: Vec<String>,
    idf: Vec<f64>,
}

/// Progress callback type
type ProgressCallback = Box<dyn Fn(String) + Send>;

/// Main entry point for intelligent file organization
pub fn organize_files_intelligently(
    directory: &Path,
    config: &IntelligentConfig,
    progress_callback: Option<ProgressCallback>,
) -> Result<ClusterResult, io::Error> {
    let send_progress = |msg: String| {
        if let Some(ref callback) = progress_callback {
            callback(msg);
        }
    };

    send_progress("Scanning directory for files...".to_string());

    // Step 1: Traverse directory and collect files
    let files = collect_files(directory)?;

    if files.is_empty() {
        return Ok(ClusterResult { groups: Vec::new() });
    }

    send_progress(format!("Found {} files to analyze", files.len()));

    // Step 2: Extract features from each file
    send_progress("Extracting features...".to_string());
    let file_features = extract_features(&files, config)?;

    // Step 3: Perform clustering
    send_progress("Clustering files...".to_string());
    let clusters = perform_clustering(&file_features, config)?;

    // Step 4: Generate group names
    send_progress("Generating group names...".to_string());
    let groups = generate_group_names(clusters, &file_features);

    send_progress(format!("✓ Created {} groups", groups.len()));

    Ok(ClusterResult { groups })
}

/// Recursively collect all files from directory (excluding hidden files and directories)
fn collect_files(directory: &Path) -> Result<Vec<PathBuf>, io::Error> {
    let mut files = Vec::new();

    if directory.is_dir() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();

            // Skip hidden files/directories
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    continue;
                }
            }

            if path.is_file() {
                files.push(path);
            }
        }
    }

    Ok(files)
}

/// Extract features from files
fn extract_features(
    files: &[PathBuf],
    config: &IntelligentConfig,
) -> Result<Vec<FileFeatures>, io::Error> {
    // First, identify text files and read their content
    let file_contents: Vec<(PathBuf, Option<String>)> = files
        .par_iter()
        .map(|path| {
            let content = if is_text_file(path) {
                read_file_lines(path, config.max_lines_to_read).ok()
            } else {
                None
            };
            (path.clone(), content)
        })
        .collect();

    // Build TF-IDF model from text files
    let text_contents: Vec<String> = file_contents
        .iter()
        .filter_map(|(_, content)| content.clone())
        .collect();

    let tfidf_model = if !text_contents.is_empty() {
        Some(build_tfidf_model(&text_contents))
    } else {
        None
    };

    // Extract features for each file
    let features: Vec<FileFeatures> = file_contents
        .into_iter()
        .map(|(path, content)| {
            let filename_vector = extract_filename_features(&path);
            let content_vector = if let (Some(ref model), Some(ref text)) = (&tfidf_model, &content) {
                Some(compute_tfidf_vector(text, model))
            } else {
                None
            };

            FileFeatures {
                path,
                filename_vector,
                content_vector,
                is_text: content.is_some(),
            }
        })
        .collect();

    Ok(features)
}

/// Check if file is likely a text file based on extension
fn is_text_file(path: &Path) -> bool {
    let text_extensions = [
        "txt", "md", "rs", "py", "js", "ts", "jsx", "tsx", "html", "css",
        "json", "xml", "yaml", "yml", "toml", "ini", "cfg", "conf",
        "c", "cpp", "h", "hpp", "java", "go", "php", "rb", "swift",
        "kt", "scala", "sh", "bat", "ps1", "r", "lua", "vim", "sql",
        "csv", "log", "tex", "rtf",
    ];

    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        return text_extensions.contains(&ext_str.as_str());
    }

    false
}

/// Read first N lines from a file
fn read_file_lines(path: &Path, max_lines: usize) -> Result<String, io::Error> {
    let content = fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().take(max_lines).collect();
    Ok(lines.join("\n"))
}

/// Extract features from filename using character n-grams
fn extract_filename_features(path: &Path) -> Vec<f64> {
    let filename = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    // Simple character-level features
    let mut features = Vec::new();

    // Character frequency (a-z)
    let mut char_freq = vec![0.0; 26];
    for c in filename.chars() {
        if c.is_ascii_alphabetic() {
            let idx = (c as u8 - b'a') as usize;
            if idx < 26 {
                char_freq[idx] += 1.0;
            }
        }
    }

    // Normalize
    let total: f64 = char_freq.iter().sum();
    if total > 0.0 {
        for freq in &mut char_freq {
            *freq /= total;
        }
    }

    features.extend(char_freq);

    // Add length feature (normalized)
    features.push((filename.len() as f64).min(50.0) / 50.0);

    // Add digit ratio
    let digit_count = filename.chars().filter(|c| c.is_ascii_digit()).count();
    features.push(digit_count as f64 / filename.len().max(1) as f64);

    // Add special char ratio
    let special_count = filename.chars().filter(|c| !c.is_alphanumeric()).count();
    features.push(special_count as f64 / filename.len().max(1) as f64);

    features
}

/// Build TF-IDF model from documents
fn build_tfidf_model(documents: &[String]) -> TfIdfModel {
    // Build vocabulary
    let mut word_set = HashSet::new();

    for doc in documents {
        let words = preprocess_text(doc);
        word_set.extend(words);
    }

    let vocabulary: Vec<String> = word_set.into_iter().collect();
    let vocab_map: HashMap<&str, usize> = vocabulary
        .iter()
        .enumerate()
        .map(|(i, w)| (w.as_str(), i))
        .collect();

    // Compute IDF
    let mut doc_freq = vec![0; vocabulary.len()];

    for doc in documents {
        let words = preprocess_text(doc);
        let mut seen = HashSet::new();

        for word in words {
            if seen.insert(word.clone()) {
                if let Some(&idx) = vocab_map.get(word.as_str()) {
                    doc_freq[idx] += 1;
                }
            }
        }
    }

    let n_docs = documents.len() as f64;
    let idf: Vec<f64> = doc_freq
        .iter()
        .map(|&df| {
            if df > 0 {
                (n_docs / df as f64).ln() + 1.0
            } else {
                1.0
            }
        })
        .collect();

    TfIdfModel { vocabulary, idf }
}

/// Preprocess text into tokens
fn preprocess_text(text: &str) -> Vec<String> {
    let re = Regex::new(r"[a-zA-Z0-9]+").unwrap();

    re.find_iter(&text.to_lowercase())
        .map(|m| m.as_str().to_string())
        .filter(|w| w.len() >= 2) // Filter very short words
        .collect()
}

/// Compute TF-IDF vector for a document
fn compute_tfidf_vector(text: &str, model: &TfIdfModel) -> Vec<f64> {
    let words = preprocess_text(text);
    let vocab_map: HashMap<&str, usize> = model
        .vocabulary
        .iter()
        .enumerate()
        .map(|(i, w)| (w.as_str(), i))
        .collect();

    // Compute term frequency
    let mut tf = vec![0.0; model.vocabulary.len()];
    for word in &words {
        if let Some(&idx) = vocab_map.get(word.as_str()) {
            tf[idx] += 1.0;
        }
    }

    // Normalize TF
    let total: f64 = tf.iter().sum();
    if total > 0.0 {
        for freq in &mut tf {
            *freq /= total;
        }
    }

    // Apply IDF
    let tfidf: Vec<f64> = tf
        .iter()
        .zip(&model.idf)
        .map(|(t, i)| t * i)
        .collect();

    tfidf
}

/// Perform K-means clustering
fn perform_clustering(
    features: &[FileFeatures],
    config: &IntelligentConfig,
) -> Result<Vec<Vec<usize>>, io::Error> {
    if features.is_empty() {
        return Ok(Vec::new());
    }

    // Combine filename and content features
    let combined_vectors: Vec<Vec<f64>> = features
        .iter()
        .map(|f| combine_feature_vectors(f, config))
        .collect();

    // Determine optimal number of clusters
    let k = determine_k(&combined_vectors, config);

    if k == 0 {
        return Ok(vec![features.iter().enumerate().map(|(i, _)| i).collect()]);
    }

    // Run K-means
    let assignments = kmeans(&combined_vectors, k, config.max_iterations);

    // Group indices by cluster
    let mut clusters: Vec<Vec<usize>> = vec![Vec::new(); k];
    for (idx, &cluster_id) in assignments.iter().enumerate() {
        clusters[cluster_id].push(idx);
    }

    // Filter out small clusters
    clusters.retain(|c| c.len() >= config.min_cluster_size);

    Ok(clusters)
}

/// Combine filename and content feature vectors
fn combine_feature_vectors(features: &FileFeatures, config: &IntelligentConfig) -> Vec<f64> {
    let mut combined = features.filename_vector.clone();

    // Scale filename features
    for val in &mut combined {
        *val *= config.filename_similarity_weight;
    }

    // Add content features if available
    if let Some(ref content_vec) = features.content_vector {
        let scaled_content: Vec<f64> = content_vec
            .iter()
            .map(|&v| v * config.content_similarity_weight)
            .collect();
        combined.extend(scaled_content);
    } else {
        // Pad with zeros if no content
        combined.extend(vec![0.0; 100]); // Arbitrary padding
    }

    combined
}

/// Determine optimal number of clusters using heuristic
fn determine_k(vectors: &[Vec<f64>], config: &IntelligentConfig) -> usize {
    let n = vectors.len();

    // Simple heuristic: sqrt(n/2)
    let k = ((n as f64 / 2.0).sqrt().ceil() as usize)
        .max(2)
        .min(config.max_clusters)
        .min(n);

    k
}

/// K-means clustering algorithm
fn kmeans(vectors: &[Vec<f64>], k: usize, max_iterations: usize) -> Vec<usize> {
    let n = vectors.len();
    if n == 0 || k == 0 {
        return Vec::new();
    }

    let dim = vectors[0].len();

    // Initialize centroids randomly (use first k points)
    let mut centroids: Vec<Vec<f64>> = vectors.iter().take(k).cloned().collect();
    let mut assignments = vec![0; n];

    for _ in 0..max_iterations {
        let mut changed = false;

        // Assignment step
        for (i, vector) in vectors.iter().enumerate() {
            let mut min_dist = f64::MAX;
            let mut best_cluster = 0;

            for (j, centroid) in centroids.iter().enumerate() {
                let dist = euclidean_distance(vector, centroid);
                if dist < min_dist {
                    min_dist = dist;
                    best_cluster = j;
                }
            }

            if assignments[i] != best_cluster {
                assignments[i] = best_cluster;
                changed = true;
            }
        }

        if !changed {
            break;
        }

        // Update step
        let mut new_centroids = vec![vec![0.0; dim]; k];
        let mut counts = vec![0; k];

        for (i, vector) in vectors.iter().enumerate() {
            let cluster = assignments[i];
            counts[cluster] += 1;
            for (j, &val) in vector.iter().enumerate() {
                new_centroids[cluster][j] += val;
            }
        }

        for (cluster, count) in counts.iter().enumerate() {
            if *count > 0 {
                for val in &mut new_centroids[cluster] {
                    *val /= *count as f64;
                }
            }
        }

        centroids = new_centroids;
    }

    assignments
}

/// Compute Euclidean distance between two vectors
fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// Generate meaningful names for file groups
fn generate_group_names(clusters: Vec<Vec<usize>>, features: &[FileFeatures]) -> Vec<FileGroup> {
    clusters
        .into_iter()
        .map(|cluster_indices| {
            let files: Vec<PathBuf> = cluster_indices
                .iter()
                .map(|&i| features[i].path.clone())
                .collect();

            // Generate name from common filename patterns
            let suggested_name = generate_name_from_files(&files);

            // Compute centroid
            let centroid = compute_centroid(&cluster_indices, features);

            FileGroup {
                files,
                suggested_name,
                centroid,
            }
        })
        .collect()
}

/// Generate a meaningful name from a group of files
fn generate_name_from_files(files: &[PathBuf]) -> String {
    if files.is_empty() {
        return "Group".to_string();
    }

    // Extract all filenames
    let filenames: Vec<String> = files
        .iter()
        .filter_map(|p| {
            p.file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
        })
        .collect();

    // Find common prefix
    let common_prefix = find_common_prefix(&filenames);

    if !common_prefix.is_empty() && common_prefix.len() >= 3 {
        // Capitalize first letter
        let mut chars = common_prefix.chars();
        return chars
            .next()
            .unwrap()
            .to_uppercase()
            .chain(chars)
            .collect::<String>()
            .trim_end_matches(|c: char| !c.is_alphanumeric())
            .to_string();
    }

    // Find most common words
    let words = extract_common_words(&filenames);
    if !words.is_empty() {
        let mut name = words[0].clone();
        name.get_mut(0..1).map(|s| s.make_ascii_uppercase());
        return name;
    }

    // Fallback: use extension or "Mixed"
    if let Some(ext) = files[0].extension() {
        format!("{}_Files", ext.to_string_lossy().to_uppercase())
    } else {
        "Mixed_Files".to_string()
    }
}

/// Find common prefix among strings
fn find_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }

    let first = &strings[0];
    let mut prefix = String::new();

    for (i, c) in first.chars().enumerate() {
        if strings.iter().all(|s| s.chars().nth(i) == Some(c)) {
            prefix.push(c);
        } else {
            break;
        }
    }

    prefix
}

/// Extract most common words from filenames
fn extract_common_words(filenames: &[String]) -> Vec<String> {
    let mut word_counts: HashMap<String, usize> = HashMap::new();
    let re = Regex::new(r"[a-zA-Z]{3,}").unwrap();

    for filename in filenames {
        for word in re.find_iter(filename) {
            let word_str = word.as_str().to_lowercase();
            if word_str.len() >= 3 {
                *word_counts.entry(word_str).or_insert(0) += 1;
            }
        }
    }

    let mut words: Vec<(String, usize)> = word_counts.into_iter().collect();
    words.sort_by(|a, b| b.1.cmp(&a.1));

    words.into_iter().map(|(w, _)| w).take(1).collect()
}

/// Compute centroid of a cluster
fn compute_centroid(indices: &[usize], features: &[FileFeatures]) -> Vec<f64> {
    if indices.is_empty() {
        return Vec::new();
    }

    let first_vec = &features[indices[0]].filename_vector;
    let dim = first_vec.len();
    let mut centroid = vec![0.0; dim];

    for &idx in indices {
        for (i, &val) in features[idx].filename_vector.iter().enumerate() {
            centroid[i] += val;
        }
    }

    for val in &mut centroid {
        *val /= indices.len() as f64;
    }

    centroid
}

/// Move files into their organized groups
pub fn move_files_to_groups(
    base_path: &Path,
    result: &ClusterResult,
    dry_run: bool,
) -> Result<Vec<String>, io::Error> {
    let mut log = Vec::new();

    for (_i, group) in result.groups.iter().enumerate() {
        // Sanitize group name for directory
        let dir_name = sanitize_dirname(&group.suggested_name);
        let group_dir = base_path.join(&dir_name);

        if !dry_run {
            // Create directory if it doesn't exist
            fs::create_dir_all(&group_dir)?;
            log.push(format!("Created directory: {}", group_dir.display()));
        } else {
            log.push(format!("[DRY RUN] Would create: {}", group_dir.display()));
        }

        // Move each file to the group directory
        for file_path in &group.files {
            if let Some(filename) = file_path.file_name() {
                let dest_path = group_dir.join(filename);

                // Skip if source and dest are the same
                if file_path == &dest_path {
                    continue;
                }

                if !dry_run {
                    // Handle file name conflicts
                    let final_dest = handle_conflict(&dest_path)?;
                    fs::rename(file_path, &final_dest)?;
                    log.push(format!(
                        "  Moved: {} → {}",
                        file_path.display(),
                        final_dest.display()
                    ));
                } else {
                    log.push(format!(
                        "  [DRY RUN] Would move: {} → {}",
                        file_path.display(),
                        dest_path.display()
                    ));
                }
            }
        }
    }

    Ok(log)
}

/// Sanitize directory name
fn sanitize_dirname(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Handle file name conflicts by appending numbers
fn handle_conflict(path: &Path) -> Result<PathBuf, io::Error> {
    if !path.exists() {
        return Ok(path.to_path_buf());
    }

    let parent = path.parent().unwrap_or(Path::new("."));
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    for i in 1..1000 {
        let new_name = format!("{}_{}{}", stem, i, ext);
        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return Ok(new_path);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Too many file conflicts",
    ))
}

// TUI
#[derive(Debug)]
enum AppState {
    Ready,
    Analyzing,
    Complete(ClusterResult),
    Moving,
    Moved(Vec<String>),
}

pub struct IntelligentTuiApp {
    config: IntelligentConfig,
    base_path: PathBuf,
    state: AppState,
    progress_message: String,
    log_messages: Vec<String>,
}

impl IntelligentTuiApp {
    pub fn new(config: IntelligentConfig, base_path: PathBuf) -> Self {
        Self {
            config,
            base_path,
            state: AppState::Ready,
            progress_message: String::new(),
            log_messages: Vec::new(),
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
                                self.start_analysis()?;
                            }
                        }
                        KeyCode::Char('m') => {
                            if let AppState::Complete(result) = &self.state {
                                let result_clone = result.clone();
                                self.move_files(&result_clone)?;
                            }
                        }
                        KeyCode::Char('d') => {
                            if let AppState::Complete(result) = &self.state {
                                let result_clone = result.clone();
                                self.dry_run_move(&result_clone)?;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn start_analysis(&mut self) -> io::Result<()> {
        self.state = AppState::Analyzing;
        self.progress_message = "Initializing ML clustering...".to_string();
        self.log_messages.clear();

        match organize_files_intelligently(&self.base_path, &self.config, None) {
            Ok(result) => {
                self.state = AppState::Complete(result);
                Ok(())
            }
            Err(e) => {
                self.state = AppState::Ready;
                self.progress_message = format!("Error: {}", e);
                Err(e)
            }
        }
    }

    fn dry_run_move(&mut self, result: &ClusterResult) -> io::Result<()> {
        self.log_messages.clear();
        match move_files_to_groups(&self.base_path, result, true) {
            Ok(log) => {
                self.log_messages = log;
            }
            Err(e) => {
                self.log_messages.push(format!("Error: {}", e));
            }
        }
        Ok(())
    }

    fn move_files(&mut self, result: &ClusterResult) -> io::Result<()> {
        self.state = AppState::Moving;
        self.log_messages.clear();

        match move_files_to_groups(&self.base_path, result, false) {
            Ok(log) => {
                self.state = AppState::Moved(log.clone());
                self.log_messages = log;
                Ok(())
            }
            Err(e) => {
                self.state = AppState::Complete(ClusterResult {
                    groups: result.groups.clone(),
                });
                self.log_messages.push(format!("Error: {}", e));
                Err(e)
            }
        }
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
        let title = Paragraph::new(" Tyr - Intelligent ML-Based File Organizer")
            .style(
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Main content
        match &self.state {
            AppState::Ready => self.draw_ready_state(f, chunks[1]),
            AppState::Analyzing => self.draw_analyzing_state(f, chunks[1]),
            AppState::Complete(result) => self.draw_complete_state(f, chunks[1], result),
            AppState::Moving => self.draw_moving_state(f, chunks[1]),
            AppState::Moved(_) => self.draw_moved_state(f, chunks[1]),
        }

        // Info panel
        self.draw_info_panel(f, chunks[2]);

        // Controls
        self.draw_controls(f, chunks[3]);
    }

    fn draw_ready_state(&self, f: &mut ratatui::Frame, area: Rect) {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                " Ready to Analyze Files with ML",
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
                "How Intelligent Grouping Works:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(" Text Files:"),
            Line::from(format!(
                "     • Reads first {} lines of content",
                self.config.max_lines_to_read
            )),
            Line::from("     • Builds TF-IDF vectors for semantic analysis"),
            Line::from("     • Combines with filename similarity"),
            Line::from(""),
            Line::from(" All Files:"),
            Line::from("     • Extracts filename patterns and features"),
            Line::from("     • Uses K-means clustering algorithm"),
            Line::from(format!("     • Creates up to {} intelligent groups", self.config.max_clusters)),
            Line::from("     • Generates meaningful group names"),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Weights: Filename "),
                Span::styled(
                    format!("{}%", (self.config.filename_similarity_weight * 100.0) as u8),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(" | Content "),
                Span::styled(
                    format!("{}%", (self.config.content_similarity_weight * 100.0) as u8),
                    Style::default().fg(Color::Magenta),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                " Press 's' to start intelligent analysis",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            )),
        ];

        let widget =
            Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(" Status "));
        f.render_widget(widget, area);
    }

    fn draw_analyzing_state(&self, f: &mut ratatui::Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(area);

        // Progress gauge
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(" Progress "))
            .gauge_style(Style::default().fg(Color::Magenta))
            .label(" Running ML clustering algorithm...")
            .percent(50);
        f.render_widget(gauge, chunks[0]);

        // Current step
        let step_text = vec![
            Line::from(vec![
                Span::styled("", Style::default().fg(Color::Yellow)),
                Span::raw(&self.progress_message),
            ]),
        ];
        let step_widget = Paragraph::new(step_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Current Step "),
        );
        f.render_widget(step_widget, chunks[1]);

        // Processing info
        let info_text = vec![
            Line::from(""),
            Line::from(" This may take a moment depending on:"),
            Line::from("   • Number of files to analyze"),
            Line::from("   • Size of text files"),
            Line::from("   • Complexity of filename patterns"),
        ];
        let info_widget =
            Paragraph::new(info_text).block(Block::default().borders(Borders::ALL));
        f.render_widget(info_widget, chunks[2]);
    }

    fn draw_complete_state(&self, f: &mut ratatui::Frame, area: Rect, result: &ClusterResult) {
        let total_files: usize = result.groups.iter().map(|g| g.files.len()).sum();

        let mut lines = vec![
            Line::from(Span::styled(
                "✦ Analysis Complete!",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw(" Total files analyzed: "),
                Span::styled(
                    total_files.to_string(),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw(" Groups created: "),
                Span::styled(
                    result.groups.len().to_string(),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                " Discovered Groups:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        // Sort groups by file count
        let mut sorted_groups: Vec<_> = result.groups.iter().collect();
        sorted_groups.sort_by(|a, b| b.files.len().cmp(&a.files.len()));

                        for (_i, group) in sorted_groups.iter().enumerate().take(12) {
            // let icon = get_group_icon(&group.suggested_name);

            // Truncate long names
            let name = if group.suggested_name.len() > 25 {
                format!("{}...", &group.suggested_name[..22])
            } else {
                group.suggested_name.clone()
            };

            lines.push(Line::from(vec![
                // Span::raw(format!("  {} ", icon)),
                Span::styled(
                    format!("{:28}", name),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(" → "),
                Span::styled(
                    format!("{} files", group.files.len()),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        if sorted_groups.len() > 12 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("... and {} more groups", sorted_groups.len() - 12),
                Style::default().fg(Color::Gray),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Press 'm' to move files | 'd' for dry run preview",
            Style::default().fg(Color::Yellow),
        )));

        // Show dry run logs if available
        if !self.log_messages.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " Preview:",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            for msg in self.log_messages.iter().take(5) {
                lines.push(Line::from(format!("   {}", msg)));
            }
            if self.log_messages.len() > 5 {
                lines.push(Line::from(format!("   ... and {} more operations", self.log_messages.len() - 5)));
            }
        }

        let widget =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Results "));
        f.render_widget(widget, area);
    }

    fn draw_moving_state(&self, f: &mut ratatui::Frame, area: Rect) {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                " Moving Files to Groups...",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(" Please wait while files are being organized..."),
            Line::from(""),
            Line::from(" This process will:"),
            Line::from("   • Create group directories"),
            Line::from("   • Move files to respective groups"),
            Line::from("   • Handle naming conflicts automatically"),
        ];

        let widget =
            Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(" Status "));
        f.render_widget(widget, area);
    }

    fn draw_moved_state(&self, f: &mut ratatui::Frame, area: Rect) {
        let mut lines = vec![
            Line::from(Span::styled(
                "✓ Files Successfully Moved! ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw(" Operations completed: "),
                Span::styled(
                    self.log_messages.len().to_string(),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                " Recent Operations:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        // Show last 15 operations
        for msg in self.log_messages.iter().rev().take(15) {
            lines.push(Line::from(format!("   {}", msg)));
        }

        if self.log_messages.len() > 15 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("... and {} more operations", self.log_messages.len() - 15),
                Style::default().fg(Color::Gray),
            )));
        }

        let widget =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Complete "));
        f.render_widget(widget, area);
    }

fn draw_info_panel(&self, f: &mut ratatui::Frame, area: Rect) {
    let info = match &self.state {
        AppState::Ready => vec![
            ListItem::new(Line::from(vec![
                Span::styled("", Style::default().fg(Color::Yellow)),
                Span::raw("Intelligent grouping uses ML to find patterns"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("", Style::default().fg(Color::Green)),
                Span::raw("Faster and smarter than manual organization"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("", Style::default().fg(Color::Cyan)),
                Span::raw("Groups files by both name and content similarity"),
            ])),
        ],
        AppState::Analyzing => vec![
            ListItem::new(Line::from(vec![
                Span::styled("", Style::default().fg(Color::Magenta)),
                Span::raw("Building TF-IDF vectors from file content..."),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("", Style::default().fg(Color::Cyan)),
                Span::raw("Extracting filename features and patterns..."),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("", Style::default().fg(Color::Green)),
                Span::raw("Running K-means clustering algorithm..."),
            ])),
        ],
        AppState::Complete(result) => {
            let total_files: usize = result.groups.iter().map(|g| g.files.len()).sum();
            let avg_group_size = if !result.groups.is_empty() {
                total_files / result.groups.len()
            } else {
                0
            };

            vec![
                ListItem::new(Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(Color::Green)),
                    Span::raw(format!(
                        "Successfully grouped {} files into {} categories",
                        total_files,
                        result.groups.len()
                    )),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled("", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("Average group size: {} files", avg_group_size)),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled("", Style::default().fg(Color::Magenta)),
                    Span::raw("Smart group names generated from file patterns"),
                ])),
            ]
        }
        AppState::Moving => vec![
            ListItem::new(Line::from(vec![
                Span::styled("", Style::default().fg(Color::Yellow)),
                Span::raw("Creating directories for each group..."),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("", Style::default().fg(Color::Cyan)),
                Span::raw("Moving files to their designated groups..."),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("", Style::default().fg(Color::Green)),
                Span::raw("Handling conflicts and organizing structure..."),
            ])),
        ],
        AppState::Moved(log) => {
            vec![
                ListItem::new(Line::from(vec![
                    Span::styled("", Style::default().fg(Color::Green)),
                    Span::raw(format!("All {} operations completed successfully", log.len())),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled("", Style::default().fg(Color::Cyan)),
                    Span::raw("Files organized into categorized directories"),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled("", Style::default().fg(Color::Magenta)),
                    Span::raw("Your files are now perfectly organized!"),
                ])),
            ]
        }
    };

    let list = List::new(info).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Information "),
    );
    f.render_widget(list, area);
}

    fn draw_controls(&self, f: &mut ratatui::Frame, area: Rect) {
        let controls = match &self.state {
            AppState::Ready => " 's' Start Analysis | 'q' Quit",
            AppState::Analyzing => " Analyzing... Please wait",
            AppState::Complete(_) => " 'm' Move Files | 'd' Dry Run | 'q' Quit",
            AppState::Moving => " Moving files... Please wait",
            AppState::Moved(_) => " 'q' Quit",
        };

        let widget = Paragraph::new(controls)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(widget, area);
    }

    /// Auto-analyze files without UI interaction
    pub fn auto_analyze(&mut self) -> io::Result<()> {
        // Redirect output to log instead of stdout
        self.log_messages.push("Starting intelligent ML-based analysis...".to_string());

        // Start analysis
        self.start_analysis()?;

        // Display results
        if let AppState::Complete(result) = &self.state {
            let total_files: usize = result.groups.iter().map(|g| g.files.len()).sum();

            self.log_messages.push("\n✦ Analysis Complete! \n".to_string());
            self.log_messages.push("Summary:".to_string());
            self.log_messages.push(format!("   • Total files analyzed: {}", total_files));
            self.log_messages.push(format!("   • Groups created:       {}", result.groups.len()));

            if !result.groups.is_empty() {
                self.log_messages.push("\n Discovered Groups:\n".to_string());

                // Sort groups by file count
                let mut sorted_groups: Vec<_> = result.groups.iter().collect();
                sorted_groups.sort_by(|a, b| b.files.len().cmp(&a.files.len()));

                for (_i, group) in sorted_groups.iter().enumerate() {
                    // let icon = get_group_icon(&group.suggested_name);
                    self.log_messages.push(format!(
                        "  {:30} → {} files",
                        // icon,
                        group.suggested_name,
                        group.files.len()
                    ));
                }

                // Calculate statistics
                let avg_group_size = total_files / result.groups.len();
                self.log_messages.push("\n Statistics:".to_string());
                self.log_messages.push(format!("   • Average group size: {} files", avg_group_size));
                self.log_messages.push(format!(
                    "   • Largest group:      {} files ({})",
                    sorted_groups[0].files.len(),
                    sorted_groups[0].suggested_name
                ));
                self.log_messages.push(format!(
                    "   • Smallest group:     {} files ({})",
                    sorted_groups.last().unwrap().files.len(),
                    sorted_groups.last().unwrap().suggested_name
                ));
            }

            // Print all logs to stdout after analysis
            for msg in &self.log_messages {
                println!("{}", msg);
            }
        }

        Ok(())
    }
}

// Get an icon for a group based on its name
// fn get_group_icon(name: &str) -> &'static str {
//     let name_lower = name.to_lowercase();

//     if name_lower.contains("image") || name_lower.contains("photo") || name_lower.contains("picture") {
//         "🖼️"
//     } else if name_lower.contains("video") || name_lower.contains("movie") || name_lower.contains("film") {
//         "🎬"
//     } else if name_lower.contains("audio") || name_lower.contains("music") || name_lower.contains("sound") {
//         "🎵"
//     } else if name_lower.contains("document") || name_lower.contains("doc") || name_lower.contains("text") {
//         "📄"
//     } else if name_lower.contains("code") || name_lower.contains("script") || name_lower.contains("program") {
//         "💻"
//     } else if name_lower.contains("archive") || name_lower.contains("zip") || name_lower.contains("compressed") {
//         "📦"
//     } else if name_lower.contains("data") || name_lower.contains("database") || name_lower.contains("json") {
//         "💾"
//     } else if name_lower.contains("spreadsheet") || name_lower.contains("excel") || name_lower.contains("csv") {
//         "📊"
//     } else if name_lower.contains("presentation") || name_lower.contains("slide") {
//         "📽️"
//     } else if name_lower.contains("design") || name_lower.contains("graphic") {
//         "🎨"
//     } else if name_lower.contains("3d") || name_lower.contains("model") {
//         "🗿"
//     } else if name_lower.contains("font") {
//         "🔤"
//     } else if name_lower.contains("report") {
//         "📋"
//     } else if name_lower.contains("project") {
//         "📁"
//     } else {
//         "📂"
//     }
// }
