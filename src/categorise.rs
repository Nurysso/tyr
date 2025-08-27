use crate::{Config, log_action, should_skip_file};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Creates the default extension to category mapping
fn create_extension_map() -> HashMap<String, Vec<&'static str>> {
    let mut map = HashMap::new();

    // Images
    map.insert("images".to_string(), vec![
        "jpg", "jpeg", "png", "gif", "bmp", "svg", "webp", "tiff", "ico", "heic", "raw"
    ]);

    // Videos
    map.insert("videos".to_string(), vec![
        "mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "m4v", "3gp", "mpg", "mpeg"
    ]);

    // Audio
    map.insert("audio".to_string(), vec![
        "mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus", "aiff"
    ]);

    // Documents
    map.insert("documents".to_string(), vec![
        "pdf", "doc", "docx", "txt", "rtf", "odt", "pages", "tex", "md", "epub"
    ]);

    // Spreadsheets
    map.insert("spreadsheets".to_string(), vec![
        "xls", "xlsx", "csv", "ods", "numbers"
    ]);

    // Presentations
    map.insert("presentations".to_string(), vec![
        "ppt", "pptx", "odp", "key"
    ]);

    // Archives
    map.insert("archives".to_string(), vec![
        "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "dmg", "pkg", "deb", "rpm"
    ]);

    // Code
    map.insert("code".to_string(), vec![
        "rs", "py", "js", "ts", "html", "css", "cpp", "c", "h", "java", "go",
        "php", "rb", "swift", "kt", "dart", "scala", "sh", "bat", "ps1"
    ]);

    // Data
    map.insert("data".to_string(), vec![
        "json", "xml", "yaml", "yml", "toml", "ini", "cfg", "conf", "sql", "db"
    ]);

    // Executables
    map.insert("executables".to_string(), vec![
        "exe", "msi", "app", "deb", "rpm", "dmg", "pkg", "appimage"
    ]);

    map
}

/// Organizes files by their extension category
pub fn organize_by_category(config: &Config) -> io::Result<usize> {
    let base_path = Path::new(&config.directory);
    let extension_map = create_extension_map();

    // Create reverse lookup map for faster extension-to-category mapping
    let mut ext_to_category: HashMap<String, String> = HashMap::new();
    for (category, extensions) in &extension_map {
        for &ext in extensions {
            ext_to_category.insert(ext.to_string(), category.clone());
        }
    }

    let mut files_organized = 0;
    let mut category_counts: HashMap<String, usize> = HashMap::new();

    // Read directory entries
    for entry in fs::read_dir(base_path)? {
        let entry = entry?;
        let file_path = entry.path();

        // Skip directories and system files
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

        // Determine target category
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .unwrap_or_else(|| "no_extension".to_string());

        let category = ext_to_category.get(&extension)
            .cloned()
            .unwrap_or_else(|| "extras".to_string());

        // Create target directory
        let target_dir = base_path.join(&category);
        let target_path = target_dir.join(filename);

        // Handle file name conflicts
        let final_target = handle_naming_conflict(&target_path)?;

        if config.verbose {
            println!("📂 Categorizing '{}' as '{}' (extension: {})",
                filename, category, extension);
        }

        log_action(config, "Moving",
            &file_path.display().to_string(),
            &final_target.display().to_string());

        if !config.dry_run {
            // Create directory if it doesn't exist
            if !target_dir.exists() {
                fs::create_dir_all(&target_dir)?;
                if config.verbose {
                    println!("  📁 Created directory: {}", category);
                }
            }

            // Move the file
            fs::rename(&file_path, &final_target)?;
        }

        files_organized += 1;
        *category_counts.entry(category).or_insert(0) += 1;
    }

    // Print summary
    if !category_counts.is_empty() {
        println!("\n📊 Organization Summary:");
        let mut sorted_categories: Vec<_> = category_counts.iter().collect();
        sorted_categories.sort_by(|a, b| b.1.cmp(a.1));

        for (category, count) in sorted_categories {
            println!("  {} files → {} folder", count, category);
        }
    }

    Ok(files_organized)
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
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn test_extension_mapping() {
        let ext_map = create_extension_map();

        // Test that common extensions are mapped correctly
        let mut ext_to_category: HashMap<String, String> = HashMap::new();
        for (category, extensions) in &ext_map {
            for &ext in extensions {
                ext_to_category.insert(ext.to_string(), category.clone());
            }
        }

        assert_eq!(ext_to_category.get("jpg"), Some(&"images".to_string()));
        assert_eq!(ext_to_category.get("mp4"), Some(&"videos".to_string()));
        assert_eq!(ext_to_category.get("pdf"), Some(&"documents".to_string()));
        assert_eq!(ext_to_category.get("rs"), Some(&"code".to_string()));
    }

    #[test]
    fn test_should_skip_system_files() {
        assert!(should_skip_file(".DS_Store"));
        assert!(should_skip_file("Thumbs.db"));
        assert!(should_skip_file(".gitignore"));
        assert!(!should_skip_file("document.pdf"));
        assert!(!should_skip_file("image.jpg"));
    }

    #[test]
    fn test_handle_naming_conflict() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path();

        // Create a file that will cause a conflict
        let original_file = temp_path.join("test.txt");
        File::create(&original_file)?;

        // Test conflict resolution
        let result = handle_naming_conflict(&original_file)?;
        assert_eq!(result, temp_path.join("test_1.txt"));

        // Create the conflicting file and test again
        File::create(&result)?;
        let result2 = handle_naming_conflict(&original_file)?;
        assert_eq!(result2, temp_path.join("test_2.txt"));

        Ok(())
    }
}
