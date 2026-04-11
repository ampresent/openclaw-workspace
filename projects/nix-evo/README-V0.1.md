# nix-evo v0.1 — Final README

A progressive markdown toolkit that helps you go from "I have some notes" to "I have a polished, publishable document" through a series of optional refinement passes.

## What it does

Takes markdown input (file or stdin) and runs it through a customizable pipeline of transformation passes:

1. **Normalize** — Fix inconsistent formatting (headings, lists, spacing, code blocks)
2. **Extract Front Matter** — Pull or generate YAML metadata (title, date, word count, reading time)
3. **Table of Contents** — Generate TOC from headings with anchor links
4. **Cross References** — Detect and link repeated terms/concepts
5. **Admonitions** — Convert `NOTE:`, `TIP:`, `WARNING:` patterns to blockquote syntax
6. **Smart Punctuation** — Straight quotes → curly, dashes → em/en, ellipsis
7. **Reading Stats** — Add reading time estimates and word counts
8. **Lint Report** — Flag issues (long paragraphs, passive voice, etc.) without modifying

## Quick Start

```bash
# From stdin
echo "# My Doc\n\nSome content here." | nix-evo

# From file
nix-evo input.md

# With specific passes
nix-evo --passes normalize,front-matter,toc input.md

# Dry run (show what would change)
nix-evo --dry-run input.md

# Output to file
nix-evo input.md -o output.md
```

## Architecture

```
stdin/file → [Pass Manager] → [Pass 1] → [Pass 2] → ... → stdout/file
                              ↓
                        [Lint Report]
```

Each pass is independent and composable. The pass manager orchestrates execution order (some passes depend on others).

## Pass Details

### normalize
- Standardizes heading levels (no skipping)
- Fixes list indentation (2-space consistent)
- Normalizes code fence languages
- Collapses excessive blank lines (max 2)
- Trims trailing whitespace

### front-matter
- Extracts existing YAML front matter
- Generates if missing: title (from H1), date, word count, reading time
- Updates stale fields (e.g., word count after edits)

### toc
- Scans headings, generates nested TOC
- Inserts after front matter (or at top)
- Respects `<!-- toc-ignore -->` comments
- Configurable max depth

### cross-refs
- Builds glossary from H2/H3 headings
- Links first occurrence of each term
- Skips code blocks and headings

### admonitions
- `NOTE:` → `> 📘 **Note**`
- `TIP:` → `> 💡 **Tip**`
- `WARNING:` → `> ⚠️ **Warning**`
- `DANGER:` → `> 🚨 **Danger**`

### smart-punctuation
- `'text'` → `'text'` (curly singles)
- `"text"` → `"text"` (curly doubles)
- `--` → `–` (en dash)
- `---` → `—` (em dash)
- `...` → `…` (ellipsis)

### reading-stats
- Appends stats block at end (or front-matter)
- Word count, sentence count, reading time
- Flesch reading ease score (optional)

### lint-report
- Long paragraphs (>300 words)
- Passive voice detection (heuristic)
- Heading hierarchy issues
- Broken links (local file refs)
- Outputs report to stderr, doesn't modify document

## Dependencies

| Library | Purpose | License |
|---------|---------|---------|
| pulldown-cmark | Markdown parsing/rendering | MIT |
| clap | CLI argument parsing | MIT/APACHE |
| serde + serde_yaml | Front matter handling | MIT/APACHE |
| regex | Pattern matching | MIT/APACHE |
| chrono | Date handling | MIT |
| anyhow | Error handling | MIT |

Total dependency tree: ~15 crates (lean by Rust standards)

## Pass Dependencies

```
normalize ──→ front-matter ──→ toc ──→ cross-refs ──→ admonitions ──→ smart-punctuation ──→ reading-stats
                                                                                                    │
                                                                                              lint-report (read-only)
```

## Configuration

### CLI flags
```
--passes <list>         Comma-separated pass list (default: all)
--skip <list>           Passes to skip
--dry-run               Show diff without writing
--toc-depth <n>         Max TOC nesting (default: 3)
--front-matter <mode>   always/never/auto (default: auto)
--stats-location        front/back/both (default: back)
--lint-threshold <n>    Min severity to report (1-3, default: 1)
```

### Config file (.nix-evo.yml)
```yaml
passes:
  - normalize
  - front-matter
  - toc
  - reading-stats

normalize:
  heading_style: atx        # atx (#) or setext (underlines)
  list_indent: 2
  max_blank_lines: 2

front-matter:
  auto_generate: true
  fields: [title, date, word_count, reading_time]

toc:
  max_depth: 3
  position: after-front-matter

lint:
  max_paragraph_words: 300
  check_passive_voice: true
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Processing error |
| 2 | CLI argument error |
| 3 | Lint issues found (when --lint-fail is set) |

## Design Decisions

1. **Passes are pure(ish)**: Each pass takes markdown in, returns markdown out. No shared mutable state.
2. **Order matters**: Dependencies are enforced by the pass manager, not the user.
3. **Lint is non-destructive**: Reports never modify. Separate from transforms.
4. **Conservative by default**: Won't change content semantics, only formatting.
5. **Streaming where possible**: Large files should work without loading everything in memory.
