//  Copyright (C) 2026 Dawood Khan
//  SPDX-License-Identifier: GPL-3.0-or-later

// Maintainer Dawood (Nurysso) contact - nurysso [at] proton.me

/*
  This file is responsible for executing intelligent file organizing for
  tyr(https:github.com/Nurysso/tyr) project
*/

use ratatui::crossterm::event::KeyCode;
use ratatui::style::Color;
use ratatui::{layout::Constraint, layout::Rect, text::Line, Frame};
use rayon::prelude::*;
use regex::Regex;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::organizer::tui::{self, App, Flow, Job, Progress};
use crate::organizer::utils::{is_hidden, LazyDirManager};
use serde::{Deserialize, Serialize};

/// Configuration for intelligent grouping
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IntelligentConfig {
    #[serde(default = "default_max_lines_to_read")]
    pub max_lines_to_read: usize,
    #[serde(default = "default_min_cluster_size")]
    pub min_cluster_size: usize,
    #[serde(default = "default_max_clusters")]
    pub max_clusters: usize,
    #[serde(default = "default_filename_weight")]
    pub filename_similarity_weight: f64,
    #[serde(default = "default_content_weight")]
    pub content_similarity_weight: f64,
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f64,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,

    #[serde(default)]
    pub skip_patterns: Vec<String>,
    #[serde(default = "default_true")]
    pub skip_hidden_files: bool,
}

fn default_max_lines_to_read() -> usize {
    100
}
fn default_min_cluster_size() -> usize {
    2
}
fn default_max_clusters() -> usize {
    20
}
fn default_filename_weight() -> f64 {
    0.3
}
fn default_content_weight() -> f64 {
    0.7
}
fn default_similarity_threshold() -> f64 {
    0.65
}
fn default_max_iterations() -> usize {
    100
}
fn default_true() -> bool {
    true
}

impl Default for IntelligentConfig {
    fn default() -> Self {
        Self {
            max_lines_to_read: default_max_lines_to_read(),
            min_cluster_size: default_min_cluster_size(),
            max_clusters: default_max_clusters(),
            filename_similarity_weight: default_filename_weight(),
            content_similarity_weight: default_content_weight(),
            similarity_threshold: default_similarity_threshold(),
            max_iterations: default_max_iterations(),
            skip_patterns: Vec::new(),
            skip_hidden_files: true,
        }
    }
}

/// Represents a file with its features for clustering
#[derive(Debug, Clone)]
pub struct FileFeatures {
    pub path: PathBuf,
    pub filename_vector: Vec<f64>,
    pub content_vector: Option<Vec<f64>>,
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
}

/// TF-IDF vocabulary and document vectors
#[derive(Debug)]
struct TfIdfModel {
    vocabulary: Vec<String>,
    vocab_map: HashMap<String, usize>,
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
    let files = collect_files(directory, config)?;

    if files.is_empty() {
        return Ok(ClusterResult { groups: Vec::new() });
    }

    send_progress(format!("Found {} files to analyze", files.len()));
    send_progress("Extracting features...".to_string());
    let file_features = extract_features(&files, config)?;

    send_progress("Clustering files...".to_string());
    let clusters = perform_clustering(&file_features, config)?;

    send_progress("Generating group names...".to_string());
    let groups = generate_group_names(clusters, &file_features);

    send_progress(format!("Created {} groups", groups.len()));
    Ok(ClusterResult { groups })
}

fn collect_files(directory: &Path, config: &IntelligentConfig) -> Result<Vec<PathBuf>, io::Error> {
    let mut files = Vec::new();
    let mut stack = vec![directory.to_path_buf()];

    while let Some(dir) = stack.pop() {
        if !dir.is_dir() {
            continue;
        }

        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();

            if config.skip_hidden_files && is_hidden(&path) {
                continue;
            }

            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if config
                    .skip_patterns
                    .iter()
                    .any(|p| name.contains(p.as_str()))
                {
                    continue;
                }
            }

            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }

    Ok(files)
}

