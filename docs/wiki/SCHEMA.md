# Wiki Schema

This wiki is an LLM-maintained knowledge base for the game-server project. The LLM owns all files in `docs/wiki/` — it creates, updates, and cross-references them. Humans read; the LLM writes.

## Structure

```
docs/wiki/
├── SCHEMA.md          # This file — conventions and workflows
├── index.md           # Content catalog (categories → pages with summaries)
├── log.md             # Chronological record of ingests/queries/lints
├── systems/           # Server systems (combat, AI, persistence, networking, etc.)
├── design/            # Architecture decisions, design specs, comparisons
├── investigations/    # Debug logs, root cause analyses, bug workarounds
└── reference/         # External resources, data sources, tooling
```

## Page Format

Every wiki page uses this template:

```markdown
# Page Title

One-paragraph summary.

## Content

Main content organized with headers.

## Sources

- [source-name](../relative-path.md) — what was used from this source

## See Also

- [[other-wiki-page]] — why it's related
```

Use `[[page-name]]` for internal wiki links (Obsidian-compatible). Use relative markdown links for source docs.

## Workflows

### Ingest

When processing a new source document:

1. Read the source fully
2. Identify which wiki pages it touches (entities, concepts, systems)
3. Create new pages or update existing ones
4. Update cross-references (`See Also` sections) on all affected pages
5. Update `index.md` with any new pages
6. Append an entry to `log.md`

A single source may touch 5-15 wiki pages. Always check existing pages before creating new ones — update > create.

### Query

When answering questions against the wiki:

1. Read `index.md` to find relevant pages
2. Read those pages and synthesize an answer
3. If the answer produces a valuable new page (comparison, analysis, synthesis), file it into the wiki

### Lint

Periodic health check. Look for:

- Contradictions between pages
- Stale claims superseded by newer sources
- Orphan pages with no inbound links
- Important concepts lacking their own page
- Missing cross-references
- Pages that reference removed/renamed source files

## Conventions

- **File names**: lowercase, hyphens, no dates unless the page is inherently temporal
- **Categories are directories**, not tags
- **One concept per page** — split rather than merge
- **Sources section is mandatory** — every claim traces back to a source doc or code file
- **Keep pages current** — when a newer source contradicts an older page, update the page and note the change
