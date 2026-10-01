<div align="center">

# Tyr

**A blazingly fast, local file organizer with extension categorization, filename similarity matching, and ML-powered content clustering.**

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL3-yellow.svg?style=for-the-badge)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macos%20%7C%20windows-lightgrey?style=for-the-badge)](https://github.com/Nurysso/tyr)
[![Version](https://img.shields.io/badge/version-0.3.1-blue.svg?style=for-the-badge)](Cargo.toml)

[Features](#features) · [Modes](#modes-of-operation) · [Installation](#installation) · [Usage](#usage) · [Configuration](#configuration) · [Documentation](#documentation)

---

<img src="https://raw.githubusercontent.com/Nurysso/Hermes/main/kondo/kondo.gif" alt="Tyr Demo" width="100%" style="border-radius: 10px; box-shadow: 0 4px 8px rgba(0,0,0,0.2);"/>

</div>

## What is Tyr?

**Tyr** is a modern, high-performance command-line and terminal UI file organizer built in Rust. It eliminates folder clutter instantly by intelligently organizing files using one of three dedicated engines:

1. **Category (Extension) Mode (`-c`)** — Organizes files into sensible folders (`Images/`, `Documents/`, `Code/`, `Videos/`, etc.) based on customizable extension mappings.
2. **Filename Similarity Mode (`-f`)** — Uses a hybrid of **Levenshtein edit-distance** and **Jaccard token overlap** to group related files (e.g., `contract_v1.pdf`, `contract_v2.docx`, `contract_final.pdf`, `contract_scan.jpg`) together, regardless of their extensions.
3. **Intelligent ML Mode (`-i`)** — Extracts TF-IDF content vectors from text files combined with character n-gram filename features, running **K-Means++ clustering** to group files by actual semantic topic and context.

Everything runs **100% locally and offline**. No telemetry, no cloud dependencies.

---

## Features

- ⚡ **Blazingly Fast Parallelism** — Powered by [Rayon](https://github.com/rayon-rs/rayon) for multi-threaded scanning, feature extraction, and clustering across all CPU cores.
- 🎨 **Beautiful Terminal UI** — Rich interactive TUI built with [Ratatui](https://github.com/ratatui/ratatui) and [Crossterm](https://github.com/crossterm-rs/crossterm), featuring live progress bars, category breakdown bar charts, and scrollable previews.
- 🤖 **Multiple Organization Engines** — Choose between deterministic extension grouping, fuzzy filename similarity, or content-aware TF-IDF K-Means clustering.
- 🛡️ **Safe by Default** — Built-in conflict resolution (auto-renames duplicates `_1`, `_2`, ... up to 999 variations without data loss) and non-destructive dry-run previews.
- 🚀 **Lazy Directory Management** — Creates destination directories on demand with in-memory caching to minimize filesystem syscalls.
- 🤖 **Headless / Automation Ready (`-nui`)** — Run headlessly in scripts, CI pipelines, or cron jobs without launching the TUI.
- ⚙️ **Fully Configurable** — Unified TOML configuration located at `~/.config/tyr/tyr.toml` (or `%APPDATA%\tyr\tyr.toml` on Windows).

---

## Modes of Operation

| Mode                    | Flag                | Description                                                     | Best For                                                       |
| :---------------------- | :------------------ | :-------------------------------------------------------------- | :------------------------------------------------------------- |
| **Categorize**          | `-c, --categorize`  | Sorts files into predefined folders based on extension mappings | Downloads folders, desktop cleanups, general sorting           |
| **Filename Similarity** | `-f, --filename`    | Groups files using Levenshtein distance + Jaccard tokenization  | Versioned documents, photo shoot batches, sequential files     |
| **Intelligent ML**      | `-i, --intelligent` | Clusters files via TF-IDF content analysis + K-Means++          | Code repositories, research papers, markdown notes, text dumps |
| **Headless**            | `-nui, --no-ui`     | Runs any mode without launching the interactive TUI             | Automation, scripts, background jobs                           |

---

## Installation

### Requirements

- **Rust** 1.70 or newer (Rust 2024 edition compatible, recommended 1.80+)
- **Cargo** package manager

### From Source

```bash
# Clone the repository
git clone https://github.com/Nurysso/tyr.git
cd tyr

# Build and install using the Makefile
make install
```

Or build manually with Cargo:

```bash
cargo install --path .
```

### Linux / macOS One-Liner

```bash
curl -fsSL https://raw.githubusercontent.com/Nurysso/tyr/main/install.sh | bash
```

### Windows (PowerShell)

```powershell
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/Nurysso/tyr/main/install.ps1" -OutFile "install.ps1"
.\install.ps1
```

---

## Usage

### Command Syntax

```bash
tyr [OPTIONS] [DIRECTORY]
```

If `[DIRECTORY]` is omitted, Tyr targets the current working directory.

### Common Commands

```bash
# 1. Organize by category (extension mode)
tyr -c ~/Downloads

# 2. Group similar filenames together
tyr -f ~/Documents/Projects

# 3. Intelligent ML-based topic clustering (content + filename)
tyr -i ~/Research/Notes

# 4. Headless execution (no TUI)
tyr -c -nui ~/Downloads
tyr -f -nui ~/Pictures/Exports
tyr -i -nui ~/Notes

# 5. Show help or version
tyr --help
tyr --version
```

---

## Interactive TUI Controls

### Categorize Mode (`tyr -c`)

| Key         | Action                                                     |
| :---------- | :--------------------------------------------------------- |
| `s`         | **Start** organizing files into category folders           |
| `d`         | **Dry run** (preview planned moves without touching files) |
| `q` / `Esc` | **Quit** the application                                   |

### Filename Similarity Mode (`tyr -f`)

| Key                   | Action                                                                    |
| :-------------------- | :------------------------------------------------------------------------ |
| `a`                   | **Analyze** directory and compute similarity groups                       |
| `s`                   | **Organize** and move files into discovered groups                        |
| `k`                   | **Toggle skip destination** (route unmatched single files to `tyr-skip/`) |
| `↑` / `↓`             | **Scroll** through discovered groups and file lists                       |
| `PageUp` / `PageDown` | **Fast scroll** by 10 items                                               |
| `q` / `Esc`           | **Quit** the application                                                  |

### Intelligent ML Mode (`tyr -i`)

| Key         | Action                                                |
| :---------- | :---------------------------------------------------- |
| `s`         | **Start** feature extraction and K-Means++ clustering |
| `m`         | **Move** files into suggested cluster directories     |
| `d`         | **Dry run** preview of operations                     |
| `q` / `Esc` | **Quit** the application                              |

---

## Example Walkthrough

### 1. Extension Categorization (`tyr -c`)

**Before:**

```
Downloads/
├── invoice_2026.pdf
├── project_mockup.png
├── presentation.pptx
├── main.rs
├── track01.flac
└── dataset.csv
```

**After `tyr -c ~/Downloads`:**

```
Downloads/
├── Documents/
│   └── invoice_2026.pdf
├── Images/
│   └── project_mockup.png
├── Presentations/
│   └── presentation.pptx
├── Code/
│   └── main.rs
├── Music/
│   └── track01.flac
└── Spreadsheets/
    └── dataset.csv
```

### 2. Filename Similarity (`tyr -f`)

**Before:**

```
Downloads/
├── report_v1.pdf
├── report_v2.pdf
├── report_final.docx
├── report_signed_scan.jpg
├── vacation_beach_01.jpg
├── vacation_beach_02.jpg
└── unrelated_note.txt
```

**After `tyr -f ~/Downloads`:**

```
Downloads/
├── Report/
│   ├── report_v1.pdf
│   ├── report_v2.pdf
│   ├── report_final.docx
│   └── report_signed_scan.jpg
├── VacationBeach/
│   ├── vacation_beach_01.jpg
│   └── vacation_beach_02.jpg
└── unrelated_note.txt           # Remains in place or moved to tyr-skip/ (if [k] toggled)
```

---

## Configuration

Tyr automatically creates a configuration file on its first run at:

- **Linux/macOS:** `~/.config/tyr/tyr.toml`
- **Windows:** `%APPDATA%\tyr\tyr.toml`

### Default `tyr.toml`

```toml
# Tyr File Organizer Configuration
batch_size = 100
log_file = "~/.config/tyr/tyr.log"

# Global skip options across all modes
skip_patterns = [".DS_Store", "Thumbs.db", ".git", ".gitignore", "desktop.ini", ".localized"]
skip_hidden_files = true

[similarity_config]
levenshtein_threshold = 0.7   # Character-level similarity threshold (0.0 - 1.0)
jaccard_threshold = 0.5       # Token-overlap threshold (0.0 - 1.0)
levenshtein_weight = 0.6      # Character edit-distance weight
jaccard_weight = 0.4          # Token overlap weight
min_similarity_score = 0.65   # Minimum composite score to form a group

[intelligent_config]
max_lines_to_read = 100          # First N lines of text files sampled for TF-IDF
min_cluster_size = 2             # Minimum items to form a cluster
max_clusters = 20                # Upper bound on K-Means clusters
filename_similarity_weight = 0.3 # Weight for filename n-gram features
content_similarity_weight = 0.7  # Weight for TF-IDF content vectors
similarity_threshold = 0.65      # Clustering threshold
max_iterations = 100             # Max iterations for K-Means convergence

# Category definitions (customize folders & extensions)
[categories.images]
extensions = ["jpg", "jpeg", "png", "gif", "bmp", "svg", "webp", "tiff", "ico", "heic", "raw", "cr2", "nef", "orf", "sr2"]
folder_name = "Images"
```

### Makefile Convenience Commands

```bash
make config-edit     # Open tyr.toml in $EDITOR
make config-path     # Display full configuration file path
make config-view     # Display configuration contents in terminal
make config-backup   # Create a timestamped backup of your config
make config-reset    # Reset config file to factory defaults
```

---

## Documentation

For in-depth guides, algorithms, and visual examples, check the `docs/` directory:

- [**docs/features.md**](docs/features.md) — Comprehensive technical breakdown of all features, architecture, and benchmarks.
- [**docs/filename-engine.md**](docs/filename-engine.md) — Mathematical explanation of the ML algorithms (Levenshtein, Jaccard, TF-IDF, K-Means++).
- [**docs/smart_grouping.md**](docs/smart_grouping.md) — Visual scenarios, threshold guides, and pattern detection examples.

---

## Roadmap

- [ ] **Content Hashing & Duplicate Deduplication** — Fast Blake3/SHA-256 hash checks to detect exact duplicate files.
- [ ] **Date-based Partitioning** — Automatic organization by EXIF photo creation dates or filesystem timestamps (`YYYY/MM/`).
- [ ] **Undo / Rollback History** — Transactional journal allowing one-key rollback of previous organization runs.
- [ ] **Daemon / Watch Mode** — Background daemon listening to filesystem events (inotify/fsevents) to organize downloads automatically.

---

## Contributing

Contributions are welcome! If you want to add a feature, report a bug, or improve documentation:

1. Fork the repository.
2. Create your feature branch (`git checkout -b feature/amazing-feature`).
3. Ensure formatting and clippy lints pass:
   ```bash
   make fmt
   make lint
   make test
   ```
4. Commit your changes and open a Pull Request.

---

## License

This project is licensed under the **GNU General Public License v3.0 or later** ([GPL-3.0-or-later](LICENSE)).

Copyright (C) 2026 Dawood Khan (Nurysso) <nurysso@proton.me>.