/// Filenames (without extension requirements) that are conventionally
/// plain text even though they have no/unusual extension.
const KNOWN_TEXT_FILENAMES: &[&str] = &[
    "dockerfile",
    "makefile",
    "readme",
    "license",
    "licence",
    "changelog",
    "authors",
    "contributing",
    "gitignore",
    "gitattributes",
    "editorconfig",
    "cargo.lock",
    "notice",
];

/// Check if file is likely a text file based on extension or well-known name
fn is_text_file(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        let name_lower = name.to_string_lossy().to_lowercase();
        let stripped = name_lower.trim_start_matches('.');
        if KNOWN_TEXT_FILENAMES.contains(&name_lower.as_str())
            || KNOWN_TEXT_FILENAMES.contains(&stripped)
        {
            return true;
        }
    }

    let text_extensions = [
        "txt", "md", "rs", "py", "js", "ts", "jsx", "tsx", "html", "css", "json", "xml", "yaml",
        "yml", "toml", "ini", "cfg", "conf", "c", "cpp", "h", "hpp", "java", "go", "php", "rb",
        "swift", "kt", "scala", "sh", "bat", "ps1", "r", "lua", "vim", "sql", "csv", "log", "tex",
        "rtf",
    ];

    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        return text_extensions.contains(&ext_str.as_str());
    }

    false
}

/// Read first N lines from a file without loading the whole file into memory
fn read_first_n_file_lines(path: &Path, max_lines: usize) -> Result<String, io::Error> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut lines = Vec::with_capacity(max_lines);
    for line in reader.lines().take(max_lines) {
        match line {
            Ok(l) => lines.push(l),
            Err(_) => break,
        }
    }

    Ok(lines.join("\n"))
}

/// Extract features from filename using character n-grams
fn extract_filename_features(path: &Path) -> Vec<f64> {
    let filename = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    let mut features = Vec::new();

    let mut char_freq = vec![0.0; 26];
    for c in filename.chars() {
        if c.is_ascii_alphabetic() {
            let idx = (c as u8 - b'a') as usize;
            if idx < 26 {
                char_freq[idx] += 1.0;
            }
        }
    }

    let total: f64 = char_freq.iter().sum();
    if total > 0.0 {
        for freq in &mut char_freq {
            *freq /= total;
        }
    }

    features.extend(char_freq);
    features.push((filename.len() as f64).min(50.0) / 50.0);

    let digit_count = filename.chars().filter(|c| c.is_ascii_digit()).count();
    features.push(digit_count as f64 / filename.len().max(1) as f64);

    let special_count = filename.chars().filter(|c| !c.is_alphanumeric()).count();
    features.push(special_count as f64 / filename.len().max(1) as f64);

    features
}

fn word_regex() -> &'static Regex {
    static WORD_RE: OnceLock<Regex> = OnceLock::new();
    WORD_RE.get_or_init(|| Regex::new(r"[a-zA-Z0-9]+").unwrap())
}

fn common_word_regex() -> &'static Regex {
    static COMMON_RE: OnceLock<Regex> = OnceLock::new();
    COMMON_RE.get_or_init(|| Regex::new(r"[a-zA-Z]{3,}").unwrap())
}

