# Dungeon progress and achievements

Server-only phase 1: ordered boss progress and DB2-driven character achievements. Source: `crates/server/src/achievement.rs`, `achievement_catalog.rs`, `maps/`, and `scripted_ai/instance_script.rs`. [Implementation and provenance](../wiki/systems/dungeon-achievements.md).

## What it must do

- [x] Read DungeonEncounter map/difficulty/order/bit rows, including negative order indices; isolate defeated encounters per copy.
- [x] Successful scripted deaths record encounter progress; duplicate DONE does not credit it.
- [x] Persist scheduled instance boss/encounter masks with existing instance locks; normal-copy checkpoints survive restart without acquiring scheduled reset restrictions and clear on manual reset.
- [x] Grant Stockade achievement 633 after all three encounters, using its real Hogger criterion; deliver existing toast and snapshot to that copy's players only.
- [x] Evaluate unconditional kill-creature (0), reach-level (5), and defeat-dungeon-encounter (165) criteria from DB2.
- [x] Evaluate Single/All/Any and direct-child count operators, preserving numeric progress in redb.
- [x] Grant Retail achievements at the previous 10/20/40/60/70/80 thresholds, using actual Retail IDs rather than placeholder 1–6; migrate persisted placeholders once.

## How it works

- [Dungeon achievements](../wiki/systems/dungeon-achievements.md)
- [Boss scripts](../wiki/systems/boss-scripts.md)

## Implementation inventory

- `scripts/import_achievements.py` — pinned local DB2 CSV export and atomic SQLite import.
- `scripts/db2_casc.py` — existing WDC5 reader, extended for decrypted sections and virtual string blocks.
- `crates/server/src/achievement_catalog.rs` — supported catalog, counters, tree evaluation and grants.
- `crates/server/src/achievement.rs` — persistent character state, gameplay events and existing message delivery.
- `crates/server/src/maps/store.rs`, `instances.rs` — ordered encounter rows and copy masks.
- `crates/server/src/scripted_ai/instance_script.rs` — successful encounter hook.

## Tests asserting this spec

- `crates/server/src/achievement_catalog_tests.rs`
- `crates/server/src/achievement_tests.rs`
- `crates/server/src/achievement_normal_lock_tests.rs`
- `crates/server/src/scripted_ai/tests/dungeonach.rs`
- `scripts/tests/test_db2_sections.py`

## Known gaps (current cycle)

No open phase-1 acceptance cases. Targeted proof: eight tests passed at `702cb80`, Stockade fixture passed at `96cad53`; the latter changes only its message-draining test fixture. See wiki proof ledger. Weighted/sum/max operators are implemented but lack dedicated acceptance cases; source-verified Retail parity remains unavailable without local TrinityCore.

## Out of scope

- Client objective tracker / achievement window and new protocol messages (phase 2).
- ModifierTree, timed, world-state, faction-specific, account-wide, unsupported/inverse criteria; their achievements remain disabled rather than partially evaluated.
- Achievement_Category: no category UI in phase 1.
