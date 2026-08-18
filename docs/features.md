# Tyr File Organizer - Complete Feature Guide

Tyr is a modern, blazingly fast file organizer built in Rust with interactive Terminal UI (TUI) and headless CLI support. This document provides an exhaustive reference of all architectural capabilities, algorithms, performance characteristics, and configuration options available in **Tyr v0.3.1**.

---

## 🚀 Core Organization Modes

Tyr provides three distinct operational engines tailored for different organization workflows, plus a headless mode for automation.

### 1. Category (Extension-Based) Organization (`-c, --categorize`)

- **Deterministic Extension Mapping**: Maps file extensions to configured category folders.
- **14 Built-in Categories**:
  - `Images` (`jpg`, `jpeg`, `png`, `gif`, `bmp`, `svg`, `webp`, `tiff`, `ico`, `heic`, `raw`, `cr2`, `nef`, `orf`, `sr2`)
  - `Videos` (`mp4`, `avi`, `mkv`, `mov`, `wmv`, `flv`, `webm`, `m4v`, `3gp`, `mpg`, `mpeg`, `vob`)
  - `Music` (`mp3`, `wav`, `flac`, `aac`, `ogg`, `wma`, `m4a`, `opus`, `aiff`, `ape`, `alac`)
  - `Documents` (`pdf`, `doc`, `docx`, `txt`, `rtf`, `odt`, `pages`, `tex`, `md`, `epub`, `mobi`)
  - `Spreadsheets` (`xls`, `xlsx`, `csv`, `ods`, `numbers`)
  - `Presentations` (`ppt`, `pptx`, `odp`, `key`)
  - `Archives` (`zip`, `rar`, `7z`, `tar`, `gz`, `bz2`, `xz`, `dmg`, `pkg`, `deb`, `rpm`, `iso`)
  - `Code` (`rs`, `py`, `js`, `ts`, `jsx`, `tsx`, `html`, `css`, `scss`, `sass`, `cpp`, `c`, `h`, `hpp`, `java`, `go`, `php`, `rb`, `swift`, `kt`, `dart`, `scala`, `sh`, `bat`, `ps1`, `r`, `lua`, `vim`)
  - `Data` (`json`, `xml`, `yaml`, `yml`, `toml`, `ini`, `cfg`, `conf`, `sql`, `db`, `sqlite`, `mdb`)
  - `Applications` (`exe`, `msi`, `app`, `deb`, `rpm`, `dmg`, `pkg`, `appimage`, `run`)
  - `Fonts` (`ttf`, `otf`, `woff`, `woff2`, `eot`)
  - `Ebooks` (`epub`, `mobi`, `azw`, `azw3`, `cbr`, `cbz`)
  - `3D Models` (`obj`, `fbx`, `stl`, `blend`, `dae`, `3ds`, `max`, `gltf`, `glb`)
  - `Design Files` (`psd`, `ai`, `xd`, `sketch`, `fig`, `indd`, `cdr`)
- **Uncategorized Fallback**: Files with unknown extensions are safely organized into `Extras/`.
- **Parallel Classification**: Files are inspected and matched in parallel across worker threads.

### 2. Filename Similarity Organization (`-f, --filename`)

- **Levenshtein Distance**: Measures character-level edit distance. Ideal for version increments (`doc_v1`, `doc_v2`), dates, and sequential numbers (`DSC_001`, `DSC_002`).
- **Jaccard Token Similarity**: Splits filenames into words, bigrams, and phrase hints (`whatsapp image`, `screenshot`, `chatgpt`, etc.) to measure semantic overlap across different formats.
- **Composite Scoring**:
  $$\text{Score} = (W_{\text{lev}} \times \text{Levenshtein}) + (W_{\text{jac}} \times \text{Jaccard})$$
  Default: $0.6 \times \text{Levenshtein} + 0.4 \times \text{Jaccard}$ with a threshold of $\ge 0.65$.
- **Smart Folder Naming**: Automatically extracts common prefixes and detects known patterns (`Screenshots`, `WhatsAppImages`, `CameraPhotos`, `Invoices`, `Reports`, etc.) to name output folders cleanly.
- **Skip Routing (`k` key)**: Choose whether unmatched single files remain in place or get consolidated into `tyr-skip/`.

### 3. Intelligent ML Content Clustering (`-i, --intelligent`)

- **Text File Sampling**: Efficiently reads the first $N$ lines (default 100) of plain-text, markdown, code, and config files without reading entire files into RAM.
- **TF-IDF Vectorization**: Builds an in-memory document-term matrix with dynamic vocabulary extraction and inverse document frequency weighting.
- **Character N-Gram Filename Features**: Analyzes letter frequencies, string length, digits ratio, and special character ratios.
- **K-Means++ Clustering**: Automatically chooses optimal `k (2 <= k <= max_clusters)` using K-Means++ seeding with a fast, deterministic Xorshift64 PRNG.
- **Automated Cluster Naming**: Determines concise folder names based on common prefixes, highest-frequency meaningful keywords, or fallback dominant extensions.

