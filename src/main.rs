use chrono::Local;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process;

mod organizer;
use organizer::categorise::{FileOrganizerConfig, TuiApp};
use organizer::filename::{FilenameTuiApp, SimilarityConfig};

/// Configuration structure that includes log file path
#[derive(Debug, Clone)]
pub struct KondoConfig {
    pub log_file: Option<PathBuf>,
}

impl Default for KondoConfig {
    fn default() -> Self {
        Self { log_file: None }
    }
}

/// Gets the config file path: ~/.config/kondo/kondo.toml
fn get_config_path() -> std::io::Result<PathBuf> {
    let home_dir = if let Ok(home) = env::var("HOME") {
        PathBuf::from(home)
    } else if let Ok(user_profile) = env::var("USERPROFILE") {
        PathBuf::from(user_profile)
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        ));
    };

    let config_dir = home_dir.join(".config").join("kondo");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
        println!("Created config directory: {}", config_dir.display());
    }

    Ok(config_dir.join("kondo.toml"))
}

/// Gets the default log file path: ~/.config/kondo/kondo.log
fn get_default_log_path() -> std::io::Result<PathBuf> {
    let home_dir = if let Ok(home) = env::var("HOME") {
        PathBuf::from(home)
    } else if let Ok(user_profile) = env::var("USERPROFILE") {
        PathBuf::from(user_profile)
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        ));
    };

    let config_dir = home_dir.join(".config").join("kondo");
    Ok(config_dir.join("kondo.log"))
}

