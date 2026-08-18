/*
  Tyr - ML-Powered, Blazingly Fast File Organizer with a Beautiful TUI.
  Copyright (C) 2026  Dawood Khan (Nurysso)

  Contact Maintainer Nurysso [@] proton.me
  This program is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  This program is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

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

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BIN_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, Clone, Deserialize)]
pub struct TyrConfig {
    #[serde(default)]
    pub log_file: Option<String>,
    #[serde(default = "default_skip_patterns")]
    pub skip_patterns: Vec<String>,
    #[serde(default = "default_true")]
    pub skip_hidden_files: bool,
    #[serde(default)]
    pub similarity_config: SimilarityConfigToml,
    #[serde(default)]
    pub intelligent_config: IntelligentConfigToml,
}

impl Default for TyrConfig {
    fn default() -> Self {
        Self {
            log_file: None,
            // enable_smart_grouping: false,
            // enable_intelligent_grouping: false,
            skip_patterns: default_skip_patterns(),
            skip_hidden_files: true,
            similarity_config: SimilarityConfigToml::default(),
            intelligent_config: IntelligentConfigToml::default(),
        }
    }
}

fn default_skip_patterns() -> Vec<String> {
    [
        ".DS_Store",
        "Thumbs.db",
        ".git",
        ".gitignore",
        "desktop.ini",
        ".localized",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn default_true() -> bool {
    true
}

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

impl Default for SimilarityConfigToml {
    fn default() -> Self {
        Self {
            levenshtein_threshold: default_levenshtein_threshold(),
            jaccard_threshold: default_jaccard_threshold(),
            levenshtein_weight: default_levenshtein_weight(),
            jaccard_weight: default_jaccard_weight(),
            min_similarity_score: default_min_similarity_score(),
        }
    }
}

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

impl Default for IntelligentConfigToml {
    fn default() -> Self {
        Self {
            max_lines_to_read: default_max_lines_to_read(),
            min_cluster_size: default_min_cluster_size(),
            max_clusters: default_max_clusters(),
            filename_similarity_weight: default_filename_similarity_weight(),
            content_similarity_weight: default_content_similarity_weight(),
            similarity_threshold: default_similarity_threshold(),
            max_iterations: default_max_iterations(),
        }
    }
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
fn default_max_lines_to_read() -> usize {
    100
}
fn default_min_cluster_size() -> usize {
    2
}
fn default_max_clusters() -> usize {
    20
}
fn default_filename_similarity_weight() -> f64 {
    0.3
}
fn default_content_similarity_weight() -> f64 {
    0.7
}
fn default_similarity_threshold() -> f64 {
    0.65
}
fn default_max_iterations() -> usize {
    100
}

fn to_similarity_config(
    c: &SimilarityConfigToml,
    skip_patterns: Vec<String>,
    skip_hidden_files: bool,
) -> SimilarityConfig {
    SimilarityConfig {
        levenshtein_threshold: c.levenshtein_threshold,
        jaccard_threshold: c.jaccard_threshold,
        levenshtein_weight: c.levenshtein_weight,
        jaccard_weight: c.jaccard_weight,
        min_similarity_score: c.min_similarity_score,
        skip_patterns,
        skip_hidden_files,
    }
}

fn to_intelligent_config(
    c: &IntelligentConfigToml,
    skip_patterns: Vec<String>,
    skip_hidden_files: bool,
) -> IntelligentConfig {
    IntelligentConfig {
        max_lines_to_read: c.max_lines_to_read,
        min_cluster_size: c.min_cluster_size,
        max_clusters: c.max_clusters,
        filename_similarity_weight: c.filename_similarity_weight,
        content_similarity_weight: c.content_similarity_weight,
        similarity_threshold: c.similarity_threshold,
        max_iterations: c.max_iterations,
        skip_patterns,
        skip_hidden_files,
    }
}

fn platform_config_dir() -> std::io::Result<PathBuf> {
    let (var, sub) = if cfg!(target_os = "windows") {
        ("APPDATA", "tyr")
    } else {
        ("HOME", ".config/tyr")
    };
    env::var(var)
        .map(|base| PathBuf::from(base).join(sub))
        .map_err(|_| missing_env_err(var))
}

fn get_config_dir() -> std::io::Result<PathBuf> {
    let dir = platform_config_dir()?;
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
        println!("Created config directory: {}", dir.display());
    }
    Ok(dir)
}

fn missing_env_err(var: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("Could not determine {var} directory"),
    )
}

fn get_config_path() -> std::io::Result<PathBuf> {
    Ok(get_config_dir()?.join("tyr.toml"))
}

fn get_default_log_path() -> std::io::Result<PathBuf> {
    Ok(get_config_dir()?.join("tyr.log"))
}

fn load_tyr_config() -> TyrConfig {
    let Ok(config_path) = get_config_path() else {
        eprintln!("Warning: Could not determine config path");
        return TyrConfig::default();
    };

    if config_path.exists() {
        return read_tyr_config(&config_path).unwrap_or_default();
    }
    write_default_config(&config_path)
}

fn read_tyr_config(config_path: &PathBuf) -> Option<TyrConfig> {
    let content = fs::read_to_string(config_path)
        .map_err(|e| eprintln!("Warning: Could not read config file: {e}"))
        .ok()?;
    let mut config: TyrConfig = toml::from_str(&content)
        .map_err(|e| eprintln!("Warning: Could not parse config file: {e}"))
        .ok()?;
    config.log_file = absolute_log_path(config.log_file);
    Some(config)
}

/// Resolves a configured (possibly relative, possibly "none") log path
/// against the config directory.
fn absolute_log_path(log_file: Option<String>) -> Option<String> {
    let log_file = log_file.filter(|f| f != "none" && !f.is_empty())?;
    let path = PathBuf::from(&log_file);
    if path.is_relative() {
        if let Ok(dir) = get_config_dir() {
            return Some(dir.join(path).to_string_lossy().to_string());
        }
    }
    Some(log_file)
}

fn write_default_config(config_path: &PathBuf) -> TyrConfig {
    let Ok(log_path) = get_default_log_path() else {
        eprintln!("Warning: Could not determine log path");
        return TyrConfig::default();
    };
    let log_path_str = log_path.to_string_lossy().replace('\\', "/");

    match fs::write(config_path, default_config_toml(&log_path_str)) {
        Ok(_) => println!(
            "\u{2713} Created default config at: {}",
            config_path.display()
        ),
        Err(e) => eprintln!("!  Could not create config file: {e}"),
    }

    TyrConfig {
        log_file: Some(log_path_str),
        ..TyrConfig::default()
    }
}

fn log_to_file(log_path: &Option<String>, message: &str) {
    let Some(path) = log_path else { return };
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{timestamp}] {message}\n");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

fn print_help() {
    println!("TTTTTTTTTTTTTTTTTTTTTTTYYYYYYY       YYYYYYYRRRRRRRRRRRRRRRRR");
    println!("T:::::::::::::::::::::TY:::::Y       Y:::::YR::::::::::::::::R");
    println!("T:::::::::::::::::::::TY:::::Y       Y:::::YR::::::RRRRRR:::::R");
    println!("T:::::TT:::::::TT:::::TY::::::Y     Y::::::YRR:::::R     R:::::R");
    println!("TTTTTT  T:::::T  TTTTTTYYY:::::Y   Y:::::YYY  R::::R     R:::::R");
    println!("        T:::::T           Y:::::Y Y:::::Y     R::::R     R:::::R");
    println!("        T:::::T            Y:::::Y:::::Y      R::::RRRRRR:::::R ");
    println!("        T:::::T             Y:::::::::Y       R:::::::::::::RR  ");
    println!("        T:::::T              Y:::::::Y        R::::RRRRRR:::::R ");
    println!("        T:::::T               Y:::::Y         R::::R     R:::::R");
    println!("        T:::::T               Y:::::Y         R::::R     R:::::R");
    println!("        T:::::T               Y:::::Y         R::::R     R:::::R");
    println!("      TT:::::::TT             Y:::::Y       RR:::::R     R:::::R");
    println!("      T:::::::::T          YYYY:::::YYYY    R::::::R     R:::::R");
    println!("      T:::::::::T          Y:::::::::::Y    R::::::R     R:::::R");
    println!("      TTTTTTTTTTT          YYYYYYYYYYYYY    RRRRRRRR     RRRRRRR");
    println!(" ML-Powered \u{2022} Blazingly Fast \u{2022} Beautiful TUI");
    println!("\nUSAGE:\n    tyr [OPTIONS] [DIRECTORY]");
    println!("\nOPTIONS:");
    println!("    -v, --version       Show version");
    println!(
        "    -c, --categorize    Organize files by category (images, videos, documents, etc.)"
    );
    println!("    -f, --filename      Group similar files based on filename patterns");
    println!("    -i, --intelligent   Use ML-based clustering with TF-IDF content analysis");
    println!("    -nui, --no-ui       Skip UI and automatically organize files");
    println!("    -h, --help          Show this help message");
}

fn print_version() {
    println!("{} v{}", BIN_NAME, VERSION);
    println!(
    "{}  Copyright (C) 2026  Nurysso\n\
    This program is free software: you can redistribute it and/or modify\n\
    it under the terms of the GNU General Public License as published by\n\
    the Free Software Foundation, either version 3 of the License, or\n\
    (at your option) any later version.\n\n\
    This program is distributed in the hope that it will be useful,\n\
    but WITHOUT ANY WARRANTY; without even the implied warranty of\n\
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the\n\
    GNU General Public License for details.\n\n\
    You should have received a copy of the GNU General Public License\n\
    along with this program.  If not, see <https://www.gnu.org/licenses/>.",
    BIN_NAME
);
}

/// The first positional argument that isn't `-nui`/`--no-ui`, or the
/// current directory if none was given.
fn resolve_target_dir(args: &[String]) -> std::io::Result<PathBuf> {
    let path_arg = args
        .iter()
        .skip(2)
        .find(|a| a.as_str() != "-nui" && a.as_str() != "--no-ui");
    match path_arg {
        Some(path) => Ok(PathBuf::from(path)),
        None => env::current_dir(),
    }
}

fn exit_with_error(tyr_config: &TyrConfig, message: String) -> ! {
    eprintln!("\u{2717} Error: {message}");
    log_to_file(&tyr_config.log_file, &message);
    process::exit(1);
}

fn require_dir(args: &[String], tyr_config: &TyrConfig) -> PathBuf {
    let target_dir = resolve_target_dir(args).unwrap_or_else(|e| {
        exit_with_error(tyr_config, format!("Could not get current directory: {e}"))
    });
    if !target_dir.exists() {
        exit_with_error(
            tyr_config,
            format!("Directory does not exist: {}", target_dir.display()),
        );
    }
    target_dir
}

fn run_categorize_mode(
    target_dir: PathBuf,
    tyr_config: &TyrConfig,
    no_ui: bool,
) -> std::io::Result<()> {
    log_to_file(
        &tyr_config.log_file,
        &format!("=== Starting Tyr (Categorize Mode - No UI: {no_ui}) ==="),
    );
    println!("Tyr - Categorize Mode");

    let config_path = get_config_path()?;
    let config = load_or_create_organizer_config(&config_path);

    let mut app = TuiApp::new(config, target_dir);
    let result = if no_ui {
        app.auto_organize()
    } else {
        app.run()
    };

    for entry in app.get_logs() {
        log_to_file(&tyr_config.log_file, &entry);
    }
    log_result(&tyr_config.log_file, &result, "Organization");
    result
}

fn load_or_create_organizer_config(config_path: &PathBuf) -> FileOrganizerConfig {
    if config_path.exists() {
        return FileOrganizerConfig::load_from_file(config_path).unwrap_or_else(|e| {
            eprintln!("!  Failed to load config: {e}\nUsing default configuration...");
            FileOrganizerConfig::default()
        });
    }
    let default_config = FileOrganizerConfig::default();
    match default_config.save_to_file(config_path) {
        Ok(_) => println!(
            "\u{2713} Default config created at: {}",
            config_path.display()
        ),
        Err(e) => eprintln!("!  Could not save default config: {e}"),
    }
    default_config
}

fn run_filename_mode(
    target_dir: PathBuf,
    tyr_config: &TyrConfig,
    no_ui: bool,
) -> std::io::Result<()> {
    log_to_file(
        &tyr_config.log_file,
        &format!("=== Starting Tyr (Filename Similarity Mode - No UI: {no_ui}) ==="),
    );
    println!("Tyr - Filename Similarity Mode");

    let similarity_config = to_similarity_config(
        &tyr_config.similarity_config,
        tyr_config.skip_patterns.clone(),
        tyr_config.skip_hidden_files,
    );
    let mut app = FilenameTuiApp::new(target_dir, similarity_config);
    let result = if no_ui {
        app.auto_organize()
    } else {
        app.run()
    };

    for line in app.get_logs() {
        log_to_file(&tyr_config.log_file, &line);
    }
    log_result(&tyr_config.log_file, &result, "Organization");
    result
}

fn run_intelligent_mode(
    target_dir: PathBuf,
    tyr_config: &TyrConfig,
    no_ui: bool,
) -> std::io::Result<()> {
    log_to_file(
        &tyr_config.log_file,
        &format!("=== Starting Tyr (Intelligent ML Mode - No UI: {no_ui}) ==="),
    );
    println!("Tyr - Intelligent ML Mode");

    let intelligent_config = to_intelligent_config(
        &tyr_config.intelligent_config,
        tyr_config.skip_patterns.clone(),
        tyr_config.skip_hidden_files,
    );
    let mut app = IntelligentTuiApp::new(intelligent_config, target_dir);
    let result = if no_ui { app.auto_analyze() } else { app.run() };
    log_result(&tyr_config.log_file, &result, "Intelligent analysis");
    result
}

fn log_result(log_file: &Option<String>, result: &std::io::Result<()>, label: &str) {
    match result {
        Ok(_) => {
            log_to_file(log_file, &format!("{label} completed successfully"));
            println!("\n\u{2726} {label} complete!");
        }
        Err(e) => log_to_file(
            log_file,
            &format!("Error during {}: {e}", label.to_lowercase()),
        ),
    }
}

fn dispatch(
    mode: &str,
    args: &[String],
    tyr_config: &TyrConfig,
    no_ui: bool,
) -> std::io::Result<()> {
    match mode {
        "-h" | "--help" => {
            print_help();
            process::exit(0);
        }
        "-c" | "--categorize" => {
            run_categorize_mode(require_dir(args, tyr_config), tyr_config, no_ui)
        }
        "-f" | "--filename" => run_filename_mode(require_dir(args, tyr_config), tyr_config, no_ui),
        "-i" | "--intelligent" => {
            run_intelligent_mode(require_dir(args, tyr_config), tyr_config, no_ui)
        }
        "-nui" | "--no-ui" => {
            eprintln!("\u{2717} Error: -nui flag must be used with -c, -f, or -i mode");
            eprintln!("\nExamples:\n  tyr -c -nui /path/to/folder\n  tyr -f -nui /path/to/folder\n  tyr -i -nui /path/to/folder");
            process::exit(1);
        }
        "-v" | "--version" => {
            print_version();
            Ok(())
        }
        other => exit_with_error(
            tyr_config,
            format!("Unknown option '{other}'\n\nRun 'tyr --help' for usage information"),
        ),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let tyr_config = load_tyr_config();

    if args.len() < 2 {
        print_help();
        process::exit(0);
    }

    let no_ui = args.iter().any(|a| a == "-nui" || a == "--no-ui");
    if let Err(e) = dispatch(&args[1], &args, &tyr_config, no_ui) {
        exit_with_error(&tyr_config, e.to_string());
    }

    log_to_file(&tyr_config.log_file, "=== Tyr session ended ===\n");
}

fn default_config_toml(log_path: &str) -> String {
    format!(
        r#"# Tyr File Organizer Configuration
batch_size = 100
log_file = "{log_path}"

# Applies to all modes (categorize, filename, intelligent)
skip_patterns = [".DS_Store", "Thumbs.db", ".git", ".gitignore", "desktop.ini", ".localized"]
skip_hidden_files = true

[similarity_config]
levenshtein_threshold = 0.7
jaccard_threshold = 0.5
levenshtein_weight = 0.6
jaccard_weight = 0.4
min_similarity_score = 0.65

[intelligent_config]
max_lines_to_read = 100
min_cluster_size = 2
max_clusters = 20
filename_similarity_weight = 0.3
content_similarity_weight = 0.7
similarity_threshold = 0.65
max_iterations = 100

# Custom categories used when intelligent/smart grouping is disabled.
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
"#
    )
}
