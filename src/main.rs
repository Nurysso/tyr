use chrono::Local;
use serde::Deserialize;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process;

mod organizer;
use organizer::categorise::{FileOrganizerConfig, TuiApp};
use organizer::filename::{FilenameTuiApp, SimilarityConfig};
use organizer::intelligent::{IntelligentConfig, IntelligentTuiApp};

/// Main configuration structure that includes all settings
#[derive(Debug, Clone, Deserialize)]
pub struct KondoConfig {
    #[serde(default)]
    pub log_file: Option<String>,

    #[serde(default)]
    pub enable_smart_grouping: bool,

    #[serde(default)]
    pub enable_intelligent_grouping: bool,

    #[serde(default)]
    pub similarity_config: SimilarityConfigToml,

    #[serde(default)]
    pub intelligent_config: IntelligentConfigToml,
}

/// TOML representation of similarity config
#[derive(Debug, Clone, Deserialize)]
pub struct SimilarityConfigToml {
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
}

/// TOML representation of intelligent config
#[derive(Debug, Clone, Deserialize)]
pub struct IntelligentConfigToml {
    #[serde(default = "default_max_lines_to_read")]
    pub max_lines_to_read: usize,

    #[serde(default = "default_min_cluster_size")]
    pub min_cluster_size: usize,

    #[serde(default = "default_max_clusters")]
    pub max_clusters: usize,

    #[serde(default = "default_filename_similarity_weight")]
    pub filename_similarity_weight: f64,

    #[serde(default = "default_content_similarity_weight")]
    pub content_similarity_weight: f64,

    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f64,

    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
}

// Default functions for serde - Similarity Config
fn default_levenshtein_threshold() -> f64 { 0.7 }
fn default_jaccard_threshold() -> f64 { 0.5 }
fn default_levenshtein_weight() -> f64 { 0.6 }
fn default_jaccard_weight() -> f64 { 0.4 }
fn default_min_similarity_score() -> f64 { 0.65 }

// Default functions for serde - Intelligent Config
fn default_max_lines_to_read() -> usize { 100 }
fn default_min_cluster_size() -> usize { 2 }
fn default_max_clusters() -> usize { 20 }
fn default_filename_similarity_weight() -> f64 { 0.3 }
fn default_content_similarity_weight() -> f64 { 0.7 }
fn default_similarity_threshold() -> f64 { 0.65 }
fn default_max_iterations() -> usize { 100 }

impl Default for SimilarityConfigToml {
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

impl Default for IntelligentConfigToml {
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

impl Default for KondoConfig {
    fn default() -> Self {
        Self {
            log_file: None,
            enable_smart_grouping: false,
            enable_intelligent_grouping: false,
            similarity_config: SimilarityConfigToml::default(),
            intelligent_config: IntelligentConfigToml::default(),
        }
    }
}

/// Convert TOML config to runtime config
impl From<SimilarityConfigToml> for SimilarityConfig {
    fn from(toml_config: SimilarityConfigToml) -> Self {
        SimilarityConfig {
            levenshtein_threshold: toml_config.levenshtein_threshold,
            jaccard_threshold: toml_config.jaccard_threshold,
            levenshtein_weight: toml_config.levenshtein_weight,
            jaccard_weight: toml_config.jaccard_weight,
            min_similarity_score: toml_config.min_similarity_score,
        }
    }
}

impl From<IntelligentConfigToml> for IntelligentConfig {
    fn from(toml_config: IntelligentConfigToml) -> Self {
        IntelligentConfig {
            max_lines_to_read: toml_config.max_lines_to_read,
            min_cluster_size: toml_config.min_cluster_size,
            max_clusters: toml_config.max_clusters,
            filename_similarity_weight: toml_config.filename_similarity_weight,
            content_similarity_weight: toml_config.content_similarity_weight,
            similarity_threshold: toml_config.similarity_threshold,
            max_iterations: toml_config.max_iterations,
        }
    }
}

/// Gets the config directory path in a cross-platform way
fn get_config_dir() -> std::io::Result<PathBuf> {
    let config_dir = if cfg!(target_os = "windows") {
        // Windows: Use %APPDATA%\kondo
        let appdata = env::var("APPDATA").map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine APPDATA directory",
            )
        })?;
        PathBuf::from(appdata).join("kondo")
    } else {
        // Unix/Linux/macOS: Use ~/.config/kondo
        let home = env::var("HOME").map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine HOME directory",
            )
        })?;
        PathBuf::from(home).join(".config").join("kondo")
    };

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
        println!("Created config directory: {}", config_dir.display());
    }

    Ok(config_dir)
}

