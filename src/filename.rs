use crate::{Config, log_action, should_skip_file};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use regex::Regex;

// use std::str::FromStr;

/// Represents different types of filename patterns
#[derive(Debug, Clone)]
enum PatternType {
    Prefix(String),      // Files starting with a specific prefix
    Contains(String),    // Files containing a specific substring
    BaseName,           // Extract base name before numbers/dates
    DatePattern,        // Files with date patterns
}

/// A pattern rule for organizing files
#[derive(Debug, Clone)]
struct PatternRule {
    pattern_type: PatternType,
    target_folder: String,
    priority: u8, // Higher priority patterns are checked first
}

/// Creates the default filename pattern rules
fn create_pattern_rules() -> Vec<PatternRule> {
    vec![
        // Screenshot patterns (highest priority)
        PatternRule {
            pattern_type: PatternType::Prefix("screenshot".to_string()),
            target_folder: "screenshots".to_string(),
            priority: 100,
        },
        PatternRule {
            pattern_type: PatternType::Prefix("screen shot".to_string()),
            target_folder: "screenshots".to_string(),
            priority: 100,
        },
        PatternRule {
            pattern_type: PatternType::Contains("screenshot".to_string()),
            target_folder: "screenshots".to_string(),
            priority: 90,
        },

        // Download patterns
        PatternRule {
            pattern_type: PatternType::Prefix("download".to_string()),
            target_folder: "downloads".to_string(),
            priority: 80,
        },

        // Invoice patterns
        PatternRule {
            pattern_type: PatternType::Prefix("invoice".to_string()),
            target_folder: "invoices".to_string(),
            priority: 85,
        },
        PatternRule {
            pattern_type: PatternType::Contains("invoice".to_string()),
            target_folder: "invoices".to_string(),
            priority: 75,
        },

        // Receipt patterns
        PatternRule {
            pattern_type: PatternType::Prefix("receipt".to_string()),
            target_folder: "receipts".to_string(),
            priority: 85,
        },
        PatternRule {
            pattern_type: PatternType::Contains("receipt".to_string()),
            target_folder: "receipts".to_string(),
            priority: 75,
        },

        // Backup patterns
        PatternRule {
            pattern_type: PatternType::Prefix("backup".to_string()),
            target_folder: "backups".to_string(),
            priority: 80,
        },
        PatternRule {
            pattern_type: PatternType::Contains("backup".to_string()),
            target_folder: "backups".to_string(),
            priority: 70,
        },

        // Draft patterns
        PatternRule {
            pattern_type: PatternType::Prefix("draft".to_string()),
            target_folder: "drafts".to_string(),
            priority: 80,
        },

        // Copy patterns
        PatternRule {
            pattern_type: PatternType::Contains(" copy".to_string()),
            target_folder: "copies".to_string(),
            priority: 60,
        },
        PatternRule {
            pattern_type: PatternType::Contains("_copy".to_string()),
            target_folder: "copies".to_string(),
            priority: 60,
        },

        // Date-based organization (lower priority)
        PatternRule {
            pattern_type: PatternType::DatePattern,
            target_folder: "dated_files".to_string(),
            priority: 30,
        },

        // Base name extraction (lowest priority)
        PatternRule {
            pattern_type: PatternType::BaseName,
            target_folder: "grouped".to_string(),
            priority: 10,
        },
    ]
}

