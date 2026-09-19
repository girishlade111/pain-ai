---
name: file-organize
description: "Use when organizing a cluttered folder by file extension or date, or preparing a safe dry-run plan before moving files."
version: 1.0.0
author: LadeStack
license: MIT
platforms: [windows, linux]
tools_required: [read_file, search_files, terminal]
---
# File Organize

Sorts cluttered directories into structured categories (Documents, Images, Archives, Code) or year-month folders, always generating a non-destructive dry-run preview before moving any files.

## When to use (3+ situations) / When NOT to use (refuse-when ≥2)
- **Use when**: The user requests "organize my Downloads folder" or "clean up this folder by file type".
- **Use when**: Grouping project assets, screenshots, or logs into subfolders by creation timestamp.
- **Use when**: Preparing an audit of file distributions within a cluttered workspace.
- **Do NOT use when**: The target directory is a critical system path (e.g. `C:\Windows`, `/usr`, `/etc`, or git internals `.git`).
- **Do NOT use when**: The user explicitly asks to permanently delete files rather than categorize them.

## Procedure (numbered, tool calls with params)
1. Scan target directory contents:
   - Call `search_files(query="*", dir=target_path)` or execute non-destructive listing to catalog all files, extensions, and file sizes.
2. Group files into target organizational buckets:
   - Documents: `.pdf`, `.docx`, `.txt`, `.md`, `.xlsx`, `.pptx`
   - Images: `.png`, `.jpg`, `.jpeg`, `.gif`, `.svg`, `.webp`
   - Archives: `.zip`, `.tar`, `.gz`, `.7z`, `.rar`
   - Media: `.mp4`, `.mov`, `.mp3`, `.wav`
3. Generate and present a Dry-Run Plan:
   - Output a clear table of proposed source -> destination paths to the operator.
   - Do NOT move any files until the operator confirms the dry-run plan.
4. Execute moves gate-first:
   - For confirmed relocations, invoke file moves individually or via script, verifying that no existing files are overwritten.

## Worked example (input → tool sequence → output)
- **Input**: "Organize the loose files in ./downloads"
- **Tool sequence**:
  1. `search_files(query="*.*", dir="./downloads")` -> `["invoice.pdf", "photo.png", "archive.zip"]`
  2. Synthesize plan:
     - `invoice.pdf` -> `downloads/Documents/invoice.pdf`
     - `photo.png` -> `downloads/Images/photo.png`
     - `archive.zip` -> `downloads/Archives/archive.zip`
- **Output**:
  "I surveyed 3 loose files in `./downloads`. Here is the proposed reorganization:
  - Move `invoice.pdf` to `./downloads/Documents/`
  - Move `photo.png` to `./downloads/Images/`
  - Move `archive.zip` to `./downloads/Archives/`
  Please approve to execute these moves."

## Refuse when
- The target folder is root (`/` or `C:\`) or home directory root without subfolder specification.
- The proposed reorganization would overwrite existing files of the same name without explicit versioning.
