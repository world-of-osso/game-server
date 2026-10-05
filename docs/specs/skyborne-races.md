# Skyborne races

Server support for Forever races 95/96, grounded in local build 1.60.1.70205 ChrRaces/CharBaseInfo and pinned FactionTemplate/Map snapshots. [Implementation and remaining world-start requirements](../wiki/systems/skyborne.md).

## What it must do

- [x] Accept only authored Skyborne classes: 95→1/3/4/8/11, 96→1/3/4/7/11; use verified race bits 32/33.
- [ ] Create all ten pairs at level 1 with actual Forever70205 starting items/counts, persist across storage reopen, and select them without Retail kit leakage.
- [x] Assign Alliance/template 3629 to race 95 and Horde/template 3628 to race 96; imported templates yield friendly allied and hostile opposing reactions.
- [x] Decode only source-backed faction rows 3628/3629 and Map 2991; preserve other Retail rows, reapply the overlay during regeneration, and record row provenance.
- [x] Reject source corruption before changing the destination database.
- [x] Decode the six pinned starting loadouts (2373–2378), their 45 memberships and all 30 referenced Item/ItemSparse rows, plus clear related appearance/effect rows with exact provenance. Accept only Purpose 9, ModID 75, extra field 0, masks (0,3).
- [x] Reject undeclared row-ID collisions rather than replacing Retail loadout/item data; rollback publication atomically.
- [x] Preserve Retail definitions; isolate all 30 Forever kit definitions in six `skyborne_item*` tables and project `skyborne_content_item` using the existing content policy. Regeneration retains isolated definitions and exact row provenance.

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
- `crates/server/src/auth_character_tests.rs` — creation, storage reopen and message-boundary selection; not sufficient alone for kit-content proof.
- `crates/server/src/skyborne_kit_tests.rs` — importer-generated exact ten-pair counts, source-local equipment and redb reopen, Retail preservation and required-content failure.
- `scripts/tests/test_skyborne_data.py`, `test_skyborne_kits.py` — source decoding, full DB2/content regeneration, conflicting shared definitions preserved separately, missing source/parent rollback, and corruption/collision rejection.

## Known gaps (current cycle)

- [x] New races 95/96 persist map 2991 and approved reconstructed starts: 95→(4088.501,1847.5,976.169), 96→(4051.417,1858.625,977.620). Race 96 is inferred, not an established level-1 intro. Initial facing is atan2(giver Y−start Y,giver X−start X), converted to replicated yaw; existing characters are not relocated.
- [ ] Full terrain/LOS closure and Forever racial/loadout parity remain separate gaps.

## Out of scope

Forever racial spells and language/skill parity remain excluded. Shared kit IDs keep separate Forever definitions under the approved source-isolation boundary. Runtime stats, validation, reload and client tooltips must resolve that same source before full kit support is claimed. [Reconstructed content](zephras-content.md) is a separate contract; client assets and full LOS closure remain independent gaps.