/// Load configuration from kondo.toml or create default
fn load_kondo_config() -> KondoConfig {
    let config_path = match get_config_path() {
        Ok(path) => path,
        Err(_) => return KondoConfig::default(),
    };

    if config_path.exists() {
        // Try to read and parse config
        if let Ok(content) = fs::read_to_string(&config_path) {
            // Simple TOML parsing for log_file key
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("log_file") && line.contains('=') {
                    if let Some(value) = line.split('=').nth(1) {
                        let value = value.trim().trim_matches('"').trim_matches('\'');
                        if !value.is_empty() && value != "none" {
                            return KondoConfig {
                                log_file: Some(PathBuf::from(value)),
                            };
                        }
                    }
                }
            }
        }
    } else {
        // Create default config file with logging enabled
        let default_log_path = match get_default_log_path() {
            Ok(path) => path,
            Err(_) => return KondoConfig::default(),
        };

        let config_content = format!(
            r#"# Kondo File Organizer Configuration
batch_size = 100

# Enable smart grouping using ML-based similarity detection
# When enabled, files with similar names will be grouped together
# even if they have different extensions
enable_smart_grouping = false
log_file = "{}"
# Files/patterns to skip during organization
skip_patterns = [
    ".DS_Store",
    "Thumbs.db",
    ".git",
    ".gitignore",
    "desktop.ini",
    ".localized"
]

# Smart grouping configuration (only used if enable_smart_grouping = true)
[similarity_config]
# Levenshtein distance threshold (0.0 to 1.0)
# Higher = stricter matching. Measures character-level similarity.
levenshtein_threshold = 0.7

# Jaccard similarity threshold (0.0 to 1.0)
# Higher = stricter matching. Measures word/token overlap.
jaccard_threshold = 0.5

# Weight for Levenshtein distance in final score (0.0 to 1.0)
levenshtein_weight = 0.6

# Weight for Jaccard similarity in final score (0.0 to 1.0)
# Note: levenshtein_weight + jaccard_weight should = 1.0
jaccard_weight = 0.4

# Minimum similarity score to group files together (0.0 to 1.0)
# Higher = files must be more similar to be grouped
# 0.65 is a good balance for most use cases
min_similarity_score = 0.65

# Define your custom categories
# Each category has:
#   - extensions: list of file extensions (without dot)
#   - folder_name: optional custom folder name (defaults to category key)

[categories.images]
extensions = ["jpg", "jpeg", "png", "gif", "bmp", "svg", "webp", "tiff", "ico", "heic", "raw", "cr2", "nef", "orf", "sr2"]
folder_name = "Images"

[categories.videos]
extensions = ["mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "m4v", "3gp", "mpg", "mpeg", "vob"]
folder_name = "Videos"

[categories.audio]
extensions = ["mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus", "aiff", "ape", "alac"]
folder_name = "Music"

[categories.documents]
extensions = ["pdf", "doc", "docx", "txt", "rtf", "odt", "pages", "tex", "md", "epub", "mobi"]
folder_name = "Documents"

[categories.spreadsheets]
extensions = ["xls", "xlsx", "csv", "ods", "numbers"]
folder_name = "Spreadsheets"

[categories.presentations]
extensions = ["ppt", "pptx", "odp", "key"]
folder_name = "Presentations"

[categories.archives]
extensions = ["zip", "rar", "7z", "tar", "gz", "bz2", "xz", "dmg", "pkg", "deb", "rpm", "iso"]
folder_name = "Archives"

[categories.code]
extensions = ["rs", "py", "js", "ts", "jsx", "tsx", "html", "css", "scss", "sass", "cpp", "c", "h", "hpp", "java", "go", "php", "rb", "swift", "kt", "dart", "scala", "sh", "bat", "ps1", "r", "lua", "vim"]
folder_name = "Code"

[categories.data]
extensions = ["json", "xml", "yaml", "yml", "toml", "ini", "cfg", "conf", "sql", "db", "sqlite", "mdb"]
folder_name = "Data"

[categories.executables]
extensions = ["exe", "msi", "app", "deb", "rpm", "dmg", "pkg", "appimage", "run"]
folder_name = "Applications"

[categories.fonts]
extensions = ["ttf", "otf", "woff", "woff2", "eot"]
folder_name = "Fonts"

[categories.ebooks]
extensions = ["epub", "mobi", "azw", "azw3", "cbr", "cbz"]
folder_name = "Ebooks"

[categories.3d_models]
extensions = ["obj", "fbx", "stl", "blend", "dae", "3ds", "max", "gltf", "glb"]
folder_name = "3D Models"

[categories.design]
extensions = ["psd", "ai", "xd", "sketch", "fig", "indd", "cdr"]
folder_name = "Design Files"

# Add your custom categories below:
# [categories.my_custom_category]
# extensions = ["ext1", "ext2", "ext3"]
# folder_name = "My Custom Folder"

"#,
            default_log_path.display()
        );

        if let Err(e) = fs::write(&config_path, config_content) {
            eprintln!("!  Could not create config file: {}", e);
        } else {
            println!("✓ Created default config at: {}", config_path.display());
        }

        return KondoConfig {
            log_file: Some(default_log_path),
        };
    }

    KondoConfig::default()
}

/// Log a message to the configured log file
fn log_to_file(log_path: &Option<PathBuf>, message: &str) {
    if let Some(path) = log_path {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let log_message = format!("[{}] {}\n", timestamp, message);

        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(log_message.as_bytes());
        }
    }
}

