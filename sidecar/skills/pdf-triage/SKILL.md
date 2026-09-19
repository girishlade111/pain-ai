---
name: pdf-triage
description: "Use when analyzing PDF documents, extracting tabular data, or generating concise executive summaries from research papers and reports."
version: 1.0.0
author: LadeStack
license: MIT
platforms: [windows, linux]
tools_required: [read_file, vision_analyze]
---
# PDF Triage

Extracts structured text, outlines, key findings, and data tables from PDF documents within strict memory and token budgets.

## When to use (3+ situations) / When NOT to use (refuse-when ≥2)
- **Use when**: The user asks "summarize this PDF", "extract key points from this report", or "what does this contract say?".
- **Use when**: Extracting financial tables, invoices, or structured grids from PDF documents.
- **Use when**: Inspecting multi-page PDF documents for specific sections without loading the entire document into context.
- **Do NOT use when**: The PDF file exceeds the 100MB file size ceiling or 500-page limit (enforced fail-closed).
- **Do NOT use when**: The file is password-protected or DRM encrypted without operator credentials.

## Procedure (numbered, tool calls with params)
1. Verify file size and page bounds:
   - Check file size; refuse if >100MB.
2. Ingest document text page-by-page:
   - Call `read_file(path=pdf_path, limit=500)` to extract metadata, table of contents, and initial executive summary pages.
3. Target key data sections:
   - Extract targeted sections (e.g. Financial Overview, Conclusions, Methodology) using offset pagination.
4. Extract embedded tables or diagrams:
   - If pages contain scanned raster images or complex diagrams, call `vision_analyze(image_b64=page_png, prompt="Extract table as markdown")`.
5. Output structured Markdown triage report:
   - Document Title & Metadata
   - Executive Summary (3–5 bullet points)
   - Extracted Data Tables (formatted in Markdown)
   - Action Items or Outstanding Questions

## Worked example (input → tool sequence → output)
- **Input**: "Review quarterly_report.pdf and extract the financial summary table."
- **Tool sequence**:
  1. `read_file(path="quarterly_report.pdf", offset=0, limit=200)` -> reads executive summary
  2. `read_file(path="quarterly_report.pdf", offset=201, limit=300)` -> locates financial results table
- **Output**:
  "### Quarterly Financial Summary
  - **Revenue**: $14.2M (+12% YoY)
  - **Operating Margin**: 24.5%
  - **Key Finding**: Enterprise expansion drove 65% of net new ARR."

## Refuse when
- The PDF exceeds 100MB or 500 pages.
- The document is encrypted and cannot be decrypted without a missing passphrase.
