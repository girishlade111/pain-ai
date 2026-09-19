---
name: backup-folder
description: "Use when creating timestamped, compressed backup archives of a directory with SHA-256 integrity verification."
version: 1.0.0
author: LadeStack
license: MIT
platforms: [windows, linux]
tools_required: [terminal, search_files, read_file]
---
# Backup Folder

Creates compressed `.tar.gz` or `.zip` backup archives for a designated folder with automated ISO-8601 timestamps, exclusion of temporary files (`node_modules`, `.git`, `target`), and SHA-256 integrity checksum generation.

## When to use (3+ situations) / When NOT to use (refuse-when ≥2)
- **Use when**: Taking a pre-refactor snapshot of a codebase or document directory.
- **Use when**: Creating scheduled recurring folder backups via cron.
- **Use when**: Preparing an offline export of a project workspace with an audit manifest.
- **Do NOT use when**: Backing up the entire root filesystem (`/` or `C:\`).
- **Do NOT use when**: The target directory contains active live database lockfiles that require database-level dumps.

## Procedure (numbered, tool calls with params)
1. Survey directory size and exclude patterns:
   - Call `search_files(dir=source_path, query="*")` to calculate total files and filter out noise (`.git`, `node_modules`, `target`, `dist`, `__pycache__`).
2. Generate target archive path:
   - Construct timestamped archive filename: `<folder_name>_backup_YYYYMMDD_HHMMSS.zip` (Windows) or `.tar.gz` (Linux).
3. Request host permission gate approval:
   - Present archive destination and calculated size to operator via `ApprovalCard`.
4. Create archive:
   - Execute compression command packing source directory excluding ignored folders.
5. Compute SHA-256 verification hash:
   - Compute hash and write alongside archive: `<archive_name>.sha256`.
6. Return backup summary:
   - Output archive location, size in MB, and SHA-256 checksum string.

## Worked example (input → tool sequence → output)
- **Input**: "Backup my ./src directory before I refactor."
- **Tool sequence**:
  1. Calculate files in `./src` (48 files, 1.2 MB)
  2. Prepare archive name: `src_backup_20260919_061500.zip`
  3. Execute compression with gate approval.
  4. Generate SHA-256 hash.
- **Output**:
  "Backup created successfully:
  - Archive: `./backups/src_backup_20260919_061500.zip` (420 KB)
  - Files archived: 48
  - SHA-256: `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
  - Integrity verified."

## Refuse when
- The target folder is root or parent directory without path boundary limits.
- Insufficient disk space on the backup target drive.
