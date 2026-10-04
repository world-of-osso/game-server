# Skyborne races

Server support for Forever races 95/96, grounded in local build 1.60.1.70205 ChrRaces/CharBaseInfo and pinned FactionTemplate/Map snapshots. [Implementation and remaining world-start requirements](../wiki/systems/skyborne.md).

## What it must do

- [x] Accept only authored Skyborne classes: 95→1/3/4/8/11, 96→1/3/4/7/11; use verified race bits 32/33.
- [x] Create at level 1, persist across storage reopen, and select all ten pairs with real Retail kit data.
- [x] Assign Alliance/template 3629 to race 95 and Horde/template 3628 to race 96; imported templates yield friendly allied and hostile opposing reactions.
- [x] Decode only source-backed faction rows 3628/3629 and Map 2991; preserve other Retail rows, reapply the overlay during regeneration, and record row provenance.
- [x] Reject source corruption before changing the destination database.

## How it works

- [Skyborne system](../wiki/systems/skyborne.md)
- [Faction reactions](../wiki/systems/factions.md)

## Implementation inventory

- `crates/server/src/race_class.rs`, `faction.rs` — allowed pairs, masks and faction identity.
- `scripts/skyborne_data.py`, `db2_casc.py` — pinned local source decode and selected-row import.
- `scripts/db2.py` — atomic Retail regeneration with overlay.
- `crates/server/src/skyborne_faction_fixture.rs` — decoder-generated SQLite reaction fixture; no live overlay required.

## Tests asserting this spec

- `crates/server/src/race_class.rs`, `faction_tests.rs` — pair/mask/faction rules and imported reactions.
- `crates/server/src/auth_character_tests.rs` — real-kit creation, storage reopen and message-boundary selection.
- `scripts/tests/test_skyborne_data.py` — source decoding, regeneration, row preservation and corruption rejection.

## Known gaps (current cycle)

- [ ] Authored map-2991 start belongs to the separate world-start investigation; current Northshire placement remains unchanged.

## Out of scope

Forever racial spells, language/skill/loadout parity, content spawns, asset extraction and LOS baking are not invented or imported in this bounded server slice.
