# Dungeon progress and achievements

Protocol/server phases 1–2: ordered dungeon objectives, DB2-driven character achievements and bounded achievement-window data. Source: `crates/server/src/achievement.rs`, `achievement_catalog.rs`, `maps/`, and `scripted_ai/instance_script.rs`. [Implementation and provenance](../wiki/systems/dungeon-achievements.md).

## What it must do

- [x] Read DungeonEncounter map/difficulty/order/bit rows, including negative order indices; isolate defeated encounters per copy.
- [x] Successful scripted deaths record encounter progress; duplicate DONE does not credit it.
- [x] Persist scheduled instance boss/encounter masks with existing instance locks; normal-copy checkpoints survive restart without acquiring scheduled reset restrictions and clear on manual reset.
- [x] Grant Stockade achievement 633 after all three encounters, using its real Hogger criterion; deliver existing toast and snapshot to that copy's players only.
- [x] Evaluate unconditional kill-creature (0), reach-level (5), complete-quest (27), earned-achievement dependency (8), successful spell casts (29), current base skill ranks (7), spell-target hits (28), and defeat-dungeon-encounter (165) criteria from DB2.
- [x] Credit quest completion only after successful reward persistence; incomplete/rejected turn-ins never credit. Repeat reward events remain boolean, not accumulated counts.
- [x] Recover quest criteria from the character's persisted rewarded quest IDs on login; dependency criteria read persisted earned IDs and cascade newly earned meta-achievements to a fixed point. Reconnect sends snapshots without replaying earned toasts.
- [x] Match Achievement.Faction (-1 both, 0 Horde, 1 Alliance) to the player's authored faction template; exclude opposing/neutral faction-only rows from grants, catalog/criteria pages and live progress.
- [x] Separate account-wide grants with Retail ACHIEVEMENT_FLAGS_ACCOUNT (0x20000), not SHOW_CRITERIA_MEMBERS (0x10000). Aggregate durable character contributions within the owning account; migrate historical progress without changing other character rows. Never persist account totals into character counters.
- [x] Retain account earned IDs and earliest known dates across reconnect, restart and character deletion. Account criteria pages and snapshots use account aggregates; character rows use their own counters. Unrelated accounts remain isolated.
- [x] Evaluate Single/All/Any and direct-child count operators, preserving numeric progress in redb.
- [x] Grant Retail achievements at the previous 10/20/40/60/70/80 thresholds, using actual Retail IDs rather than placeholder 1–6; migrate persisted placeholders once.

### Phase 2

- [x] Import pinned local Achievement_Category FDID 1324299, category parents/order, and achievement category/icon metadata atomically with the phase-1 tables.
- [x] Send ordered encounter names and defeated state on instance entry, reconnect and each successful encounter defeat, scoped to the copy; preserve raw flags and unknown optionality.
- [x] Clear dungeon objective rows on leaving an instance, even when DungeonEncounter also contains world-boss rows for the continent.
- [x] Serve categories, category achievements and criterion lines on request with bounded pages; never push the catalog on login.
- [x] Include character earned state, known completion dates and numeric criterion progress in category responses; preserve unknown historical dates.

### Native Summary

- [ ] Serve full faction-eligible non-statistic, non-guild catalog counts and earned points, independent of client-loaded pages; distinguish All, account-flag and character-flag scopes.
- [ ] Include category totals with descendant contributions and the newest three earned rows, known dates before unknown dates. Preserve authoritative account aggregates and character isolation.
- [ ] Expose authentic Achievement.Flags with every row for native account/character and Statistics presentation.
- [ ] Resolve individual achievement IDs to authoritative own-character/account state for chat/toast navigation; reject opposing-faction rows without inventing completion data.

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
- ModifierTree, timed, world-state and unsupported/inverse criteria; their achievements remain disabled rather than partially evaluated.
- Exploration43 uses pinned local WorldMapOverlay→AreaTable.AreaBit joins and authoritative living, non-taxi terrain exploration, persisted per character; overlay IDs are never treated as AreaIDs. Reputation46 uses signed Retail standing and the recoverable-delta migration documented below; never compare legacy unsigned63000 to Retail42000.
- Additional criteria types beyond 0/5/7/8/27/28/29/165. Target hits credit each distinct recipient once after defense/missile filtering, including triggered spells, not the caster. Skill rows require an authored local SkillLine; loaded and changed nonzero ranks set numeric progress, never use maximum rank or increment it. Cast rows require a local spell and no DB2 conditions; TDB1210 holiday404 criteria11118–11127 remain disabled.

### Continued source-backed coverage

- Type43: pinned WorldMapOverlay areas, Retail AreaBit persistence and actual living/non-taxi terrain discovery; no overlay/area ID substitution.
- Type73: authoritative successful spell effect61 sends its authored MiscValue to the player caster. Targetless effects execute once; unit-target effects use the hit/immunity boundary. Timers, fail-event conditions, unknown modifiers and absent scripted producers remain unevaluated.
- Type39 rated arena remains unsupported: no rating producer exists. Never seed a rating to claim support.
- Type69: new authoritative aura applications credit the receiving player after successful installation; refreshes, immune/empty applications and dead-target rejection do not credit.
- Type132 rated battleground rating has no native rating producer, matching the type39 gap. Type205 lacks authored local TransmogSet/TransmogSetGroup membership and a group-completion producer; collection IDs are not inferred.
- Types1/130: native objective terminal wins credit only accepted, authenticated winning participants still in the completed copy; draws, losses, leaving and repeated ticks do not credit. Rated rows remain unsupported.
- Type261 ReachRenownLevel remains blocked by absent native renown amounts/levels; signed traditional standings are not renown levels.
- Type42: only actual loot-slot commits (including group-roll delivery) add the awarded item count. General acquisition36 remains separate; vendor/craft rewards and inventory moves cannot impersonate looting.
- Type152 current producer gap: the only mapped, structurally executable matching scenario58 is on1031, rejected by native InstanceManager as MapNotAllowed. Gnomeregan649 is playable but has no type152 criterion; all type151 rows need unsupported modifiers. No synthetic admission, binding or credit is claimed.
- Type68: native successful goober/chest use and Font-of-Power activation credit actual authored GameObject entry after reach, life, map, lock, quest/cooldown and qualified-font validation. Requests and denied uses cannot credit. Portal/flag/minigame sources not owned by these handlers remain unclaimed.
