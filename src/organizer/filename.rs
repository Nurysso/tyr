//  Copyright (C) 2026 Dawood Khan
//  SPDX-License-Identifier: GPL-3.0-or-later

// Maintainer Dawood (Nurysso) contact - nurysso [at] proton.me

//  Filename based similarity organizer for tyr (https://github.com/Nurysso/tyr).

use ratatui::crossterm::event::KeyCode;
use ratatui::style::Color;
use ratatui::{layout::Constraint, layout::Rect, text::Line, Frame};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::organizer::tui::{self, App, Flow, Job, Progress};
use crate::organizer::utils::{is_hidden, LazyDirManager};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SimilarityConfig {
    #[serde(default = "default_levenshtein_threshold")]
    pub levenshtein_threshold: f64,
    #[serde(default = "default_jaccard_threshold")]
    pub jaccard_threshold: f64,
    #[serde(default = "default_levenshtein_weight")]
    pub levenshtein_weight: f64,
    #[serde(default = "default_jaccard_weight")]
    pub jaccard_weight: f64,
    #[serde(default = "default_min_similarity_score")]
    pub min_similarity_score: f64,

    #[serde(default = "default_skip_patterns")]
    pub skip_patterns: Vec<String>,
    #[serde(default = "default_true")]
    pub skip_hidden_files: bool,
}

fn default_levenshtein_threshold() -> f64 {
    0.7
}
fn default_jaccard_threshold() -> f64 {
    0.5
}
fn default_levenshtein_weight() -> f64 {
    0.6
}
fn default_jaccard_weight() -> f64 {
    0.4
}
fn default_min_similarity_score() -> f64 {
    0.65
}
fn default_true() -> bool {
    true
}