fn print_help() {
    // println!(" Kondo - File Organizer");
    println!("╔═══════════════════════════════════════════════════╗");
    println!("║                                                   ║");
    println!("║   ██╗  ██╗ ██████╗ ███╗   ██╗██████╗  ██████╗     ║");
    println!("║   ██║ ██╔╝██╔═══██╗████╗  ██║██╔══██╗██╔═══██╗    ║");
    println!("║   █████╔╝ ██║   ██║██╔██╗ ██║██║  ██║██║   ██║    ║");
    println!("║   ██╔═██╗ ██║   ██║██║╚██╗██║██║  ██║██║   ██║    ║");
    println!("║   ██║  ██╗╚██████╔╝██║ ╚████║██████╔╝╚██████╔╝    ║");
    println!("║   ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝╚═════╝  ╚═════╝     ║");
    // println!("║                                                ║");
    // println!("║   The Art of Tidying Your Digital Life   ║");
    // println!("║                                               ║");
    println!("║    ML-Powered • Blazingly Fast • Beautiful TUI    ║");
    println!("║                                                   ║");
    println!("╚═══════════════════════════════════════════════════╝");
    println!("USAGE:");
    println!("    kondo [OPTIONS] [DIRECTORY]");
    println!("OPTIONS:");
    println!(
        "    -c, --categorize    Organize files by category (images, videos, documents, etc.)"
    );
    println!("    -f, --filename      Group similar files based on filename patterns");
    println!("    -h, --help          Show this help message\n");
    // println!("EXAMPLES:");
    // println!("    kondo -c                  # Categorize files in current directory");
    // println!("    kondo -c ~/Downloads      # Categorize files in Downloads folder");
    // println!("    kondo -f                  # Group similar files in current directory");
    // println!("    kondo -f ~/Documents      # Group similar files in Documents folder\n");
    // println!("CONFIG:");
    // println!("    Config file location: ~/.config/kondo/kondo.toml");
    // println!("    Log file location: Set in kondo.toml (default: ~/.config/kondo/kondo.log)");
    // println!("    Edit config file to customize categories, extensions, and logging");
}

fn run_categorize_mode(target_dir: PathBuf, kondo_config: &KondoConfig) -> std::io::Result<()> {
    let config_path = get_config_path()?;

    log_to_file(
        &kondo_config.log_file,
        "=== Starting Kondo (Categorize Mode) ===",
    );
    log_to_file(
        &kondo_config.log_file,
        &format!("Target directory: {}", target_dir.display()),
    );

    println!(" Kondo - Categorize Mode");
    println!(" Config location: {}", config_path.display());

    if let Some(log_path) = &kondo_config.log_file {
        println!(" Logging to: {}", log_path.display());
    }

    // Load or create config
    let config = if config_path.exists() {
        println!("✓ Loading existing config...");
        match FileOrganizerConfig::load_from_file(&config_path) {
            Ok(cfg) => {
                println!("✓ Config loaded successfully");
                log_to_file(&kondo_config.log_file, "Config loaded successfully");
                cfg
            }
            Err(e) => {
                eprintln!("!  Failed to load config: {}", e);
                println!("Using default configuration...");
                log_to_file(
                    &kondo_config.log_file,
                    &format!("Failed to load config: {}", e),
                );
                FileOrganizerConfig::default()
            }
        }
    } else {
        println!(" No config file found, creating default config...");
        let default_config = FileOrganizerConfig::default();

        if let Err(e) = default_config.save_to_file(&config_path) {
            eprintln!("! Could not save default config: {}", e);
            log_to_file(
                &kondo_config.log_file,
                &format!("Could not save default config: {}", e),
            );
        } else {
            println!("✓ Default config created at: {}", config_path.display());
            println!("   Edit this file to customize categories!");
            log_to_file(&kondo_config.log_file, "Created default config");
        }

        default_config
    };

    println!(" Target directory: {}\n", target_dir.display());

    // Launch TUI
    let mut app = TuiApp::new(config, target_dir);
    let result = app.run();

    // Log completion
    match &result {
        Ok(_) => {
            log_to_file(
                &kondo_config.log_file,
                "Organization completed successfully",
            );
            println!("File organization complete!");
        }
        Err(e) => {
            log_to_file(
                &kondo_config.log_file,
                &format!("Error during organization: {}", e),
            );
        }
    }

    result
}

