use clap::{Arg, Command};
use std::io;
use std::path::Path;

mod categorise;
mod filename;

use categorise::organize_by_category;
use filename::organize_by_filename;

#[derive(Debug, Clone)]
pub struct Config {
    pub directory: String,
    pub dry_run: bool,
    pub verbose: bool,
    pub strategy: OrganizeStrategy,
}

#[derive(Debug, Clone)]
pub enum OrganizeStrategy {
    Category,
    Filename,
}

fn main() -> io::Result<()> {
    let app = Command::new("Kondo File Organizer")
        .version("0.1.0")
        .author("Dawood Ailune@proton.me")
        .about("Organizes files in a directory by extension or filename patterns")
        .override_usage("kondo [Flags]")
        .arg_required_else_help(true)
        .arg(
            Arg::new("here")
                .short('C')
                .long("here")
                .help("Organize the current directory (same as -d .)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dry-run")
                .short('D')
                .long("dry-run")
                .help("Show what would be done without actually moving files")
                .action(clap::ArgAction::SetTrue),
        )
                .arg(
            Arg::new("directory")
                .short('d')
                .long("directory")
                .value_name("DIR")
                .help("Specify the directory to organize")
                .num_args(1),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("category")
                .short('c')
                .long("category")
                .help("Organize files by extension category (default)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("filename")
                .short('f')
                .long("filename")
                .help("Organize files by filename patterns")
                .action(clap::ArgAction::SetTrue),
        );

    let matches = app.get_matches();

    // Determine directory
    let directory = if matches.get_flag("here") {
        ".".to_string()
    } else if let Some(dir) = matches.get_one::<String>("directory") {
        dir.to_string()
    } else {
        eprintln!("❌ Error: You must specify a directory with -d <DIR> or use -e for current directory.");
        std::process::exit(1);
    };

    let config = Config {
        directory,
        dry_run: matches.get_flag("dry-run"),
        verbose: matches.get_flag("verbose"),
        strategy: if matches.get_flag("filename") {
            OrganizeStrategy::Filename
        } else {
            OrganizeStrategy::Category
        },
    };

    // Validate directory exists
    let path = Path::new(&config.directory);
    if !path.exists() {
        eprintln!("❌ Error: Directory '{}' does not exist", config.directory);
        std::process::exit(1);
    }
    if !path.is_dir() {
        eprintln!("❌ Error: '{}' is not a directory", config.directory);
        std::process::exit(1);
    }

    println!("🗂️  Kondo File Organizer v0.1.0");
    println!("📁 Directory: {}", config.directory);
    println!("🎯 Strategy: {:?}", config.strategy);

    if config.dry_run {
        println!("🔍 Dry run mode - no files will be moved");
    }

    if config.verbose {
        println!("📢 Verbose mode enabled");
    }

    println!();

    let result = match config.strategy {
        OrganizeStrategy::Category => organize_by_category(&config),
        OrganizeStrategy::Filename => organize_by_filename(&config),
    };

    match result {
        Ok(files_moved) => {
            if config.dry_run {
                println!("✅ Dry run complete! {} files would be organized", files_moved);
            } else {
                println!("✅ Organization complete! {} files organized", files_moved);
            }
        }
        Err(e) => {
            eprintln!("❌ Error during organization: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Utility function to check if a file should be skipped
pub fn should_skip_file(filename: &str) -> bool {
    let skip_patterns = [
        ".DS_Store",
        "Thumbs.db",
        "desktop.ini",
        ".gitkeep",
        ".gitignore",
    ];

    skip_patterns.iter().any(|&pattern| filename == pattern)
        || filename.starts_with('.')
}

/// Utility function for logging
pub fn log_action(config: &Config, action: &str, from: &str, to: &str) {
    if config.verbose || config.dry_run {
        if config.dry_run {
            println!("  [DRY RUN] {} {} -> {}", action, from, to);
        } else {
            println!("  {} {} -> {}", action, from, to);
        }
    }
}
