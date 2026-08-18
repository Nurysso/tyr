# Smart Grouping - Visual Scenarios & Case Studies

This guide showcases real-world before/after scenarios illustrating how **Tyr** organizes messy folders using its similarity and ML engines.

---

## 📸 Scenario 1: Photography & Content Creation

### Problem

Camera shoots and edits create pairs of RAW and JPEG files along with project metadata, which standard extension organizers split into different directories.

```
Photos/
├── DSCF0001.RAF
├── DSCF0001.jpg
├── DSCF0002.RAF
├── DSCF0002.jpg
├── beach_sunset_raw.dng
├── beach_sunset_edit.jpg
└── photo_notes.txt
```

### Result with `tyr -f Photos` (Filename Similarity)

```
Photos/
├── CameraPhotos/
│   ├── DSCF0001.RAF
│   ├── DSCF0001.jpg
│   ├── DSCF0002.RAF
│   └── DSCF0002.jpg
├── BeachSunset/
│   ├── beach_sunset_raw.dng
│   └── beach_sunset_edit.jpg
└── photo_notes.txt            # Remains in place or routed to tyr-skip/
```

**✓ Outcome:** RAW + JPG pairs and edit batches stay together automatically.

---

## 📄 Scenario 2: Legal Contracts & Versioned Documents

### Problem

Document iterations across PDF, Word, and scanned images end up scattered.

```
Contracts/
├── client_agreement_draft_v1.docx
├── client_agreement_draft_v2.docx
├── client_agreement_final.pdf
├── client_agreement_signed_scan.jpg
├── vendor_contract_v1.pdf
├── vendor_contract_v2.pdf
└── q1_invoice.xlsx
```

### Result with `tyr -f Contracts`

```
Contracts/
├── ClientAgreement/
│   ├── client_agreement_draft_v1.docx
│   ├── client_agreement_draft_v2.docx
│   ├── client_agreement_final.pdf
│   └── client_agreement_signed_scan.jpg
├── VendorContract/
│   ├── vendor_contract_v1.pdf
│   └── vendor_contract_v2.pdf
└── q1_invoice.xlsx
```

**✓ Outcome:** All drafts, finals, and scans for each contract are consolidated into unified folders.

---

## 🎵 Scenario 3: Music & Podcast Production

### Problem

Audio stems, track mixes, lyrics, and artwork for a single song end up in separate `Music/`, `Documents/`, and `Images/` folders if sorted only by extension.

```
Studio/
├── song_mix_v1.mp3
├── song_mix_v2.mp3
├── song_master.wav
├── song_lyrics.txt
├── song_cover_art.png
├── podcast_ep12_raw.wav
├── podcast_ep12_final.mp3
└── podcast_ep12_notes.md
```

### Result with `tyr -f Studio`

```
Studio/
├── Song/
│   ├── song_mix_v1.mp3
│   ├── song_mix_v2.mp3
│   ├── song_master.wav
│   ├── song_lyrics.txt
│   └── song_cover_art.png
└── PodcastEp12/
    ├── podcast_ep12_raw.wav
    ├── podcast_ep12_final.mp3
    └── podcast_ep12_notes.md
```

**✓ Outcome:** Complete project assets stay in one folder.

---

## 💻 Scenario 4: Research Notes & Code Dumps

### Problem

A flat folder of mixed notes, scripts, and documentation on different subjects (Machine Learning vs Web Development vs DevOps).

```
Workspace/
├── neural_net.py
├── transformer_notes.md
├── train_model.py
├── docker_setup.sh
├── k8s_deploy.yaml
├── nginx_config.conf
├── react_app.tsx
├── styles.css
└── frontend_readme.md
```

### Result with `tyr -i Workspace` (Intelligent ML Mode)

```
Workspace/
├── Model_Training/
│   ├── neural_net.py
│   ├── transformer_notes.md
│   └── train_model.py
├── Deployment_Config/
│   ├── docker_setup.sh
│   ├── k8s_deploy.yaml
│   └── nginx_config.conf
└── Frontend_App/
    ├── react_app.tsx
    ├── styles.css
    └── frontend_readme.md
```

**✓ Outcome:** TF-IDF content vectors cluster files by topic and semantic context.

---

## 🔬 Similarity Scoring Matrix

The table below illustrates how the combined Levenshtein + Jaccard score evaluates sample filename pairs under default settings ($0.6 \text{ Lev} + 0.4 \text{ Jac}$):

| File 1               | File 2                   | Levenshtein | Jaccard | Combined Score | Grouped? ($\ge 0.65$) |
| :------------------- | :----------------------- | :---------: | :-----: | :------------: | :-------------------: |
| `report_v1.pdf`      | `report_v2.pdf`          |    0.92     |  0.50   |    **0.75**    |        ✅ Yes         |
| `DSC_0001.jpg`       | `DSC_0002.jpg`           |    0.91     |  1.00   |    **0.95**    |        ✅ Yes         |
| `project_code.py`    | `project_test.py`        |    0.80     |  0.67   |    **0.75**    |        ✅ Yes         |
| `vacation_beach.jpg` | `vacation_mountains.jpg` |    0.58     |  0.50   |    **0.55**    |  ⚠️ Config Dependent  |
| `invoice_2026.pdf`   | `vacation.jpg`           |    0.19     |  0.00   |    **0.11**    |         ❌ No         |
| `notes.txt`          | `something.doc`          |    0.15     |  0.00   |    **0.09**    |         ❌ No         |

---

## 🎯 Threshold Tuning Guide

| Profile                | `min_similarity_score` | Weights (Lev / Jac) | Behavior                                                              |
| :--------------------- | :--------------------: | :-----------------: | :-------------------------------------------------------------------- |
| **Conservative**       |         `0.80`         |    `0.70 / 0.30`    | Only near-exact matches and direct version increments are grouped.    |
| **Balanced (Default)** |         `0.65`         |    `0.60 / 0.40`    | Groups version series, sequential numbers, and strong phrase matches. |
| **Aggressive**         |         `0.50`         |    `0.30 / 0.70`    | Groups broader thematic files sharing one or more common keywords.    |

---

## 🛠️ Testing Your Custom Configuration

You can easily test your configuration without altering files:

1. Create a test directory with sample files:

   ```bash
   cd /path/to/tyr/repo # where the source code exists
   mkdir -p /tmp/tyr-test
   python test/DummyFilesCreation.py -o /tmp/tyr-test
   tyr -c /tmp/tyr-test # you can change mode as well
   ```

2. Run Tyr in dry-run mode:
   - For **Extension Mode**: `tyr -c .` $\to$ Press `d` for dry run.
   - For **Filename Similarity**: `tyr -f .` $\to$ Press `a` to analyze and inspect the groups.
   - For **Intelligent ML**: `tyr -i .` $\to$ Press `s` to analyze and `d` for dry-run preview.

3. Clean up when finished:
   ```bash
   # Also check vanish https://github.com/Nurysso/vanish a modern alternative to rm with Recycle bin folder
   vx /tmp/tyr-test
   ```
   > [!IMPORTANT]
   > This file was generated via antiGravity