### 4. Headless Automation Mode (`-nui, --no-ui`)

- Bypasses the interactive TUI completely.
- Formats structured terminal summaries with files organized, skipped, failed, and category counts.
- Perfect for shell scripts, systemd timers, and automated directory cleaners.

---

## ⚡ Performance & Safety Architecture

### 1. Lazy Directory Management (`LazyDirManager`)

- **Syscall Reduction**: Before creating any directory on disk, Tyr queries an in-memory cache protected by a thread-safe mutex.
- **Once-Per-Session Creation**: Prevents duplicate `mkdir`/`create_dir_all` calls for high-density folders, cutting filesystem syscalls by up to 99%.

### 2. Rayon Parallelism

- Directory traversal, filename tokenization, extension lookups, TF-IDF vector calculations, and K-Means distance matrices run concurrently using Rayon threadpools.
- Scales linearly with available CPU cores.

### 3. Safe Conflict Resolution

- **Zero Data Overwriting**: Never overwrites existing target files.
- **Automated Collision Renaming**: If `document.pdf` exists in the target destination, Tyr searches for `document_1.pdf`, `document_2.pdf`, up to `document_999.pdf`.
- **Atomic Operations**: Moves are executed using atomic OS renames with copy+delete fallback for cross-filesystem / mount boundaries.

### 4. Safe Logging & Session Logs

- Thread-safe in-memory log buffer (`SafeLogger`) captures every action and warning.
- Structured session logging written directly to `~/.config/tyr/tyr.log`.

---

## 🎮 Interactive Terminal UI (TUI)

Built using **Ratatui** and **Crossterm**, featuring:

- **Responsive 3-Column Layout**: Left panel (target details & statistics), Center panel (preview / results / live gauges), Right panel (sample files / logs / options).
- **Live Progress Gauges**: Animated spinners and determinate progress meters during heavy batch runs.
- **ASCII Bar Charts**: Category and cluster size distribution visual charts.
- **Scrollable Review Lists**: Inspect similarity groups and skipped files with keyboard navigation.

### Keybinding Reference

| Context                | Key             | Action                                            |
| :--------------------- | :-------------- | :------------------------------------------------ |
| **Global**             | `q` / `Esc`     | Quit the active screen / application              |
| **Categorize (`-c`)**  | `s`             | Start file organization                           |
|                        | `d`             | Run dry-run preview                               |
| **Filename (`-f`)**    | `a`             | Begin similarity analysis                         |
|                        | `s`             | Organize discovered groups into folders           |
|                        | `k`             | Toggle moving skipped single files to `tyr-skip/` |
|                        | `↑` / `↓`       | Scroll through preview groups                     |
|                        | `PgUp` / `PgDn` | Fast scroll by 10 items                           |
| **Intelligent (`-i`)** | `s`             | Start TF-IDF feature extraction & clustering      |
|                        | `m`             | Move files into suggested clusters                |
|                        | `d`             | Perform dry-run preview of cluster movements      |

---

## ⚙️ Configuration Reference (`tyr.toml`)

Located at `~/.config/tyr/tyr.toml` (Linux/macOS) or `%APPDATA%\tyr\tyr.toml` (Windows).

```toml
# General settings
batch_size = 100
log_file = "~/.config/tyr/tyr.log"
skip_patterns = [".DS_Store", "Thumbs.db", ".git", ".gitignore", "desktop.ini", ".localized"]
skip_hidden_files = true

# Settings for Filename Similarity Mode (-f)
[similarity_config]
levenshtein_threshold = 0.7
jaccard_threshold = 0.5
levenshtein_weight = 0.6
jaccard_weight = 0.4
min_similarity_score = 0.65

# Settings for Intelligent ML Mode (-i)
[intelligent_config]
max_lines_to_read = 100
min_cluster_size = 2
max_clusters = 20
filename_similarity_weight = 0.3
content_similarity_weight = 0.7
similarity_threshold = 0.65
max_iterations = 100

# Custom extension categories (-c)
[categories.images]
extensions = ["jpg", "jpeg", "png", "webp", "svg", "gif"]
folder_name = "Images"
```

---

## Extra Info

| Metric                 | Categorize (`-c`) | Filename Similarity (`-f`) | Intelligent ML (`-i`) |
| :--------------------- | :---------------- | :------------------------- | :-------------------- |
| **Algorithm**          | Hash Map Lookup   | Levenshtein + Jaccard      | TF-IDF + K-Means++    |
| **Memory Footprint**   | ~6 MB             | ~10 MB                     | ~18 MB                |
| **Dry Run Supported**  | Yes (`d`)         | NO (Review state)          | Yes (Review state)    |
| **Headless Supported** | Yes (`-c -nui`)   | Yes (`-f -nui`)            | Yes (`-i -nui`)       |

---

Last updated on August 18 2026

> [!IMPORTANT]
> This file was generated via antiGravity