fn default_skip_patterns() -> Vec<String> {
    [
        ".DS_Store",
        "Thumbs.db",
        ".git",
        ".gitignore",
        "desktop.ini",
        ".localized",
        "~$",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

pub fn should_skip_file(filename: &str, skip_patterns: &[String]) -> bool {
    skip_patterns.iter().any(|p| filename.contains(p.as_str()))
}

impl Default for SimilarityConfig {
    fn default() -> Self {
        Self {
            levenshtein_threshold: default_levenshtein_threshold(),
            jaccard_threshold: default_jaccard_threshold(),
            levenshtein_weight: default_levenshtein_weight(),
            jaccard_weight: default_jaccard_weight(),
            min_similarity_score: default_min_similarity_score(),
            skip_patterns: default_skip_patterns(),
            skip_hidden_files: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileGroup {
    pub representative_name: String,
    pub files: Vec<String>,
    pub avg_similarity: f64,
}

#[derive(Debug, Default)]
pub struct OrganizeResult {
    pub files_moved: usize,
    pub folders_created: usize,
    pub files_skipped: usize,
    pub skipped_details: Vec<SkippedFile>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SkippedFile {
    pub filename: String,
    pub reason: SkipReason,
}

#[derive(Debug, Clone)]
pub enum SkipReason {
    SingleFile,
    SystemFile,
    #[allow(dead_code)]
    AlreadyOrganized,
}

impl SkipReason {
    fn label(&self) -> &'static str {
        match self {
            SkipReason::SingleFile => "no similar matches found",
            SkipReason::SystemFile => "system file",
            SkipReason::AlreadyOrganized => "already organized",
        }
    }
}

/// Minimum single-character edits to turn `s1` into `s2`.
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (s1.chars().collect(), s2.chars().collect());
    let (len1, len2) = (a.len(), b.len());
    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut matrix = vec![vec![0usize; len2 + 1]; len1 + 1];
    for (i, row) in matrix.iter_mut().enumerate() {
        row[0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }
    matrix[len1][len2]
}

pub fn levenshtein_similarity(s1: &str, s2: &str) -> f64 {
    let max_len = s1.len().max(s2.len()) as f64;
    if max_len == 0.0 {
        return 1.0;
    }
    1.0 - (levenshtein_distance(s1, s2) as f64 / max_len)
}

const PHRASES: [&str; 10] = [
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
];
const DELIMITERS: [char; 9] = ['-', '_', '.', '(', ')', '[', ']', '{', '}'];

fn push_token(tokens: &mut HashSet<String>, raw: &str) {
    let lower = raw.to_lowercase();
    if lower.len() > 1 && !lower.chars().all(|c| c.is_numeric()) {
        tokens.insert(lower);
    }
}

/// Tokenizes a filename into words, phrase hints and bigrams for fuzzy
/// matching (e.g. "IMG_2024_beach.jpg" and "IMG_2024_beach(1).jpg" overlap).
fn tokenize_filename(filename: &str) -> HashSet<String> {
    let mut tokens = HashSet::new();
    let name = filename
        .rsplit_once('.')
        .map(|(n, _)| n)
        .unwrap_or(filename);
    let name_lower = name.to_lowercase();

    let clean_full: String = name_lower
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    if !clean_full.trim().is_empty() {
        tokens.insert(clean_full.trim().to_string());
    }

    for phrase in PHRASES.iter().filter(|p| name_lower.contains(*p)) {
        tokens.insert(phrase.to_string());
    }

    let mut current = String::new();
    for c in name.chars() {
        if DELIMITERS.contains(&c) || c == ' ' {
            if !current.is_empty() {
                push_token(&mut tokens, &current);
                current.clear();
            }
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        push_token(&mut tokens, &current);
    }

    let words: Vec<String> = name
        .split(|c: char| !c.is_alphanumeric() && c != ' ')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_lowercase())
        .collect();
    for window in words.windows(2) {
        tokens.insert(format!("{} {}", window[0], window[1]));
    }

    tokens
}

pub fn jaccard_similarity(s1: &str, s2: &str) -> f64 {
    let (t1, t2) = (tokenize_filename(s1), tokenize_filename(s2));
    if t1.is_empty() && t2.is_empty() {
        return 1.0;
    }
    let union = t1.union(&t2).count();
    if union == 0 {
        return 0.0;
    }
    t1.intersection(&t2).count() as f64 / union as f64
}

pub fn combined_similarity(s1: &str, s2: &str, config: &SimilarityConfig) -> f64 {
    levenshtein_similarity(s1, s2) * config.levenshtein_weight
        + jaccard_similarity(s1, s2) * config.jaccard_weight
}

/// Greedily clusters filenames: each unassigned file seeds a group, then
/// pulls in every remaining file scoring above `min_similarity_score`.
pub fn group_similar_files(filenames: &[String], config: &SimilarityConfig) -> Vec<FileGroup> {
    let mut groups = Vec::new();
    let mut assigned: HashSet<usize> = HashSet::new();

    for i in 0..filenames.len() {
        if assigned.contains(&i) {
            continue;
        }
        assigned.insert(i);
        let mut group_files = vec![filenames[i].clone()];
        let mut similarities = Vec::new();

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

fn extract_common_prefix(filenames: &[String]) -> String {
    let Some(first) = filenames.first() else {
        return String::new();
    };
    if filenames.len() == 1 {
        return first.clone();
    }

    let prefix: String = first
        .chars()
        .enumerate()
        .take_while(|(i, c)| filenames.iter().all(|f| f.chars().nth(*i) == Some(*c)))
        .map(|(_, c)| c)
        .collect();

    prefix
        .trim_end_matches(['-', '_', ' ', '.', '(', '[', '{'])
        .to_string()
}

pub fn suggest_folder_name(group: &FileGroup) -> String {
    let name = if group.representative_name.is_empty() {
        "SimilarFiles"
    } else {
        &group.representative_name
    };
    let cleaned = smart_folder_naming(name);
    if cleaned.is_empty() {
        "SimilarFiles".to_string()
    } else {
        cleaned
    }
}

const NAME_PATTERNS: [(&str, &str); 17] = [
    ("whatsapp chat", "WhatsAppChats"),
    ("whatsapp image", "WhatsAppImages"),
    ("whatsapp", "WhatsApp"),
    ("screenshot", "Screenshots"),
    ("screen shot", "Screenshots"),
    ("screencapture", "Screenshots"),
    ("chatgpt", "ChatGPTImages"),
    ("document", "Documents"),
    ("report", "Reports"),
    ("invoice", "Invoices"),
    ("receipt", "Receipts"),
    ("img_", "Images"),
    ("dsc", "CameraPhotos"),
    ("dcim", "CameraPhotos"),
    ("photo", "Photos"),
    ("pic", "Pictures"),
    ("download", "Downloads"),
];

/// Maps a raw common-prefix into a memorable folder name: known patterns
/// (WhatsApp, screenshots, ...) win outright, otherwise dates/versions/
/// numeric suffixes are stripped from the prefix and it's title-cased.
fn smart_folder_naming(name: &str) -> String {
    let lower = name.to_lowercase();
    if let Some((_, replacement)) = NAME_PATTERNS.iter().find(|(p, _)| lower.contains(p)) {
        return replacement.to_string();
    }

    let joined: String = name
        .split(|c: char| !c.is_alphanumeric() && c != ' ')
        .filter(|s| !s.is_empty())
        .collect();
    let mut result =
        remove_number_suffixes(&remove_version_patterns(&remove_date_patterns(&joined)));

    if let Some(first) = result.chars().next() {
        result = first.to_uppercase().collect::<String>() + &result[first.len_utf8()..];
    }
    result
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "")
        .trim()
        .to_string()
}

fn remove_date_patterns(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut cleaned = String::new();
    let mut i = 0;
    while i < chars.len() {
        if i + 7 < chars.len() && chars[i..i + 4].iter().all(|c| c.is_numeric()) {
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
    let Some(pos) = s.rfind('v').filter(|p| *p > 0) else {
        return s.to_string();
    };
    let after = &s[pos + 1..];
    if after
        .chars()
        .all(|c| c.is_numeric() || c == '.' || c == '_' || c == '-')
    {
        s[..pos].to_string()
    } else {
        s.to_string()
    }
}

fn remove_number_suffixes(s: &str) -> String {
    let mut result = s.to_string();
    while matches!(result.chars().last(), Some(c) if c.is_numeric() || c == '_' || c == '-' || c == ' ')
    {
        result.pop();
    }
    result
}

fn handle_naming_conflict(path: &Path) -> io::Result<PathBuf> {
    let parent = path.parent().unwrap();
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();

    (1..1000)
        .map(|i| parent.join(format!("{stem}_{i}{ext}")))
        .find(|p| !p.exists())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Could not find available filename",
            )
        })
}

fn prepare_skip_folder(
    base_path: &Path,
    enabled: bool,
    dir_manager: &LazyDirManager,
    logger: &mut dyn FnMut(&str),
    errors: &mut Vec<String>,
) -> Option<PathBuf> {
    if !enabled {
        return None;
    }
    let skip_dir = base_path.join("tyr-skip");
    match dir_manager.ensure_dir_exists(&skip_dir) {
        Ok(_) => {
            logger(&format!("Skip folder ready: {}", skip_dir.display()));
            Some(skip_dir)
        }
        Err(e) => {
            errors.push(format!("Failed to create skip folder: {e}"));
            None
        }
    }
}

fn skip_group(
    files: &[String],
    skip_folder: Option<&PathBuf>,
    base_path: &Path,
    skip_patterns: &[String],
    logger: &mut dyn FnMut(&str),
    errors: &mut Vec<String>,
) -> Vec<SkippedFile> {
    files
        .iter()
        .map(|filename| {
            let reason = if should_skip_file(filename, skip_patterns) {
                SkipReason::SystemFile
            } else {
                SkipReason::SingleFile
            };
            logger(&format!("Skipped {}: {}", reason.label(), filename));
            if let Some(skip_dir) = skip_folder {
                let (source, dest) = (base_path.join(filename), skip_dir.join(filename));
                match fs::rename(&source, &dest) {
                    Ok(_) => logger(&format!("Moved to skip folder: {filename}")),
                    Err(e) => {
                        errors.push(format!("Failed to move '{filename}' to skip folder: {e}"))
                    }
                }
            }
            SkippedFile {
                filename: filename.clone(),
                reason,
            }
        })
        .collect()
}

fn move_group(
    group: &FileGroup,
    base_path: &Path,
    dir_manager: &LazyDirManager,
    logger: &mut dyn FnMut(&str),
    errors: &mut Vec<String>,
) -> (usize, usize) {
    let folder_name = suggest_folder_name(group);
    let target_dir = base_path.join(&folder_name);
    let was_known = dir_manager.is_known(&*target_dir);
    if let Err(e) = dir_manager.ensure_dir_exists(&target_dir) {
        errors.push(format!("Failed to create folder '{folder_name}': {e}"));
        return (0, 0);
    }
    let created_folders = usize::from(!was_known);
    if created_folders == 1 {
        logger(&format!("Created folder: {folder_name}"));
    }
    let mut moved = 0;
    for filename in &group.files {
        let (source, dest) = (base_path.join(filename), target_dir.join(filename));
        let final_dest = if dest.exists() {
            match handle_naming_conflict(&dest) {
                Ok(p) => p,
                Err(e) => {
                    errors.push(format!("Naming conflict for '{filename}': {e}"));
                    continue;
                }
            }
        } else {
            dest
        };
        match fs::rename(&source, &final_dest) {
            Ok(_) => {
                moved += 1;
                logger(&format!("Moved: {filename} -> {folder_name}"));
            }
            Err(e) => errors.push(format!("Failed to move '{filename}': {e}")),
        }
    }
    (moved, created_folders)
}

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

    let entries: Vec<_> = fs::read_dir(base_path)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let p = e.path();
            p.is_file() && !(config.skip_hidden_files && is_hidden(&p))
        })
        .collect();
    let filenames: Vec<String> = entries
        .iter()
        .filter_map(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from)
        })
        .collect();
    logger(&format!("Found {} files to process", filenames.len()));

    logger("Analyzing file similarities...");
    let groups = group_similar_files(&filenames, config);
    logger(&format!("Identified {} file groups", groups.len()));

    let mut result = OrganizeResult::default();
    let dir_manager = LazyDirManager::new();
    let skip_folder = prepare_skip_folder(
        base_path,
        move_skipped,
        &dir_manager,
        logger,
        &mut result.errors,
    );

    for group in groups {
        if group.files.len() < 2 {
            result.files_skipped += group.files.len();
            result.skipped_details.extend(skip_group(
                &group.files,
                skip_folder.as_ref(),
                base_path,
                &config.skip_patterns,
                logger,
                &mut result.errors,
            ));
            continue;
        }
        let (moved, created) =
            move_group(&group, base_path, &dir_manager, logger, &mut result.errors);
        result.files_moved += moved;
        result.folders_created += created;
    }

    logger(&format!(
        "Organization complete: {} files moved, {} folders created, {} files skipped",
        result.files_moved, result.folders_created, result.files_skipped
    ));
    Ok(result)
}