/// Gets the config file path: Windows: %APPDATA%\kondo\kondo.toml, Unix: ~/.config/kondo/kondo.toml
fn get_config_path() -> std::io::Result<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("kondo.toml"))
}

/// Gets the default log file path: Windows: %APPDATA%\kondo\kondo.log, Unix: ~/.config/kondo/kondo.log
fn get_default_log_path() -> std::io::Result<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("kondo.log"))
}

/// Load configuration from kondo.toml or create default
fn load_kondo_config() -> KondoConfig {
    let config_path = match get_config_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Warning: Could not determine config path: {}", e);
            return KondoConfig::default();
        }
    };

    if config_path.exists() {
        // Try to read and parse config using proper TOML deserialization
        match fs::read_to_string(&config_path) {
            Ok(content) => {
                match toml::from_str::<KondoConfig>(&content) {
                    Ok(mut config) => {
                        // Convert relative log path to absolute if needed
                        if let Some(ref log_file) = config.log_file {
                            if log_file != "none" && !log_file.is_empty() {
                                let log_path = PathBuf::from(log_file);
                                // If it's a relative path, make it absolute relative to config dir
                                if log_path.is_relative() {
                                    if let Ok(config_dir) = get_config_dir() {
                                        config.log_file = Some(config_dir.join(log_path).to_string_lossy().to_string());
                                    }
                                }
                            } else {
                                config.log_file = None;
                            }
                        }

                        // println!("✓ Loaded configuration from: {}", config_path.display());
                        return config;
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not parse config file: {}", e);
                        eprintln!("Using default configuration...");
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Could not read config file: {}", e);
            }
        }
    } else {
        // Create default config file with logging enabled
        let default_log_path = match get_default_log_path() {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Warning: Could not determine log path: {}", e);
                return KondoConfig::default();
            }
        };

        // Convert path to string with forward slashes for cross-platform TOML compatibility
        let log_path_str = default_log_path
            .to_string_lossy()
            .replace('\\', "/");

        let config_content = format!(
            r#"# Kondo File Organizer Configuration
batch_size = 100

# Enable smart grouping using filename similarity detection
# When enabled, files with similar names will be grouped together
# even if they have different extensions
enable_smart_grouping = false

# Enable intelligent grouping using ML clustering with TF-IDF
# This analyzes file content and uses advanced clustering algorithms
# More powerful than smart grouping but slower
enable_intelligent_grouping = false

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

# Smart grouping configuration (used in filename similarity mode)
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

# Intelligent grouping configuration (ML-based clustering)
[intelligent_config]
# Maximum number of lines to read from text files for content analysis
max_lines_to_read = 100

# Minimum number of files required to form a cluster/group
min_cluster_size = 2

# Maximum number of clusters to create
# Higher values = more granular grouping
max_clusters = 20

# Weight for filename similarity (0.0 to 1.0)
# How much the filename affects grouping decisions
filename_similarity_weight = 0.3

# Weight for content similarity (0.0 to 1.0)
# How much file content affects grouping decisions (for text files)
# Note: filename_similarity_weight + content_similarity_weight should = 1.0
content_similarity_weight = 0.7

# Minimum similarity threshold for grouping files (0.0 to 1.0)
# Higher = files must be more similar to be grouped together
similarity_threshold = 0.65

# Maximum iterations for K-means clustering algorithm
max_iterations = 100

# Define your custom categories (used when intelligent grouping is disabled)
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
            log_path_str
        );

        if let Err(e) = fs::write(&config_path, config_content) {
            eprintln!("!  Could not create config file: {}", e);
        } else {
            println!("✓ Created default config at: {}", config_path.display());
        }

        return KondoConfig {
            log_file: Some(log_path_str),
            enable_smart_grouping: false,
            enable_intelligent_grouping: false,
            similarity_config: SimilarityConfigToml::default(),
            intelligent_config: IntelligentConfigToml::default(),
        };
    }

    KondoConfig::default()
}

