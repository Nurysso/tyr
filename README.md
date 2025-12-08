<div align="center">

# Tyr

**Intelligent file organization powered by machine learning**

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macos%20%7C%20windows-lightgrey?style=for-the-badge)](https://github.com)
[![Made with Rust](https://img.shields.io/badge/Made%20with-Rust-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)

[Features](#features) • [Installation](#installation) • [Usage](#usage) • [Configuration](#configuration) • [Documentation](#documentation)

---

<img src="https://raw.githubusercontent.com/Nurysso/Hermes/main/kondo/kondo.gif" alt="Demo" width="100%" style="border-radius: 10px; box-shadow: 0 4px 8px rgba(0,0,0,0.2);"/>
<!-- ![Demo](assets/demo.gif) -->

</div>

## Overview

<table>
<tr>
<td width="50%">

Tyr is a **blazingly fast** file organizer built in Rust that automatically categorizes and groups your files. Unlike traditional organizers that only look at file extensions, Tyr uses **machine learning algorithms** to understand relationships between files and intelligently organize them.

</td>
<td width="50%">

```rust
// Organize in seconds
tyr ~/Downloads

// Features
✓ ML-powered grouping
✓ Parallel processing
✓ Beautiful TUI
✓ Safe operations
```

</td>
</tr>
</table>

---

## Features

<table>
<tr>
<td width="50%" valign="top">

### Intelligent Organization
```
┌─────────────────────────────────┐
│  Extension-based categorization │
│  10+ default categories         │
│  Unlimited custom categories    │
└─────────────────────────────────┘
        ↓
┌─────────────────────────────────┐
│  ML-powered smart grouping      │
│  Levenshtein + Jaccard          │
│  Automatic version detection    │
└─────────────────────────────────┘
```

**Capabilities:**
- Groups file versions (`file_v1`, `file_v2`)
- Detects related files (`project_code`, `project_docs`)
- Customizable similarity thresholds

</td>
<td width="50%" valign="top">

### High Performance
```
┌──────────────────────────────────┐
│  Performance Metrics             │
├──────────────────────────────────┤
│  1000 files     →  2.1 seconds   │
│  Parallel       →  3-5x speedup  │
│  Memory usage   →  ~8-12 MB      │
│  Syscalls       →  reduced alot  │
└──────────────────────────────────┘
```

**Speed:**
- 600x faster than manual sorting
- Optimized with Rayon parallelism
- Lazy directory creation
- Memory-efficient batch operations

</td>
</tr>
</table>

---

<div align="center">

## Installation

</div>

<table>
<tr>
<td width="33%" align="center">

### Linux/Mac
```bash
curl -fsSL https://raw.githubusercontent.com/aelune/tyr/main/install.sh | bash
```

</td>
<td width="33%" align="center">

### Windows
```bash
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/aelune/tyr/main/install.ps1" -OutFile "install.ps1"
.\install.ps1
```

</td>
<td width="33%" align="center">

### Manual
```bash
git clone https://github.com/Aelune/tyr.git
cd tyr
make install
```

</td>
</tr>
</table>

<div align="center">

**Requirements:** Rust 1.70+

</div>

---

## Usage

### Quick Start

<table>
<tr>
<td width="50%">

```bash
# Organize current directory
tyr

# Organize specific directory
tyr ~/Downloads

# Organize any path
tyr /path/to/folder
```

</td>
<td width="50%">

**Terminal Controls**

| Key | Action |
|:---:|--------|
| `s` | Start organizing |
| `d` | Dry run (preview) |
| `q` | Quit application |

</td>
</tr>
</table>

### Visual Example

<div align="center">

```
┌────────────────────────────────────────────────────────────────┐
│                         BEFORE                                 │
├────────────────────────────────────────────────────────────────┤
│  Downloads/                                                    │
│  ├── report_v1.pdf                                             │
│  ├── report_v2.pdf                                             │
│  ├── report_v3.pdf                                             │
│  ├── vacation.jpg                                              │
│  ├── song.mp3                                                  │
│  └── data.xlsx                                                 │
└────────────────────────────────────────────────────────────────┘
                               ↓
                    tyr ~/Downloads
                               ↓
┌────────────────────────────────────────────────────────────────┐
│                    AFTER (Category Mode)                       │
├────────────────────────────────────────────────────────────────┤
│  Downloads/                                                    │
│  ├── Documents/                                                │
│  │   ├── report_v1.pdf                                         │
│  │   ├── report_v2.pdf                                         │
│  │   └── report_v3.pdf                                         │
│  ├── Images/                                                   │
│  │   └── vacation.jpg                                          │
│  ├── Audio/                                                    │
│  │   └── song.mp3                                              │
│  └── Spreadsheets/                                             │
│      └── data.xlsx                                             │
└────────────────────────────────────────────────────────────────┘
                               ↓
              enable_smart_grouping = true
                               ↓
┌────────────────────────────────────────────────────────────────┐
│                 AFTER (Smart Grouping Mode aka filename)       │
├────────────────────────────────────────────────────────────────┤
│  Downloads/                                                    │
│  ├── Documents/                                                │
│  │   └── report_group_001/    ← Grouped by ML                  │
│  │       ├── report_v1.pdf                                     │
│  │       ├── report_v2.pdf                                     │
│  │       └── report_v3.pdf                                     │
│  ├── Images/                                                   │
│  │   └── vacation.jpg                                          │
│  ├── Audio/                                                    │
│  │   └── song.mp3                                              │
│  └── Spreadsheets/                                             │
│      └── data.xlsx                                             │
└────────────────────────────────────────────────────────────────┘
```

</div>

---

## Configuration

<div align="center">

**Configuration File:** `~/.config/tyr/tyr.toml`

</div>

<table>
<tr>
<td width="50%" valign="top">

### Basic Settings

```toml
# Batch processing size
batch_size = 100

# Enable ML grouping
enable_smart_grouping = false

# Skip these files
skip_patterns = [
    ".DS_Store",
    "Thumbs.db",
    ".git*"
]
```

</td>
<td width="50%" valign="top">

### Custom Categories

```toml
[categories.my_category]
extensions = [
    "ext1",
    "ext2",
    "ext3"
]
folder_name = "My Custom Folder"
```

</td>
</tr>
<tr>
<td colspan="2">

### Similarity Tuning

```toml
[similarity_config]
levenshtein_threshold = 0.7      # Character similarity (0.0 - 1.0)
jaccard_threshold = 0.5          # Token overlap (0.0 - 1.0)
levenshtein_weight = 0.6         # Weight for character matching
jaccard_weight = 0.4             # Weight for token matching
min_similarity_score = 0.65      # Overall threshold for grouping
```

</td>
</tr>
</table>

<!-- ### Configuration Commands

<div align="center">

```bash
make config-edit     │  make config-path     │  make config-backup     │  make config-reset
```

</div>

--- -->
<!--
## Performance

<div align="center">

### Benchmarks (1000 files)

<table>
<tr>
<th>Mode</th>
<th>Time</th>
<th>Memory</th>
<th>Speedup</th>
</tr>
<tr>
<td align="center">Standard</td>
<td align="center"><code>2.1s</code></td>
<td align="center"><code>~8MB</code></td>
<td align="center"><strong>600x</strong></td>
</tr>
<tr>
<td align="center">Smart Grouping</td>
<td align="center"><code>2.8s</code></td>
<td align="center"><code>~12MB</code></td>
<td align="center"><strong>450x</strong></td>
</tr>
<tr>
<td align="center">Manual</td>
<td align="center"><code>~30min</code></td>
<td align="center"><code>-</code></td>
<td align="center"><code>1x</code></td>
</tr>
</table>

### Performance Highlights

```
┌──────────────────────────────────────────────────────────────┐
│  Parallel Processing     →  3-5x speedup on multi-core       │
│  Lazy Operations         →  99% reduction in syscalls        │
│  Algorithm Efficiency    →  ~1ms per file comparison         │
│  Memory Footprint        →  Minimal (~8-12MB)                │
└──────────────────────────────────────────────────────────────┘
```

</div>

--- -->

## Use Cases

<table>
<tr>
<td width="50%" valign="top">

### Photography
```
┌───────────────────────────────┐
│  RAW + JPG pairs grouping     │
│  Shoot session organization   │
│  Sequential numbering         │
│  Multi-format support         │
└───────────────────────────────┘
```

### Software Development
```
┌───────────────────────────────┐
│  Source file organization     │
│  Language-agnostic sorting    │
│  Config file detection        │
│  Project-based grouping       │
└───────────────────────────────┘
```

</td>
<td width="50%" valign="top">

### Document Management
```
┌───────────────────────────────┐
│  Version tracking             │
│  Project-based grouping       │
│  Multi-format support         │
│  Archive organization         │
└───────────────────────────────┘
```

### Downloads Cleanup
```
┌───────────────────────────────┐
│  Automatic categorization     │
│  Weekly organization runs     │
│  Clutter-free workspace       │
│  Smart conflict resolution    │
└───────────────────────────────┘
```

</td>
</tr>
</table>

---

## Make Commands

<div align="center">

<table>
<tr>
<td align="center" width="25%">

**Build & Install**
```bash
make install
make uninstall
make clean
```

</td>
<td align="center" width="25%">

**Run & Test**
```bash
make run
make test
```

</td>
<td align="center" width="25%">

**Configuration**
```bash
make config-edit
make config-path
```

</td>
<td align="center" width="25%">

**Backup & Reset**
```bash
make config-backup
make config-reset
```

</td>
</tr>
</table>

</div>

---

<!-- ## Documentation

<div align="center">

<table>
<tr>
<td align="center" width="25%">

**[COMPLETE_SETUP.md](COMPLETE_SETUP.md)**

Installation & Setup

</td>
<td align="center" width="25%">

**[QUICK_REFERENCE.md](QUICK_REFERENCE.md)**

Command Reference

</td>
<td align="center" width="25%">

**[ML_FEATURES.md](ML_FEATURES.md)**

Smart Grouping

</td>
<td align="center" width="25%">

**[SMART_GROUPING_EXAMPLES.md](SMART_GROUPING_EXAMPLES.md)**

Usage Examples

</td>
</tr>
</table>

</div>

--- -->

## Built With

<div align="center">

<table>
<tr>
<td align="center" width="20%">

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)

**Rust**

Systems Language

</td>
<td align="center" width="20%">

![Ratatui](https://img.shields.io/badge/Ratatui-000000?style=for-the-badge)

**Ratatui**

Terminal UI

</td>
<td align="center" width="20%">

![Crossterm](https://img.shields.io/badge/Crossterm-4A5568?style=for-the-badge)

**Crossterm**

Terminal Control

</td>
<td align="center" width="20%">

![Rayon](https://img.shields.io/badge/Rayon-EE4C2C?style=for-the-badge)

**Rayon**

Parallelism

</td>
<td align="center" width="20%">

![Serde](https://img.shields.io/badge/Serde-000000?style=for-the-badge)

**Serde**

Serialization

</td>
</tr>
</table>

</div>

---

## Roadmap

<table>
<tr>
<td width="50%" valign="top">

### Planned Features

- [ ] Content-based similarity (file hashing)
- [ ] Date-based organization
- [ ] Duplicate file detection
- [ ] Undo functionality
- [ ] Watch mode (auto-organize on changes)

</td>
<td width="50%" valign="top">

### What it dosen't do

- [ ] Cloud storage integration
- [ ] Image content analysis
- [ ] Audio fingerprinting

</td>
</tr>
</table>

---

## Contributing

<div align="center">

Contributions are welcome! We especially appreciate help with:

```
┌──────────────────────┬──────────────────────┬──────────────────────┐
│  New Algorithms      │  Performance Tuning  │  File Type Support   │
├──────────────────────┼──────────────────────┼──────────────────────┤
│  Documentation       │  Bug Fixes           │  Testing             │
└──────────────────────┴──────────────────────┴──────────────────────┘
```

</div>

---

## Comparison

<div align="center">

<table>
<tr>
<th>Feature</th>
<th>Tyr</th>
<th>Traditional Organizers</th>
</tr>
<tr>
<td>ML-based grouping</td>
<td align="center">✅</td>
<td align="center">❌</td>
</tr>
<tr>
<td>Parallel processing</td>
<td align="center">✅</td>
<td align="center">❌</td>
</tr>
<tr>
<td>Interactive TUI</td>
<td align="center">✅</td>
<td align="center">⚠️ Some</td>
</tr>
<tr>
<td>External configuration</td>
<td align="center">✅</td>
<td align="center">⚠️ Some</td>
</tr>
<tr>
<td>Dry run mode</td>
<td align="center">✅</td>
<td align="center">⚠️ Some</td>
</tr>
<tr>
<td>Cross-platform</td>
<td align="center">✅</td>
<td align="center">⚠️ Varies</td>
</tr>
</table>

</div>

---

## License

<div align="center">

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

---

**Made with Rust**

[![Star on GitHub](https://img.shields.io/github/stars/Aelune/tyr?style=social)](https://github.com/Aelune/tyr)
[![Fork on GitHub](https://img.shields.io/github/forks/Aelune/tyr?style=social)](https://github.com/Aelune/tyr/fork)

[⬆ back to top](#tyr)

</div>
