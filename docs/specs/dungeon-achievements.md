# Dungeon progress and achievements

Protocol/server phases 1–2: ordered dungeon objectives, DB2-driven character achievements and bounded achievement-window data. Source: `crates/server/src/achievement.rs`, `achievement_catalog.rs`, `maps/`, and `scripted_ai/instance_script.rs`. [Implementation and provenance](../wiki/systems/dungeon-achievements.md).

## What it must do

- [x] Read DungeonEncounter map/difficulty/order/bit rows, including negative order indices; isolate defeated encounters per copy.
- [x] Successful scripted deaths record encounter progress; duplicate DONE does not credit it.
- [x] Persist scheduled instance boss/encounter masks with existing instance locks; normal-copy checkpoints survive restart without acquiring scheduled reset restrictions and clear on manual reset.
- [x] Grant Stockade achievement 633 after all three encounters, using its real Hogger criterion; deliver existing toast and snapshot to that copy's players only.
- [x] Evaluate unconditional kill-creature (0), reach-level (5), complete-quest (27), earned-achievement dependency (8), and defeat-dungeon-encounter (165) criteria from DB2.
- [x] Credit quest completion only after successful reward persistence; incomplete/rejected turn-ins never credit. Repeat reward events remain boolean, not accumulated counts.
- [x] Recover quest criteria from the character's persisted rewarded quest IDs on login; dependency criteria read persisted earned IDs and cascade newly earned meta-achievements to a fixed point. Reconnect sends snapshots without replaying earned toasts.
- [x] Match Achievement.Faction (-1 both, 0 Horde, 1 Alliance) to the player's authored faction template; exclude opposing/neutral faction-only rows from grants, catalog/criteria pages and live progress.
- [x] Exclude account-wide grants with Retail ACHIEVEMENT_FLAGS_ACCOUNT (0x20000), not SHOW_CRITERIA_MEMBERS (0x10000). Do not present character counters or historical character-local earned records as account progress; retain persisted history for the later account migration.
- [x] Evaluate Single/All/Any and direct-child count operators, preserving numeric progress in redb.
- [x] Grant Retail achievements at the previous 10/20/40/60/70/80 thresholds, using actual Retail IDs rather than placeholder 1–6; migrate persisted placeholders once.

### Phase 2

- [x] Import pinned local Achievement_Category FDID 1324299, category parents/order, and achievement category/icon metadata atomically with the phase-1 tables.
- [x] Send ordered encounter names and defeated state on instance entry, reconnect and each successful encounter defeat, scoped to the copy; preserve raw flags and unknown optionality.
- [x] Clear dungeon objective rows on leaving an instance, even when DungeonEncounter also contains world-boss rows for the continent.
- [x] Serve categories, category achievements and criterion lines on request with bounded pages; never push the catalog on login.
- [x] Include character earned state, known completion dates and numeric criterion progress in category responses; preserve unknown historical dates.

## How it works

- [Dungeon achievements](../wiki/systems/dungeon-achievements.md)
- [Phase-2 client hookup and payload bounds](../dungeon-achievements-client.md)
- [Boss scripts](../wiki/systems/boss-scripts.md)

## Implementation inventory

- `scripts/import_achievements.py` — pinned local DB2 CSV export and atomic SQLite import.
- `scripts/db2_casc.py` — existing WDC5 reader, extended for decrypted sections and virtual string blocks.
- `crates/server/src/achievement_catalog.rs` — supported catalog, counters, tree evaluation and grants.
- `crates/server/src/achievement_window.rs` — bounded presentation catalog and criterion continuations.
- `crates/server/src/dungeon_progress.rs` — copy entry/reconnect and defeat snapshots.
- `crates/server/src/achievement.rs` — persistent character state, gameplay events and existing message delivery.
- `crates/server/src/maps/store.rs`, `instances.rs` — ordered encounter rows and copy masks.
- `crates/server/src/scripted_ai/instance_script.rs` — successful encounter hook.

## Tests asserting this spec

- `crates/server/src/achievement_catalog_tests.rs`
- `crates/server/src/achievement_tests.rs`
- `crates/server/src/achievement_window_tests.rs` — real catalog continuation and unknown-ID behavior.
- `crates/server/src/achievement_normal_lock_tests.rs`
- `crates/server/src/scripted_ai/tests/dungeonach.rs`
- `scripts/tests/test_db2_sections.py`
- `scripts/tests/test_achievement_import.py` — real local categories and Stockade metadata.

## Known gaps (current cycle)

Specified phase-2 acceptance cases pass; see the wiki proof ledger for exact revisions, commands and retained regression evidence. No verified optional DungeonEncounter flag is available: optionality stays unknown and raw flags are preserved. Weighted/sum/max operators from phase 1 still lack dedicated acceptance cases; no new evaluation or Retail-parity claim is made.

## Out of scope

- Client objective tracker / achievement window rendering; client stays read-only in phase 2.
- ModifierTree, timed, world-state, account-wide, unsupported/inverse criteria; their achievements remain disabled rather than partially evaluated. Account aggregation/persistence/migration is the next slice.
- Additional criteria types beyond 0/5/8/27/165.