/// Log a message to the configured log file
fn log_to_file(log_path: &Option<String>, message: &str) {
    if let Some(path_str) = log_path {
        let path = PathBuf::from(path_str);
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let log_message = format!("[{}] {}\n", timestamp, message);

        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = file.write_all(log_message.as_bytes());
        }
    }
}

fn print_help() {
    println!("╔═══════════════════════════════════════════════════╗");
    println!("║                                                   ║");
    println!("║   ██╗  ██╗ ██████╗ ███╗   ██╗██████╗  ██████╗     ║");
    println!("║   ██║ ██╔╝██╔═══██╗████╗  ██║██╔══██╗██╔═══██╗    ║");
    println!("║   █████╔╝ ██║   ██║██╔██╗ ██║██║  ██║██║   ██║    ║");
    println!("║   ██╔═██╗ ██║   ██║██║╚██╗██║██║  ██║██║   ██║    ║");
    println!("║   ██║  ██╗╚██████╔╝██║ ╚████║██████╔╝╚██████╔╝    ║");
    println!("║   ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝╚═════╝  ╚═════╝     ║");
    println!("║    ML-Powered • Blazingly Fast • Beautiful TUI    ║");
    println!("║                                                   ║");
    println!("╚═══════════════════════════════════════════════════╝");
    println!("USAGE:");
    println!("    kondo [OPTIONS] [DIRECTORY]");
    println!("\nOPTIONS:");
    println!("    -c, --categorize    Organize files by category (images, videos, documents, etc.)");
    println!("    -f, --filename      Group similar files based on filename patterns");
    println!("    -i, --intelligent   Use ML-based clustering with TF-IDF content analysis");
    println!("    -nui, --no-ui       Skip UI and automatically organize files");
    println!("    -h, --help          Show this help message");
}