pub struct FilenameTuiApp {
    base_path: PathBuf,
    config: SimilarityConfig,
    state: FilenameAppState,
    move_skipped_to_folder: bool,
    groups: Vec<FileGroup>,
    scroll_offset: usize,
    log_messages: Vec<String>,
    tick: usize,
    state_changed: bool,
}

enum FilenameAppState {
    Ready,
    Analyzing(Job<Vec<FileGroup>>),
    ReviewGroups,
    Organizing(Job<(OrganizeResult, Vec<String>)>),
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
            log_messages: Vec::new(),
            tick: 0,
            state_changed: false,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        tui::run(self)
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.log_messages.clone()
    }

    fn start_analysis(&mut self) {
        let base_path = self.base_path.clone();
        let config = self.config.clone();
        self.state = FilenameAppState::Analyzing(Job::spawn(move |tx| {
            let _ = tx.send(Progress::Update(
                "Scanning and comparing filenames...".into(),
            ));

            let entries: Vec<_> = fs::read_dir(&base_path)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let p = e.path();
                    p.is_file() && !(config.skip_hidden_files && is_hidden(&p))
                })
                .collect();

            let filenames: Vec<String> = entries
                .iter()
                .filter_map(|e| {
                    e.path()
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(String::from)
                })
                .collect();

            group_similar_files(&filenames, &config)
        }));
    }

    fn start_organization(&mut self) {
        let base_path = self.base_path.clone();
        let config = self.config.clone();
        let move_skipped = self.move_skipped_to_folder;
        self.state = FilenameAppState::Organizing(Job::spawn(move |tx| {
            let mut log = Vec::new();
            let mut logger = |msg: &str| {
                log.push(msg.to_string());
                let _ = tx.send(Progress::Update(msg.to_string()));
            };
            let result = organize_by_similarity(&base_path, &config, move_skipped, &mut logger)
                .unwrap_or_default();
            (result, log)
        }));
    }

    fn draw_ready(&self, f: &mut Frame, cols: [Rect; 3]) {
        let left = vec![
            tui::blank(),
            tui::section_label("target"),
            tui::path_line(&self.base_path.display().to_string()),
            tui::blank(),
            tui::section_label("algorithm"),
            tui::stat("levenshtein weight", "60%", tui::INFO),
            tui::stat("jaccard weight", "40%", tui::MUTED),
            tui::stat("min similarity", "65%", tui::WARN),
        ];
        f.render_widget(tui::paragraph(left, "scan target"), cols[0]);

        let mid = vec![
            tui::blank(),
            tui::header("ready to analyze", tui::OK),
            tui::blank(),
            tui::bullet("compares every filename pair with dual similarity metrics"),
            tui::bullet("levenshtein: edit-distance on full name"),
            tui::bullet("jaccard: token overlap on word splits"),
            tui::blank(),
            tui::header("press [a] to begin", tui::MUTED),
        ];
        f.render_widget(tui::paragraph(mid, "overview"), cols[1]);

        let right = vec![
            tui::blank(),
            tui::section_label("options"),
            tui::stat(
                "move skipped",
                if self.move_skipped_to_folder {
                    "yes"
                } else {
                    "no"
                },
                tui::WARN,
            ),
            tui::blank(),
            tui::bullet("press [k] to toggle"),
            tui::bullet("skipped files → tyr-skip/"),
        ];
        f.render_widget(tui::paragraph(right, "options"), cols[2]);
    }

    fn draw_review(&self, f: &mut Frame, cols: [Rect; 3]) {
        let multi: Vec<_> = self.groups.iter().filter(|g| g.files.len() > 1).collect();
        let single = self.groups.len() - multi.len();

        // Left: summary stats
        let left = vec![
            tui::blank(),
            tui::header("✓ analysis complete", tui::OK),
            tui::blank(),
            tui::stat("groups (2+ files)", multi.len().to_string(), tui::INFO),
            tui::stat("single files", single.to_string(), tui::WARN),
            tui::blank(),
            tui::section_label("skip folder"),
            tui::stat(
                "move singles",
                if self.move_skipped_to_folder {
                    "yes → tyr-skip/"
                } else {
                    "no"
                },
                if self.move_skipped_to_folder {
                    tui::OK
                } else {
                    tui::MUTED
                },
            ),
        ];
        f.render_widget(tui::paragraph(left, "summary"), cols[0]);

        // Mid: scrollable group list
        let mut lines = vec![tui::section_label("groups preview")];
        for (i, group) in multi.iter().skip(self.scroll_offset).take(12).enumerate() {
            let folder = crate::organizer::filename::suggest_folder_name(group);
            lines.push(tui::header(
                format!(
                    "{:2}. {folder} ({} files, {:.0}%)",
                    i + 1 + self.scroll_offset,
                    group.files.len(),
                    group.avg_similarity * 100.0
                ),
                tui::INFO,
            ));
            for file in group.files.iter().take(2) {
                lines.push(tui::bullet(file));
            }
            if group.files.len() > 2 {
                lines.push(tui::bullet(format!("… {} more", group.files.len() - 2)));
            }
            lines.push(tui::blank());
        }
        if multi.len() > 12 + self.scroll_offset {
            lines.push(tui::more(multi.len() - 12 - self.scroll_offset, "groups"));
        }
        f.render_widget(tui::paragraph(lines, "review groups"), cols[1]);

        // Right: first group's files as a sample
        let right_lines: Vec<Line> = if let Some(g) = multi.first() {
            let mut v = vec![tui::section_label("sample group"), tui::blank()];
            for file in &g.files {
                v.push(tui::bullet(file));
            }
            v
        } else {
            vec![
                tui::blank(),
                tui::header("no multi-file groups", tui::MUTED),
            ]
        };
        f.render_widget(tui::paragraph(right_lines, "sample"), cols[2]);
    }

    fn draw_complete(&self, f: &mut Frame, cols: [Rect; 3], result: &OrganizeResult) {
        let left = vec![
            tui::blank(),
            tui::header("✓ organization complete", tui::OK),
            tui::blank(),
            tui::stat(
                "folders created",
                result.folders_created.to_string(),
                tui::OK,
            ),
            tui::stat("files moved", result.files_moved.to_string(), tui::INFO),
            tui::stat("files skipped", result.files_skipped.to_string(), tui::WARN),
            tui::stat("errors", result.errors.len().to_string(), tui::ERR),
        ];
        f.render_widget(tui::paragraph(left, "results"), cols[0]);

        let mut err_lines: Vec<Line> = vec![tui::section_label("errors"), tui::blank()];
        for e in result.errors.iter().take(14) {
            err_lines.push(tui::bullet(e));
        }
        if result.errors.len() > 14 {
            err_lines.push(tui::more(result.errors.len() - 14, "errors"));
        }
        f.render_widget(tui::paragraph(err_lines, "errors"), cols[1]);
        let mut mid_lines: Vec<Line> = vec![tui::section_label("skipped files"), tui::blank()];
        for skip in result
            .skipped_details
            .iter()
            .skip(self.scroll_offset)
            .take(14)
        {
            mid_lines.push(tui::bullet(format!(
                "{} — {}",
                skip.filename,
                skip.reason.label()
            )));
        }
        if result.skipped_details.len() > 14 + self.scroll_offset {
            mid_lines.push(tui::more(
                result.skipped_details.len() - 14 - self.scroll_offset,
                "files",
            ));
        }
        f.render_widget(tui::paragraph(mid_lines, "skipped"), cols[2]);
    }


    /// Runs without any UI: analyze then organize immediately.
    pub fn auto_organize(&mut self) -> io::Result<()> {
        let entries: Vec<_> = fs::read_dir(&self.base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                let p = e.path();
                p.is_file() && !(self.config.skip_hidden_files && is_hidden(&p))
            })
            .collect();
        let filenames: Vec<String> = entries
            .iter()
            .filter_map(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(String::from)
            })
            .collect();
        self.groups = group_similar_files(&filenames, &self.config);

        let mut logger = |_: &str| {};
        let result = organize_by_similarity(
            &self.base_path,
            &self.config,
            self.move_skipped_to_folder,
            &mut logger,
        )?;
        print_summary(&result);
        self.state = FilenameAppState::Complete(result);
        Ok(())
    }
}

