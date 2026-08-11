# game-server

Bevy 0.19 authoritative game server for the WoW project (Lightyear 0.28 networking, redb auth/characters, SQLite world data).

## Wiki

LLM-maintained knowledge base at `docs/wiki/`. See `docs/wiki/SCHEMA.md` for conventions and workflows. Read `docs/wiki/index.md` first when answering questions about the project.

### Wiki Maintenance

After completing work that produces knowledge worth preserving, update the wiki:

- **Investigations/debugging**: Create or update a page in `investigations/` with root cause, symptoms, and fix.
- **New systems or major changes**: Create or update a page in `systems/` describing how the system works.
- **Architecture decisions**: Record rationale in `design/`.

Follow the workflow in `SCHEMA.md`: check existing pages first (update > create), maintain cross-references, update `index.md` and `log.md`. Skip wiki updates for routine bug fixes, config tweaks, and cosmetic changes.

## Data

- `data/world.db` (SQLite) stores world/reference data loaded at runtime (creature templates, creature spawns, waypoint data).
- `scripts/world_schema.sql` defines the SQLite schema used for `data/world.db`.
- `data/game.redb` stores account/auth/character persistence.

## Race Metadata

- Character race metadata should be stored in `data/world.db` in a `chr_races` table when available.
- Source of truth for race rows: Wago DB2 `ChrRaces` export.
- Seed script: `scripts/seed_chr_races.sql` (imports current playable races into `chr_races`).