/// Organizes files based on filename patterns
pub fn organize_by_filename(config: &Config) -> io::Result<usize> {
    let base_path = Path::new(&config.directory);
    let mut pattern_rules = create_pattern_rules();

    // Sort rules by priority (highest first)
    pattern_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

    let mut files_organized = 0;
    let mut folder_counts: HashMap<String, usize> = HashMap::new();

    // Read directory entries
    for entry in fs::read_dir(base_path)? {
        let entry = entry?;
        let file_path = entry.path();

        // Skip directories
        if file_path.is_dir() {
            continue;
        }

        let filename = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if should_skip_file(filename) {
            if config.verbose {
                println!("  ⏭️  Skipping system file: {}", filename);
            }
            continue;
        }

        // Find the first matching pattern
        let target_folder = find_matching_pattern(filename, &pattern_rules);

        if let Some(folder_name) = target_folder {
            let target_dir = base_path.join(&folder_name);
            let target_path = target_dir.join(filename);

            // Handle file name conflicts
            let final_target = handle_naming_conflict(&target_path)?;

            if config.verbose {
                println!("🎯 Pattern match: '{}' → '{}' folder", filename, folder_name);
            }

            log_action(config, "Moving",
                &file_path.display().to_string(),
                &final_target.display().to_string());

            if !config.dry_run {
                // Create directory if it doesn't exist
                if !target_dir.exists() {
                    fs::create_dir_all(&target_dir)?;
                    if config.verbose {
                        println!("  📁 Created directory: {}", folder_name);
                    }
                }

                // Move the file
                fs::rename(&file_path, &final_target)?;
            }

            files_organized += 1;
            *folder_counts.entry(folder_name).or_insert(0) += 1;
        } else {
            // No pattern matched - use fallback to extension-based organization
            if config.verbose {
                println!("⚠️  No pattern matched for '{}', using extension fallback", filename);
            }

            let fallback_result = organize_single_file_by_extension(&file_path, base_path, config)?;
            if fallback_result.is_some() {
                files_organized += 1;
                let folder = fallback_result.unwrap();
                *folder_counts.entry(folder).or_insert(0) += 1;
            }
        }
    }

    // Print summary
    if !folder_counts.is_empty() {
        println!("\n📊 Pattern-based Organization Summary:");
        let mut sorted_folders: Vec<_> = folder_counts.iter().collect();
        sorted_folders.sort_by(|a, b| b.1.cmp(a.1));

        for (folder, count) in sorted_folders {
            println!("  {} files → {} folder", count, folder);
        }
    }

    Ok(files_organized)
}

/// Finds the first matching pattern for a filename
fn find_matching_pattern(filename: &str, rules: &[PatternRule]) -> Option<String> {
    let filename_lower = filename.to_lowercase();

    for rule in rules {
        match &rule.pattern_type {
            PatternType::Prefix(prefix) => {
                if filename_lower.starts_with(&prefix.to_lowercase()) {
                    return Some(rule.target_folder.clone());
                }
            }
            PatternType::Contains(substring) => {
                if filename_lower.contains(&substring.to_lowercase()) {
                    return Some(rule.target_folder.clone());
                }
            }
            PatternType::BaseName => {
                if let Some(base_name) = extract_base_name(filename) {
                    if base_name.len() > 2 && base_name != filename {
                        return Some(base_name);
                    }
                }
            }
            PatternType::DatePattern => {
                if contains_date_pattern(&filename_lower) {
                    return Some(rule.target_folder.clone());
                }
            }
        }
    }

    None
}

/// Extracts base name from filename (e.g., "invoice-2023-01.pdf" -> "invoice")
fn extract_base_name(filename: &str) -> Option<String> {
    // Remove extension first
    let stem = if let Some(dot_pos) = filename.rfind('.') {
        &filename[..dot_pos]
    } else {
        filename
    };

    // Look for common separators
    let separators = ['-', '_', ' ', '.'];

    for &sep in &separators {
        if let Some(pos) = stem.find(sep) {
            let base = &stem[..pos];
            if base.len() > 2 && base.chars().all(|c| c.is_alphabetic() || c == '_') {
                return Some(base.to_lowercase());
            }
        }
    }

    // Check if the stem contains numbers and letters
    let has_letters = stem.chars().any(|c| c.is_alphabetic());
    let has_numbers = stem.chars().any(|c| c.is_numeric());

    if has_letters && has_numbers && stem.len() > 4 {
        // Try to extract the alphabetic prefix
        let alphabetic_part: String = stem.chars()
            .take_while(|c| c.is_alphabetic() || *c == '_')
            .collect();

        if alphabetic_part.len() > 2 {
            return Some(alphabetic_part.to_lowercase());
        }
    }

    None
}