fn print_summary(result: &OrganizeResult) {
    println!("\n\u{2726} Organization Complete!\n");
    println!("   Folders created: {}", result.folders_created);
    println!("   Files moved:     {}", result.files_moved);
    println!("   Files skipped:   {}", result.files_skipped);
    if !result.errors.is_empty() {
        println!("\nErrors:");
        for e in &result.errors {
            println!("   - {e}");
        }
    }
}

impl App for FilenameTuiApp {
    fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        self.state_changed = false;
        match &mut self.state {
            FilenameAppState::Analyzing(job) => {
                if let Some(groups) = job.poll() {
                    self.groups = groups;
                    self.scroll_offset = 0;
                    self.state = FilenameAppState::ReviewGroups;
                    self.state_changed = true;
                }
            }
            FilenameAppState::Organizing(job) => {
                if let Some((result, log)) = job.poll() {
                    self.log_messages = log;
                    self.scroll_offset = 0;
                    self.state = FilenameAppState::Complete(result);
                    self.state_changed = true;
                }
            }
            _ => {}
        }
    }

    fn needs_clear(&self) -> bool {
        self.state_changed
    }

    fn on_key(&mut self, key: KeyCode) -> Flow {
        use FilenameAppState::*;
        match (key, &self.state) {
            (KeyCode::Char('q') | KeyCode::Esc, Analyzing(_) | Organizing(_)) => Flow::Continue,
            (KeyCode::Char('q') | KeyCode::Esc, _) => Flow::Quit,
            (KeyCode::Char('a'), Ready) => {
                self.start_analysis();
                self.state_changed = true;
                Flow::Continue
            }
            (KeyCode::Char('s'), ReviewGroups) => {
                self.start_organization();
                self.state_changed = true;
                Flow::Continue
            }
            (KeyCode::Char('k'), ReviewGroups) => {
                self.move_skipped_to_folder = !self.move_skipped_to_folder;
                Flow::Continue
            }
            (KeyCode::Up, _) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                Flow::Continue
            }
            (KeyCode::Down, _) => {
                self.scroll_offset += 1;
                Flow::Continue
            }
            (KeyCode::PageUp, _) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
                Flow::Continue
            }
            (KeyCode::PageDown, _) => {
                self.scroll_offset += 10;
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
            FilenameAppState::Ready => ("ready", tui::OK, &[("a", "analyze"), ("q", "quit")]),
            FilenameAppState::Analyzing(_) => ("analyzing", tui::WARN, &[("...", "please wait")]),
            FilenameAppState::ReviewGroups => (
                "review",
                tui::INFO,
                &[
                    ("s", "organize"),
                    ("k", "toggle skip"),
                    ("↑↓", "scroll"),
                    ("q", "quit"),
                ],
            ),
            FilenameAppState::Organizing(_) => ("running", tui::INFO, &[("...", "please wait")]),
            FilenameAppState::Complete(_) => ("done", tui::OK, &[("↑↓", "scroll"), ("q", "quit")]),
        };

        let cols = tui::screen_frame(
            f,
            rows[0],
            rows[1],
            rows[2],
            "filename similarity organizer",
            status,
            status_color,
            bindings,
            35,
            28,
        );

        match &self.state {
            FilenameAppState::Ready => self.draw_ready(f, cols),
            FilenameAppState::Analyzing(job) => {
                tui::busy_gauge(f, cols[1], "analyzing", &job.message, tui::WARN, self.tick);
            }
            FilenameAppState::ReviewGroups => self.draw_review(f, cols),
            FilenameAppState::Organizing(job) => {
                tui::busy_gauge(f, cols[1], "organizing", &job.message, tui::INFO, self.tick);
            }
            FilenameAppState::Complete(result) => self.draw_complete(f, cols, result),
        }
    }
}