fn run_filename_mode(target_dir: PathBuf, kondo_config: &KondoConfig) -> std::io::Result<()> {
    log_to_file(
        &kondo_config.log_file,
        "=== Starting Kondo (Filename Similarity Mode) ===",
    );
    log_to_file(
        &kondo_config.log_file,
        &format!("Target directory: {}", target_dir.display()),
    );

    println!(" Kondo - Filename Similarity Mode\n");
    println!(" Target directory: {}\n", target_dir.display());

    if let Some(log_path) = &kondo_config.log_file {
        println!(" Logging to: {}\n", log_path.display());
    }

    // Launch TUI
    let config = SimilarityConfig::default();
    let mut app = FilenameTuiApp::new(target_dir, config);
    let result = app.run();

    // Get logs from the app and write them to file
    if kondo_config.log_file.is_some() {
        let logs = app.get_logs();
        for log_msg in logs {
            log_to_file(&kondo_config.log_file, &log_msg);
        }
    }

    // Log completion
    match &result {
        Ok(_) => {
            log_to_file(
                &kondo_config.log_file,
                "Organization completed successfully",
            );
            println!("\n✦ File organization complete!");

            if let Some(log_path) = &kondo_config.log_file {
                println!(" Full log available at: {}", log_path.display());
            }
        }
        Err(e) => {
            log_to_file(
                &kondo_config.log_file,
                &format!("Error during organization: {}", e),
            );
        }
    }

    result
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Load configuration
    let kondo_config = load_kondo_config();

    // No arguments - show help
    if args.len() < 2 {
        print_help();
        process::exit(0);
    }

    let mode = &args[1];

    // Parse arguments
    match mode.as_str() {
        "-h" | "--help" => {
            print_help();
            process::exit(0);
        }
        "-c" | "--categorize" => {
            let target_dir = if args.len() > 2 {
                PathBuf::from(&args[2])
            } else {
                match env::current_dir() {
                    Ok(dir) => dir,
                    Err(e) => {
                        eprintln!("✗ Error: Could not get current directory: {}", e);
                        log_to_file(
                            &kondo_config.log_file,
                            &format!("Error: Could not get current directory: {}", e),
                        );
                        process::exit(1);
                    }
                }
            };

            if !target_dir.exists() {
                eprintln!(
                    "✗ Error: Directory does not exist: {}",
                    target_dir.display()
                );
                log_to_file(
                    &kondo_config.log_file,
                    &format!("Error: Directory does not exist: {}", target_dir.display()),
                );
                process::exit(1);
            }

            if let Err(e) = run_categorize_mode(target_dir, &kondo_config) {
                eprintln!("✗ Error: {}", e);
                log_to_file(&kondo_config.log_file, &format!("Fatal error: {}", e));
                process::exit(1);
            }
        }
        "-f" | "--filename" => {
            let target_dir = if args.len() > 2 {
                PathBuf::from(&args[2])
            } else {
                match env::current_dir() {
                    Ok(dir) => dir,
                    Err(e) => {
                        eprintln!("✗ Error: Could not get current directory: {}", e);
                        log_to_file(
                            &kondo_config.log_file,
                            &format!("Error: Could not get current directory: {}", e),
                        );
                        process::exit(1);
                    }
                }
            };

            if !target_dir.exists() {
                eprintln!(
                    "✗ Error: Directory does not exist: {}",
                    target_dir.display()
                );
                log_to_file(
                    &kondo_config.log_file,
                    &format!("Error: Directory does not exist: {}", target_dir.display()),
                );
                process::exit(1);
            }

            if let Err(e) = run_filename_mode(target_dir, &kondo_config) {
                eprintln!("✗ Error: {}", e);
                log_to_file(&kondo_config.log_file, &format!("Fatal error: {}", e));
                process::exit(1);
            }
        }
        _ => {
            eprintln!("✗ Error: Unknown option '{}'", mode);
            eprintln!("\nRun 'kondo --help' for usage information");
            log_to_file(
                &kondo_config.log_file,
                &format!("Error: Unknown option '{}'", mode),
            );
            process::exit(1);
        }
    }

    log_to_file(&kondo_config.log_file, "=== Kondo session ended ===\n");
}