/// Checks if filename contains a date pattern
fn contains_date_pattern(filename: &str) -> bool {
    let date_patterns = [
        r"\d{4}[-_]\d{2}[-_]\d{2}",
        r"\d{8}",
        r"\d{2}[-_]\d{2}[-_]\d{4}",
        r"\d{2}[-_]\d{2}[-_]\d{2}",
    ];

    date_patterns.iter().any(|pattern| {
        Regex::new(pattern)
            .map(|re| re.is_match(filename))
            .unwrap_or(false)
    })
}

/// Fallback function to organize a single file by extension
fn organize_single_file_by_extension(
    file_path: &Path,
    base_path: &Path,
    config: &Config
) -> io::Result<Option<String>> {
    let extension = file_path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_else(|| "no_extension".to_string());

    // Simple extension-based categorization
    let category = match extension.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" => "images",
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => "videos",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" => "audio",
        "pdf" | "doc" | "docx" | "txt" | "rtf" | "odt" => "documents",
        "zip" | "rar" | "7z" | "tar" | "gz" => "archives",
        _ => "uncategorized",
    };

    let filename = file_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let target_dir = base_path.join(category);
    let target_path = target_dir.join(filename);
    let final_target = handle_naming_conflict(&target_path)?;

    log_action(config, "Moving (fallback)",
        &file_path.display().to_string(),
        &final_target.display().to_string());

    if !config.dry_run {
        if !target_dir.exists() {
            fs::create_dir_all(&target_dir)?;
        }
        fs::rename(file_path, &final_target)?;
    }

    Ok(Some(category.to_string()))
}

/// Handles naming conflicts by appending a number to the filename
fn handle_naming_conflict(target_path: &Path) -> io::Result<PathBuf> {
    if !target_path.exists() {
        return Ok(target_path.to_path_buf());
    }

    let parent = target_path.parent().unwrap();
    let stem = target_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let extension = target_path.extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    for i in 1..1000 {
        let new_name = format!("{}_{}{}",stem, i, extension);
        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return Ok(new_path);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Could not find available filename after 999 attempts"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_base_name() {
        assert_eq!(extract_base_name("invoice-2023-01.pdf"), Some("invoice".to_string()));
        assert_eq!(extract_base_name("receipt_store_001.jpg"), Some("receipt".to_string()));
        assert_eq!(extract_base_name("report-final.docx"), Some("report".to_string()));
        assert_eq!(extract_base_name("backup20231201.zip"), Some("backup".to_string()));
        assert_eq!(extract_base_name("simple.txt"), None);
        assert_eq!(extract_base_name("a.txt"), None);
    }

    #[test]
    fn test_contains_date_pattern() {
        assert!(contains_date_pattern("file-2023-01-15.pdf"));
        assert!(contains_date_pattern("backup_2023_12_01.zip"));
        assert!(contains_date_pattern("report20231201.docx"));
        assert!(contains_date_pattern("data-15-01-2023.csv"));
        assert!(!contains_date_pattern("simple-file.txt"));
        assert!(!contains_date_pattern("test123.pdf"));
    }

    #[test]
    fn test_find_matching_pattern() {
        let rules = create_pattern_rules();

        assert_eq!(
            find_matching_pattern("screenshot-2023.png", &rules),
            Some("screenshots".to_string())
        );

        assert_eq!(
            find_matching_pattern("invoice-january.pdf", &rules),
            Some("invoices".to_string())
        );

        assert_eq!(
            find_matching_pattern("backup_database.sql", &rules),
            Some("backups".to_string())
        );

        assert_eq!(
            find_matching_pattern("document copy.docx", &rules),
            Some("copies".to_string())
        );

        assert_eq!(
            find_matching_pattern("report-2023-01-15.pdf", &rules),
            Some("dated_files".to_string())
        );
    }

    #[test]
    fn test_pattern_priority() {
        let rules = create_pattern_rules();

        // Screenshot should match prefix rule (priority 100) not contains rule (priority 90)
        assert_eq!(
            find_matching_pattern("screenshot_backup.png", &rules),
            Some("screenshots".to_string())
        );

        // Should prioritize specific patterns over generic ones
        assert_eq!(
            find_matching_pattern("invoice_backup.pdf", &rules),
            Some("invoices".to_string())
        );
    }
}