/// Build TF-IDF model from documents.
fn build_tfidf_model(documents: &[&str]) -> (TfIdfModel, Vec<Vec<String>>) {
    let tokenized: Vec<Vec<String>> = documents.iter().map(|d| preprocess_text(d)).collect();

    let mut word_set: HashSet<String> = HashSet::new();
    for tokens in &tokenized {
        word_set.extend(tokens.iter().cloned());
    }

    let vocabulary: Vec<String> = word_set.into_iter().collect();
    let vocab_map: HashMap<String, usize> = vocabulary
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, w)| (w, i))
        .collect();

    let mut doc_freq = vec![0usize; vocabulary.len()];
    for tokens in &tokenized {
        let mut seen: HashSet<&str> = HashSet::new();
        for word in tokens {
            if seen.insert(word.as_str()) {
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

    (
        TfIdfModel {
            vocabulary,
            vocab_map,
            idf,
        },
        tokenized,
    )
}

fn preprocess_text(text: &str) -> Vec<String> {
    word_regex()
        .find_iter(&text.to_lowercase())
        .map(|m| m.as_str().to_string())
        .filter(|w| w.len() >= 2)
        .collect()
}

fn compute_tfidf_vector(tokens: &[String], model: &TfIdfModel) -> Vec<f64> {
    let mut tf = vec![0.0; model.vocabulary.len()];
    for word in tokens {
        if let Some(&idx) = model.vocab_map.get(word.as_str()) {
            tf[idx] += 1.0;
        }
    }

    let total: f64 = tf.iter().sum();
    if total > 0.0 {
        for freq in &mut tf {
            *freq /= total;
        }
    }

    tf.iter().zip(&model.idf).map(|(t, i)| t * i).collect()
}

/// Extract features from files
fn extract_features(
    files: &[PathBuf],
    config: &IntelligentConfig,
) -> Result<Vec<FileFeatures>, io::Error> {
    let file_contents: Vec<(PathBuf, Option<String>)> = files
        .par_iter()
        .map(|path| {
            let content = if is_text_file(path) {
                read_first_n_file_lines(path, config.max_lines_to_read).ok()
            } else {
                None
            };
            (path.clone(), content)
        })
        .collect();

    let text_contents: Vec<&str> = file_contents
        .iter()
        .filter_map(|(_, content)| content.as_deref())
        .collect();

    let (tfidf_model, token_cache) = if !text_contents.is_empty() {
        let (model, tokens) = build_tfidf_model(&text_contents);
        (Some(model), tokens)
    } else {
        (None, Vec::new())
    };

    let mut token_idx = 0usize;
    let features: Vec<FileFeatures> = file_contents
        .into_iter()
        .map(|(path, content)| {
            let filename_vector = extract_filename_features(&path);
            let content_vector = if let (Some(model), Some(_)) = (&tfidf_model, &content) {
                let tokens = &token_cache[token_idx];
                token_idx += 1;
                Some(compute_tfidf_vector(tokens, model))
            } else {
                None
            };

            FileFeatures {
                path,
                filename_vector,
                content_vector,
            }
        })
        .collect();

    Ok(features)
}

/// Perform K-means clustering
fn perform_clustering(
    features: &[FileFeatures],
    config: &IntelligentConfig,
) -> Result<Vec<Vec<usize>>, io::Error> {
    if features.is_empty() {
        return Ok(Vec::new());
    }

    let content_dim = features
        .iter()
        .find_map(|f| f.content_vector.as_ref().map(|v| v.len()))
        .unwrap_or(0);

    let combined_vectors: Vec<Vec<f64>> = features
        .iter()
        .map(|f| combine_feature_vectors(f, content_dim, config))
        .collect();

    let k = determine_k(&combined_vectors, config);

    if k == 0 {
        return Ok(vec![features.iter().enumerate().map(|(i, _)| i).collect()]);
    }

    let assignments = kmeans(&combined_vectors, k, config.max_iterations);

    let mut clusters: Vec<Vec<usize>> = vec![Vec::new(); k];
    for (idx, &cluster_id) in assignments.iter().enumerate() {
        clusters[cluster_id].push(idx);
    }

    clusters.retain(|c| c.len() >= config.min_cluster_size);

    Ok(clusters)
}

/// Combine filename and content feature vectors into one fixed-length vector.
fn combine_feature_vectors(
    features: &FileFeatures,
    content_dim: usize,
    config: &IntelligentConfig,
) -> Vec<f64> {
    let mut combined = features.filename_vector.clone();

    for val in &mut combined {
        *val *= config.filename_similarity_weight;
    }

    match &features.content_vector {
        Some(content_vec) => {
            combined.extend(
                content_vec
                    .iter()
                    .map(|&v| v * config.content_similarity_weight),
            );
        }
        None => {
            combined.extend(std::iter::repeat(0.0).take(content_dim));
        }
    }

    combined
}

fn determine_k(vectors: &[Vec<f64>], config: &IntelligentConfig) -> usize {
    let n = vectors.len();
    ((n as f64 / 2.0).sqrt().ceil() as usize)
        .max(2)
        .min(config.max_clusters)
        .min(n)
}

struct Xorshift64(u64);

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() as f64) / (u64::MAX as f64)
    }

    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() as usize) % bound.max(1)
    }
}

