---
name: sheet-cleaner
description: "Use when cleaning, deduplicating, or normalizing messy CSV or Excel spreadsheets, strictly creating a non-destructive _clean copy."
version: 1.0.0
author: LadeStack
license: MIT
platforms: [windows, linux]
tools_required: [read_file, write_file, execute_code]
---
# Sheet Cleaner

Normalizes tabular CSV and Excel files: trims whitespace, deduplicates rows, standardizes header formats, and validates datatypes. **Strict Safety Rule**: The original file on disk is NEVER overwritten; output is written strictly to `<filename>_clean.<ext>`.

## When to use (3+ situations) / When NOT to use (refuse-when ≥2)
- **Use when**: Cleaning dirty CSV or Excel files with duplicate rows, irregular whitespace, or inconsistent capitalization.
- **Use when**: Normalizing date formats (`YYYY-MM-DD`) or phone/currency columns for data pipelines.
- **Use when**: Preparing raw spreadsheet exports for database ingestion or analytics.
- **Do NOT use when**: The user requests an in-place overwrite of the original data file without backup.
- **Do NOT use when**: The file exceeds the 100MB data ceiling.

## Procedure (numbered, tool calls with params)
1. Ingest input sheet in read-only mode:
   - Call `read_file(path=sheet_path, limit=100)` to inspect headers, delimiter, and initial sample rows.
2. Formulate cleaning plan:
   - Identify trailing/leading whitespace in text columns.
   - Detect duplicate rows based on primary keys or complete row matches.
   - Identify missing values or malformed dates/numbers.
3. Execute cleaning transformation via Python script:
   - Read the input sheet with `openpyxl` (read_only=True) or `csv.reader`.
   - Apply clean transforms: strip strings, cast numbers, parse ISO dates.
   - Write sanitized dataset to `<basename>_clean.<extension>`.
4. Output change audit:
   - Total rows processed, duplicates removed, and columns standardized.
   - Confirmation of newly created `_clean` file path with original untouched.

## Worked example (input → tool sequence → output)
- **Input**: "Clean up customer_leads.csv and remove any duplicates."
- **Tool sequence**:
  1. `read_file(path="customer_leads.csv", limit=50)` -> headers: `Name , Email,Phone , Joined Date `
  2. Run clean transform script producing `customer_leads_clean.csv`.
- **Output**:
  "Sheet cleaning complete:
  - Input: `customer_leads.csv` (1,240 rows, untouched)
  - Output: `customer_leads_clean.csv` (1,180 rows, 60 duplicates removed)
  - Headers trimmed and normalized to lowercase snake_case.
  - Dates standardized to `YYYY-MM-DD`."

## Refuse when
- The operator requests direct in-place mutation of the original spreadsheet file without creating a distinct clean copy.
- The target spreadsheet exceeds 100MB.
