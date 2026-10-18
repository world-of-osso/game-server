# Skyborne races

Server support for Forever races 95/96, grounded in local build 1.60.1.70205 ChrRaces/CharBaseInfo and pinned FactionTemplate/Map snapshots. [Implementation and remaining world-start requirements](../wiki/systems/skyborne.md).

## What it must do

- [x] Accept only authored Skyborne classes: 95→1/3/4/8/11, 96→1/3/4/7/11; use verified race bits 32/33.
- [x] Create all ten pairs at level 1 with actual Forever70205 starting items/counts, persist across storage reopen, and select them without Retail kit leakage.
- [x] Assign Alliance/template 3629 to race 95 and Horde/template 3628 to race 96; imported templates yield friendly allied and hostile opposing reactions.
- [x] Decode only source-backed faction rows 3628/3629 and Map 2991; preserve other Retail rows, reapply the overlay during regeneration, and record row provenance.
- [x] Reject source corruption before changing the destination database.
- [x] Decode the six pinned starting loadouts (2373–2378), their 45 memberships and all 30 referenced Item/ItemSparse rows, plus clear related appearance/effect rows with exact provenance. Accept only Purpose 9, ModID 75, extra field 0, masks (0,3).
- [x] Reject undeclared row-ID collisions rather than replacing Retail loadout/item data; rollback publication atomically.
- [x] Preserve Retail definitions; isolate all 30 Forever kit definitions in six `skyborne_item*` tables and project `skyborne_content_item` using the existing content policy. Full DB2/content regeneration and the additive kit refresh retain isolated definitions and exact row provenance. Standalone publication rolls back the entire overlay if content projection fails.

- [x] Import exactly nine source-pinned Forever item-scaling CSVs atomically into isolated `skyborne_` tables in an existing owned scratch database; preserve Retail schemas/rows/provenance. Validate build, FDID/layout/raw/schema pins, CSV hashes/columns/row IDs and required levels before publication; missing/tampered source or SQL failure leaves no partial refresh. Publish only authored fields and exact per-row raw provenance, never Retail scaling substitutes.

- [x] Load normal Forever armor, weapon and stat scaling from the pinned schema without requiring Retail-only spell-budget fields. Forever `spell_budgets` is empty; Retail values remain mandatory and source-owned. Normal Forever scaling tables remain mandatory. This loader support does not implement Forever spell evaluation.

## How it works

- [Skyborne system](../wiki/systems/skyborne.md)
- [Faction reactions](../wiki/systems/factions.md)

## Implementation inventory

- `crates/server/src/race_class.rs`, `faction.rs` — allowed pairs, masks and faction identity.
- `scripts/skyborne_data.py`, `db2_casc.py` — pinned local source decode and selected-row import. Raw DB2s stay outside the tracked tree; CLI consumers require `--source-dir`, tests require `FOREVER_DB2_SOURCE` ([cache paths and provenance](../../scripts/reference/forever-1.60.1.70205/README.md#local-data-paths)).
- `scripts/db2.py` — atomic Retail regeneration with overlay.
- `scripts/skyborne_scaling_data.py`, `scripts/reference/forever-scaling-70205.json` — isolated source-pinned scaling publication.
- `crates/server/src/skyborne_faction_fixture.rs` — decoder-generated SQLite reaction fixture; no live overlay required.

## Tests asserting this spec

- `crates/server/src/race_class.rs`, `faction_tests.rs` — pair/mask/faction rules and imported reactions.
- `crates/server/src/auth_character_tests.rs` — creation, storage reopen and message-boundary selection; not sufficient alone for kit-content proof.
- `crates/server/src/skyborne_kit_tests.rs` — importer-generated exact ten-pair counts, source-local equipment and redb reopen, Retail preservation and required-content failure.
- `scripts/tests/test_skyborne_data.py`, `test_skyborne_kits.py` — source decoding, full DB2/content regeneration, conflicting shared definitions preserved separately, missing source/parent rollback, and corruption/collision rejection.

- `scripts/tests/test_skyborne_scaling_data.py` — actual catalog floats, deterministic publication, Retail SQL/blob preservation, exact provenance, missing/tampered source and process-level rollback.

## Known gaps (current cycle)

- [x] New races 95/96 persist map 2991 and approved reconstructed starts: 95→(4088.501,1847.5,976.169), 96→(4051.417,1858.625,977.620). Race 96 is inferred, not an established level-1 intro. Initial facing is atan2(giver Y−start Y,giver X−start X), converted to replicated yaw; existing characters are not relocated.
- [ ] Full terrain/LOS closure and Forever racial/loadout parity remain separate gaps.
- [ ] Forever item-spell scaling classes -8/-9 remain unsupported: RandPropPoints FDID 1310245/layout 4FD22743 declares only ID and Good/Superior/Epic arrays of five values, not DamageReplaceStatF or DamageSecondaryF. No invented zero columns, Retail borrowing or fallback. Existing Retail-only caster lookup is outside this loader integration.

Loader regression: `crates/server/src/item_stats_tests.rs` uses exact pinned CSV headers/level-1 rows with integer keys in a temporary database, distinct Retail values, actual loader/stat outputs and missing-source errors. Behavioral RED at `d342155` fails on the absent Forever `DamageReplaceStatF` column; the Retail-required-field case passes. The loader limits spell-budget loading to Retail. Desktop targeted GREEN at `c1cdf6e` passes all three cases: normal Forever stats/armor/weapon outputs and empty spell budgets, missing Forever weapon-table rejection, and mandatory Retail spell-budget fields. No Forever spell or native-world proof is claimed. Prior compile-blocked and fixture-type failures are not behavioral loader evidence. Command, revision/scope hashes and results: `target/pinned-70205-loader-proof/ledger.json`.

## Out of scope

Forever racial spells and language/skill parity remain excluded. Shared kit IDs keep separate Forever definitions under the approved source-isolation boundary. [Owned source contract](item-definition-source.md) covers implemented source-qualified runtime validation, scaling and reload plus remaining integration/transfer gates. The ten-pair creation/reopen test passes at `e405232`; no full Classic parity or native client E2E claim. [Reconstructed content](zephras-content.md) is a separate contract; client assets and full LOS closure remain independent gaps.