fn run_categorize_mode(target_dir: PathBuf, kondo_config: &KondoConfig, no_ui: bool) -> std::io::Result<()> {
    let config_path = get_config_path()?;

    log_to_file(
        &kondo_config.log_file,
        &format!("=== Starting Kondo (Categorize Mode - No UI: {}) ===", no_ui),
    );
    log_to_file(
        &kondo_config.log_file,
        &format!("Target directory: {}", target_dir.display()),
    );

    println!("Kondo - Categorize Mode");

    // Load or create config
    let config = if config_path.exists() {
        match FileOrganizerConfig::load_from_file(&config_path) {
            Ok(cfg) => {
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
        println!("ℹ  No config file found, creating default config...");
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

    // Launch TUI or auto-organize
    let mut app = TuiApp::new(config, target_dir);

    let result = if no_ui {
        app.auto_organize()
    } else {
        app.run()
    };

    // Log completion
    match &result {
        Ok(_) => {
            log_to_file(
                &kondo_config.log_file,
                "Organization completed successfully",
            );
            println!("\n✦ File organization complete!");
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

fn run_filename_mode(target_dir: PathBuf, kondo_config: &KondoConfig, no_ui: bool) -> std::io::Result<()> {
    log_to_file(
        &kondo_config.log_file,
        &format!("=== Starting Kondo (Filename Similarity Mode - No UI: {}) ===", no_ui),
    );
    log_to_file(
        &kondo_config.log_file,
        &format!("Target directory: {}", target_dir.display()),
    );

    println!("Kondo - Filename Similarity Mode");

    // Load similarity config from kondo.toml
    let similarity_config: SimilarityConfig = kondo_config.similarity_config.clone().into();

    log_to_file(
        &kondo_config.log_file,
        &format!("Using similarity config: min_score={:.2}, lev_weight={:.2}, jac_weight={:.2}",
            similarity_config.min_similarity_score,
            similarity_config.levenshtein_weight,
            similarity_config.jaccard_weight
        ),
    );

    // Launch TUI or auto-organize
    let mut app = FilenameTuiApp::new(target_dir, similarity_config);

    let result = if no_ui {
        app.auto_organize()
    } else {
        app.run()
    };

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

fn run_intelligent_mode(target_dir: PathBuf, kondo_config: &KondoConfig, no_ui: bool) -> std::io::Result<()> {
    log_to_file(
        &kondo_config.log_file,
        &format!("=== Starting Kondo (Intelligent ML Mode - No UI: {}) ===", no_ui),
    );
    log_to_file(
        &kondo_config.log_file,
        &format!("Target directory: {}", target_dir.display()),
    );

    println!("Kondo - Intelligent ML Mode");

    // Load intelligent config from kondo.toml
    let intelligent_config: IntelligentConfig = kondo_config.intelligent_config.clone().into();

    log_to_file(
        &kondo_config.log_file,
        &format!(
            "Using intelligent config: max_clusters={}, min_cluster_size={}, filename_weight={:.2}, content_weight={:.2}",
            intelligent_config.max_clusters,
            intelligent_config.min_cluster_size,
            intelligent_config.filename_similarity_weight,
            intelligent_config.content_similarity_weight
        ),
    );

    // Launch TUI or auto-analyze
    let mut app = IntelligentTuiApp::new(intelligent_config, target_dir);

    let result = if no_ui {
        app.auto_analyze()
    } else {
        app.run()
    };

    // Log completion
    match &result {
        Ok(_) => {
            log_to_file(
                &kondo_config.log_file,
                "Intelligent analysis completed successfully",
            );
            println!("\n✦ Intelligent analysis complete!");
        }
        Err(e) => {
            log_to_file(
                &kondo_config.log_file,
                &format!("Error during intelligent analysis: {}", e),
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

    // Check for -nui flag
    let no_ui = args.contains(&"-nui".to_string()) || args.contains(&"--no-ui".to_string());

    let mode = &args[1];

    // Parse arguments
    match mode.as_str() {
        "-h" | "--help" => {
            print_help();
            process::exit(0);
        }
        "-c" | "--categorize" => {
            // Find target directory (skip -nui flag if present)
            let target_dir = if args.len() > 2 {
                let mut path_arg = None;
                for (i, arg) in args.iter().enumerate() {
                    if i > 1 && arg != "-nui" && arg != "--no-ui" {
                        path_arg = Some(arg);
                        break;
                    }
                }

                if let Some(path) = path_arg {
                    PathBuf::from(path)
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
                }
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

            if let Err(e) = run_categorize_mode(target_dir, &kondo_config, no_ui) {
                eprintln!("✗ Error: {}", e);
                log_to_file(&kondo_config.log_file, &format!("Fatal error: {}", e));
                process::exit(1);
            }
        }
        "-f" | "--filename" => {
            // Find target directory (skip -nui flag if present)
            let target_dir = if args.len() > 2 {
                let mut path_arg = None;
                for (i, arg) in args.iter().enumerate() {
                    if i > 1 && arg != "-nui" && arg != "--no-ui" {
                        path_arg = Some(arg);
                        break;
                    }
                }

                if let Some(path) = path_arg {
                    PathBuf::from(path)
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
                }
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

            if let Err(e) = run_filename_mode(target_dir, &kondo_config, no_ui) {
                eprintln!("✗ Error: {}", e);
                log_to_file(&kondo_config.log_file, &format!("Fatal error: {}", e));
                process::exit(1);
            }
        }
        "-i" | "--intelligent" => {
            // Find target directory (skip -nui flag if present)
            let target_dir = if args.len() > 2 {
                let mut path_arg = None;
                for (i, arg) in args.iter().enumerate() {
                    if i > 1 && arg != "-nui" && arg != "--no-ui" {
                        path_arg = Some(arg);
                        break;
                    }
                }

                if let Some(path) = path_arg {
                    PathBuf::from(path)
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
                }
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

            if let Err(e) = run_intelligent_mode(target_dir, &kondo_config, no_ui) {
                eprintln!("✗ Error: {}", e);
                log_to_file(&kondo_config.log_file, &format!("Fatal error: {}", e));
                process::exit(1);
            }
        }
        "-nui" | "--no-ui" => {
            eprintln!("✗ Error: -nui flag must be used with -c, -f, or -i mode");
            eprintln!("\nExamples:");
            eprintln!("  kondo -c -nui /path/to/folder");
            eprintln!("  kondo -f -nui /path/to/folder");
            eprintln!("  kondo -i -nui /path/to/folder");
            process::exit(1);
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
