# F18 — File Classification Sensor

**Phase:** 3 — AI
**Size:** M (1-2 sessions)
**Branch:** `feat/P3-file-classifier`
**Depends on:** F15, F05

## Objective

AI-powered sensor that categorizes files by type and project, detects duplicates, and suggests organization improvements.

## Deliverables

### 1. FileClassificationProbe
- Scan user-specified directories (never full filesystem without consent)
- Classify by: document type, project grouping, age, access frequency
- Detect duplicates: exact (SHA-256) and near-duplicate (simhash for text, perceptual hash for images)

### 2. Classification Categories
- Documents (PDF, DOCX, TXT)
- Media (images, video, audio)
- Code (by language)
- Archives (zip, tar, etc.)
- Data (CSV, JSON, databases)
- Temporary/cache
- Unknown

### 3. Organization Suggestions
- Group scattered project files
- Identify large unused files
- Flag duplicate clusters with size totals
- Suggest archive candidates (untouched > 1 year)

### 4. Privacy Constraints
- **Read metadata only** — never read file contents for classification
- File name + extension + size + dates = sufficient for most classification
- LLM sees aggregated summaries, never individual filenames
- User selects which directories to scan

## Acceptance Criteria

- [ ] Scans only user-approved directories
- [ ] Classifies by metadata only (no content reading)
- [ ] Duplicate detection via hash (opt-in, reads content for hashing only)
- [ ] Results displayed in UI with category breakdown
- [ ] Suggestions are recommendations (pass through approval gate)
- [ ] Large directory scans show progress
- [ ] Never stores full file paths in AI prompts