/// k-means++ initialization
fn kmeans_plus_plus_init(vectors: &[Vec<f64>], k: usize) -> Vec<Vec<f64>> {
    let n = vectors.len();
    let mut rng = Xorshift64::new(0x9E3779B97F4A7C15);
    let mut centroids: Vec<Vec<f64>> = Vec::with_capacity(k);

    let first = rng.next_usize(n);
    centroids.push(vectors[first].clone());

    while centroids.len() < k {
        let sq_distances: Vec<f64> = vectors
            .iter()
            .map(|v| {
                centroids
                    .iter()
                    .map(|c| euclidean_distance(v, c))
                    .fold(f64::MAX, f64::min)
                    .powi(2)
            })
            .collect();

        let sum: f64 = sq_distances.iter().sum();
        if sum <= 0.0 {
            centroids.push(vectors[rng.next_usize(n)].clone());
            continue;
        }

        let target = rng.next_f64() * sum;
        let mut acc = 0.0;
        let mut chosen = n - 1;
        for (i, d) in sq_distances.iter().enumerate() {
            acc += d;
            if acc >= target {
                chosen = i;
                break;
            }
        }
        centroids.push(vectors[chosen].clone());
    }

    centroids
}

/// K-means clustering algorithm
/// refer this page to learn more about K-means:
///  https://www.ibm.com/think/topics/k-means-clustering
fn kmeans(vectors: &[Vec<f64>], k: usize, max_iterations: usize) -> Vec<usize> {
    let n = vectors.len();
    if n == 0 || k == 0 {
        return Vec::new();
    }

    let dim = vectors[0].len();
    assert!(
        vectors.iter().all(|v| v.len() == dim),
        "All vectors must have the same dimension (expected {})",
        dim
    );
    let k = k.min(n);

    let mut centroids = kmeans_plus_plus_init(vectors, k);
    let mut assignments = vec![0usize; n];

    for _ in 0..max_iterations {
        let new_assignments: Vec<usize> = vectors
            .par_iter()
            .map(|vector| {
                centroids
                    .iter()
                    .enumerate()
                    .map(|(j, c)| (j, euclidean_distance(vector, c)))
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
                    .map(|(j, _)| j)
                    .unwrap_or(0)
            })
            .collect();

        let changed = new_assignments != assignments;
        assignments = new_assignments;

        if !changed {
            break;
        }

        let mut new_centroids = vec![vec![0.0_f64; dim]; k];
        let mut counts = vec![0usize; k];

        for (i, vector) in vectors.iter().enumerate() {
            let cluster = assignments[i];
            counts[cluster] += 1;
            for j in 0..dim {
                new_centroids[cluster][j] += vector[j];
            }
        }

        for (cluster, count) in counts.iter().enumerate() {
            if *count > 0 {
                for val in &mut new_centroids[cluster] {
                    *val /= *count as f64;
                }
            } else {
                new_centroids[cluster] = centroids[cluster].clone();
            }
        }

        centroids = new_centroids;
    }

    assignments
}

fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn generate_group_names(clusters: Vec<Vec<usize>>, features: &[FileFeatures]) -> Vec<FileGroup> {
    clusters
        .into_iter()
        .map(|cluster_indices| {
            let files: Vec<PathBuf> = cluster_indices
                .iter()
                .map(|&i| features[i].path.clone())
                .collect();
            let suggested_name = generate_name_from_files(&files);
            FileGroup {
                files,
                suggested_name,
            }
        })
        .collect()
}

fn generate_name_from_files(files: &[PathBuf]) -> String {
    if files.is_empty() {
        return "Group".to_string();
    }

    let filenames: Vec<String> = files
        .iter()
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_lowercase()))
        .collect();

    let common_prefix = find_common_prefix(&filenames);

    if !common_prefix.is_empty() && common_prefix.len() >= 3 {
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

    let words = extract_common_words(&filenames);
    if !words.is_empty() {
        let mut name = words[0].clone();
        if let Some(s) = name.get_mut(0..1) {
            s.make_ascii_uppercase();
        }
        return name;
    }

    if let Some(ext) = files[0].extension() {
        format!("{}_Files", ext.to_string_lossy().to_uppercase())
    } else {
        "Mixed_Files".to_string()
    }
}

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

fn extract_common_words(filenames: &[String]) -> Vec<String> {
    let mut word_counts: HashMap<String, usize> = HashMap::new();
    let re = common_word_regex();

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

#[allow(dead_code)]
fn compute_centroid(indices: &[usize], features: &[FileFeatures]) -> Vec<f64> {
    if indices.is_empty() {
        return Vec::new();
    }

    let dim = features[indices[0]].filename_vector.len();
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

/// Move a single file, falling back to copy+delete if a plain rename fails
/// (e.g. EXDEV when the destination is on a different filesystem/mount).
fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    fs::copy(from, to)?;
    fs::remove_file(from)?;
    Ok(())
}

/// Move files into their organized groups.
pub fn move_files_to_groups(
    base_path: &Path,
    result: &ClusterResult,
    dry_run: bool,
) -> Result<Vec<String>, io::Error> {
    let mut log = Vec::new();
    let dir_manager = LazyDirManager::new();

    for group in &result.groups {
        let dir_name = sanitize_dirname(&group.suggested_name);
        let group_dir = base_path.join(&dir_name);

        if !dry_run {
            if let Err(e) = dir_manager.ensure_dir_exists(&group_dir) {
                log.push(format!(
                    "  ERROR: could not create {}: {} (skipping group)",
                    group_dir.display(),
                    e
                ));
                continue;
            }
            log.push(format!("Created directory: {}", group_dir.display()));
        } else {
            log.push(format!("[DRY RUN] Would create: {}", group_dir.display()));
        }

        for file_path in &group.files {
            let Some(filename) = file_path.file_name() else {
                continue;
            };
            let dest_path = group_dir.join(filename);

            if file_path == &dest_path {
                continue;
            }

            if !dry_run {
                let final_dest = match handle_conflict(&dest_path) {
                    Ok(p) => p,
                    Err(e) => {
                        log.push(format!(
                            "  ERROR: conflict resolution failed for {}: {}",
                            file_path.display(),
                            e
                        ));
                        continue;
                    }
                };

                match move_file(file_path, &final_dest) {
                    Ok(()) => log.push(format!(
                        "  Moved: {} \u{2192} {}",
                        file_path.display(),
                        final_dest.display()
                    )),
                    Err(e) => log.push(format!(
                        "  ERROR: could not move {}: {}",
                        file_path.display(),
                        e
                    )),
                }
            } else {
                log.push(format!(
                    "  [DRY RUN] Would move: {} \u{2192} {}",
                    file_path.display(),
                    dest_path.display()
                ));
            }
        }
    }

    Ok(log)
}

fn sanitize_dirname(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    if cleaned.is_empty() {
        "Group".to_string()
    } else {
        cleaned
    }
}

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
        let new_path = parent.join(format!("{}_{}{}", stem, i, ext));
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

enum AppState {
    Ready,
    Analyzing(Job<Result<ClusterResult, String>>),
    Complete(ClusterResult),
    Moving(Job<Vec<String>>),
    Moved(Vec<String>),
}

pub struct IntelligentTuiApp {
    config: IntelligentConfig,
    base_path: PathBuf,
    state: AppState,
    log_messages: Vec<String>,
    tick: usize,
    state_changed: bool,
}

impl IntelligentTuiApp {
    pub fn new(config: IntelligentConfig, base_path: PathBuf) -> Self {
        Self {
            config,
            base_path,
            state: AppState::Ready,
            log_messages: Vec::new(),
            tick: 0,
            state_changed: false,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        tui::run(self)
    }

    fn start_analysis(&mut self) {
        self.state = AppState::Analyzing(Job::spawn({
            let config = self.config.clone();
            let base_path = self.base_path.clone();
            move |tx| {
                let cb: ProgressCallback = Box::new(move |msg| {
                    let _ = tx.send(Progress::Update(msg));
                });
                organize_files_intelligently(&base_path, &config, Some(cb))
                    .map_err(|e| e.to_string())
            }
        }));
    }

    fn start_move(&mut self, result: ClusterResult, dry_run: bool) {
        let base_path = self.base_path.clone();
        self.log_messages.clear();
        let job = Job::spawn(move |tx| {
            let label = if dry_run { "[DRY RUN] " } else { "" };
            let _ = tx.send(Progress::Update(format!(
                "{label}Moving files to groups..."
            )));
            move_files_to_groups(&base_path, &result, dry_run)
                .unwrap_or_else(|e| vec![format!("Error: {e}")])
        });
        if dry_run {
            // For dry run, run synchronously so we can stay in Complete state
            // and just update log_messages (no state change needed).
            // We still use tui::Job so the spinner shows briefly.
            self.state = AppState::Moving(job);
        } else {
            self.state = AppState::Moving(job);
        }
    }

    fn draw_ready(&self, f: &mut Frame, cols: [Rect; 3]) {
        let left = vec![
            tui::blank(),
            tui::section_label("target"),
            tui::path_line(&self.base_path.display().to_string()),
            tui::blank(),
            tui::section_label("weights"),
            tui::stat(
                "filename",
                format!("{:.0}%", self.config.filename_similarity_weight * 100.0),
                tui::INFO,
            ),
            tui::stat(
                "content",
                format!("{:.0}%", self.config.content_similarity_weight * 100.0),
                tui::ML,
            ),
            tui::stat(
                "threshold",
                format!("{:.0}%", self.config.similarity_threshold * 100.0),
                tui::WARN,
            ),
        ];
        f.render_widget(tui::paragraph(left, "scan target"), cols[0]);

        let mid = vec![
            tui::blank(),
            tui::header("intelligent ml grouping", tui::OK),
            tui::blank(),
            tui::section_label("text files"),
            tui::bullet(format!(
                "reads first {} lines",
                self.config.max_lines_to_read
            )),
            tui::bullet("builds tf-idf content vectors"),
            tui::blank(),
            tui::section_label("all files"),
            tui::bullet("character n-gram filename features"),
            tui::bullet(format!(
                "k-means++ clustering (max {} groups)",
                self.config.max_clusters
            )),
            tui::bullet("auto-generates group names"),
            tui::blank(),
            tui::header("press [s] to start", tui::MUTED),
        ];
        f.render_widget(tui::paragraph(mid, "overview"), cols[1]);

        let right = vec![
            tui::blank(),
            tui::section_label("clustering"),
            tui::stat(
                "min cluster size",
                self.config.min_cluster_size.to_string(),
                tui::MUTED,
            ),
            tui::stat(
                "max clusters",
                self.config.max_clusters.to_string(),
                tui::MUTED,
            ),
            tui::stat(
                "max iterations",
                self.config.max_iterations.to_string(),
                tui::MUTED,
            ),
        ];
        f.render_widget(tui::paragraph(right, "config"), cols[2]);
    }

    fn draw_complete(&self, f: &mut Frame, cols: [Rect; 3], result: &ClusterResult) {
        let total: usize = result.groups.iter().map(|g| g.files.len()).sum();
        let mut sorted: Vec<_> = result.groups.iter().collect();
        sorted.sort_by(|a, b| b.files.len().cmp(&a.files.len()));

        let left = vec![
            tui::blank(),
            tui::header("✓ analysis complete", tui::OK),
            tui::blank(),
            tui::stat("files analyzed", total.to_string(), tui::OK),
            tui::stat("groups found", result.groups.len().to_string(), tui::ML),
            tui::blank(),
            tui::header("press [m] move  [d] dry run", tui::WARN),
        ];
        f.render_widget(tui::paragraph(left, "results"), cols[0]);

        // Mid: group list as bar chart entries
        let entries: Vec<(String, usize, Color)> = sorted
            .iter()
            .map(|g| (g.suggested_name.clone(), g.files.len(), tui::ML))
            .collect();
        tui::bar_chart(f, cols[1], "groups by size", &entries);

        // Right: dry-run preview log or prompt
        let right_lines: Vec<Line> = if self.log_messages.is_empty() {
            vec![
                tui::blank(),
                tui::header("no preview yet", tui::MUTED),
                tui::blank(),
                tui::bullet("press [d] for dry-run preview"),
            ]
        } else {
            let mut v = vec![tui::section_label("preview"), tui::blank()];
            for msg in self.log_messages.iter().take(16) {
                v.push(Line::from(format!("  {msg}")));
            }
            if self.log_messages.len() > 16 {
                v.push(tui::more(self.log_messages.len() - 16, "operations"));
            }
            v
        };
        f.render_widget(tui::paragraph(right_lines, "dry run preview"), cols[2]);
    }

    fn draw_moved(&self, f: &mut Frame, cols: [Rect; 3], log: &[String]) {
        let errors: Vec<_> = log.iter().filter(|m| m.contains("ERROR")).collect();

        let left = vec![
            tui::blank(),
            tui::header("✓ files moved", tui::OK),
            tui::blank(),
            tui::stat("operations", log.len().to_string(), tui::OK),
            tui::stat("errors", errors.len().to_string(), tui::ERR),
        ];
        f.render_widget(tui::paragraph(left, "complete"), cols[0]);

        let mut err_lines: Vec<Line> = vec![tui::section_label("errors"), tui::blank()];
        for msg in errors.iter().take(14) {
            err_lines.push(tui::bullet(&**msg));
        }
        f.render_widget(tui::paragraph(err_lines, "errors"), cols[1]);
        let mut mid: Vec<Line> = vec![tui::section_label("operations log"), tui::blank()];
        for msg in log.iter().take(16) {
            mid.push(Line::from(format!("  {msg}")));
        }
        if log.len() > 16 {
            mid.push(tui::more(log.len() - 16, "operations"));
        }
        f.render_widget(tui::paragraph(mid, "log"), cols[2]);
    }

    /// Auto-analyze files without UI interaction.
    pub fn auto_analyze(&mut self) -> io::Result<()> {
        println!("Starting intelligent ML-based analysis...");

        let result = organize_files_intelligently(&self.base_path, &self.config, None)?;
        let total_files: usize = result.groups.iter().map(|g| g.files.len()).sum();

        println!("\nAnalysis Complete!\n");
        println!("Summary:");
        println!("   - Total files analyzed: {}", total_files);
        println!("   - Groups created:       {}", result.groups.len());

        if !result.groups.is_empty() {
            println!("\n Discovered Groups:\n");
            let mut sorted: Vec<_> = result.groups.iter().collect();
            sorted.sort_by(|a, b| b.files.len().cmp(&a.files.len()));

            for group in sorted.iter() {
                println!(
                    "  {:30} -> {} files",
                    group.suggested_name,
                    group.files.len()
                );
            }

            let avg_group_size = total_files / result.groups.len();
            println!("\n Statistics:");
            println!("   - Average group size: {} files", avg_group_size);
            println!(
                "   - Largest group:      {} files ({})",
                sorted[0].files.len(),
                sorted[0].suggested_name
            );
            println!(
                "   - Smallest group:     {} files ({})",
                sorted.last().unwrap().files.len(),
                sorted.last().unwrap().suggested_name
            );
        }

        self.state = AppState::Complete(result);
        Ok(())
    }
}

impl App for IntelligentTuiApp {
    fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        self.state_changed = false;

        let state = std::mem::replace(&mut self.state, AppState::Ready);

        self.state = match state {
            AppState::Analyzing(mut job) => match job.poll() {
                Some(Ok(result)) => {
                    self.state_changed = true;
                    AppState::Complete(result)
                }
                Some(Err(e)) => {
                    self.state_changed = true;
                    self.log_messages.push(format!("Error: {e}"));
                    AppState::Ready
                }
                None => AppState::Analyzing(job),
            },
            AppState::Moving(mut job) => match job.poll() {
                Some(log) => {
                    self.state_changed = true;
                    self.log_messages = log.clone();
                    AppState::Moved(log)
                }
                None => AppState::Moving(job),
            },
            other => other,
        };
    }

    fn needs_clear(&self) -> bool {
        self.state_changed
    }

    fn on_key(&mut self, key: KeyCode) -> Flow {
        match (&self.state, key) {
            (AppState::Analyzing(_) | AppState::Moving(_), KeyCode::Char('q') | KeyCode::Esc) => {
                Flow::Continue
            }
            (_, KeyCode::Char('q') | KeyCode::Esc) => Flow::Quit,
            (AppState::Ready, KeyCode::Char('s')) => {
                self.start_analysis();
                self.state_changed = true;
                Flow::Continue
            }
            (AppState::Complete(_), KeyCode::Char('m')) => {
                let state = std::mem::replace(&mut self.state, AppState::Ready);
                if let AppState::Complete(result) = state {
                    self.start_move(result, false);
                    self.state_changed = true;
                }
                Flow::Continue
            }
            (AppState::Complete(_), KeyCode::Char('d')) => {
                let state = std::mem::replace(&mut self.state, AppState::Ready);
                if let AppState::Complete(result) = state {
                    self.start_move(result, true);
                    self.state_changed = true;
                }
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
            AppState::Ready => ("ready", tui::OK, &[("s", "analyze"), ("q", "quit")]),
            AppState::Analyzing(_) => ("analyzing", tui::ML, &[("...", "please wait")]),
            AppState::Complete(_) => (
                "complete",
                tui::OK,
                &[("m", "move files"), ("d", "dry run"), ("q", "quit")],
            ),
            AppState::Moving(_) => ("moving", tui::INFO, &[("...", "please wait")]),
            AppState::Moved(_) => ("done", tui::OK, &[("q", "quit")]),
        };

        let cols = tui::screen_frame(
            f,
            rows[0],
            rows[1],
            rows[2],
            "intelligent ml organizer",
            status,
            status_color,
            bindings,
            30,
            35,
        );

        match &self.state {
            AppState::Ready => self.draw_ready(f, cols),
            AppState::Analyzing(job) => {
                tui::busy_gauge(f, cols[1], "analyzing", &job.message, tui::ML, self.tick);
            }
            AppState::Complete(result) => self.draw_complete(f, cols, result),
            AppState::Moving(job) => {
                tui::busy_gauge(f, cols[1], "moving", &job.message, tui::INFO, self.tick);
            }
            AppState::Moved(log) => self.draw_moved(f, cols, log),
        }
    }
}
