# game-server

Bevy 0.18 authoritative game server for the WoW project (Lightyear networking, redb auth/characters, SQLite world data).

## Data

- `data/world.db` (SQLite) stores world/reference data loaded at runtime (creature templates, creature spawns, waypoint data).
- `scripts/world_schema.sql` defines the SQLite schema used for `data/world.db`.
- `data/game.redb` stores account/auth/character persistence.

## Race Metadata

- Character race metadata should be stored in `data/world.db` in a `chr_races` table when available.
- Source of truth for race rows: Wago DB2 `ChrRaces` export.
- Seed script: `scripts/seed_chr_races.sql` (imports current playable races into `chr_races`).
