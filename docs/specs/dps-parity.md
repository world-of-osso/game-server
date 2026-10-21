# Native item-instance DPS parity

Compare native 70448 server combat with shipped MID2 SimulationCraft 76f8118 profiles; Evoker uses its explicitly approved prior-tier MID1 reference. Sources: `scripts/dps_parity.py`, `scripts/item_instance_data.py`, `crates/server/src/item_instance.rs`; [character stats](../wiki/systems/character-stats.md) describes the existing production stat path.

## What it must do

### Equipment

- [ ] Instances retain bonus lists, an optional explicit item level, permanent enchant and gem item IDs by GUID, with backward-compatible redb storage. Crafted stat selections survive where the profile specifies them.
- [ ] Equipment stats resolve native70448 ItemBonus, ItemSparse, RandPropPoints and Midnight scaling/configuration tables, including stat allocation, sockets and enchant/gem effects. Unsupported effects are named gaps, never invented stats.
- [x] The native-table producer preserves signed values, replaces only the requested table, supports repeated imports and rolls back a duplicate-ID failure without losing the prior table.
- [x] Shipped profile manifests preserve exact item IDs, bonus lists, item-level overrides, enchants, gems, crafted stats, talents, consumables and action text. Missing item IDs fail explicitly.
- [x] Reference selection uses MID2 for the eight available DPS/tank profiles and prior-tier MID1 for Evoker, labelled `SIMC_DEVASTATION_MID2_PROFILE_ABSENT`; it never silently substitutes another tier.
- [ ] Selected-talent metadata preserves SimC's printed node IDs, entry IDs and ranks; absent metadata must not silently become a default build.

### Measurement

- [ ] A headless300s follower dummy harness consumes exact profile gear through the production stat path and supported talents/consumables, on one target and a specified three-target DungeonSlice-like setup.
- [ ] Reports compare primary/crit/haste/mastery/versatility/AP/SP first, then action counts/average hit/damage share and buff uptimes. Flag every delta>5% and classify stats/gear, coefficients, missing proc/aura or rotation/decision.
- [x] SimC action normalization includes direct and periodic amounts without double-counting parent compound totals.
- [ ] Compare SimC70391 and native70448 effect rows before attributing a coefficient mismatch to a server bug. Rotation/decision edits remain lead-owned.
- [ ] Cover specs66/73/105/257/262/63/253/1467/269/266/103. Missing105/257 shipped healer profiles remain explicit HPS-reference gaps, not synthetic BiS profiles. Report first-three and all-spec summaries to the lead.

## How it works

- [Character stats and gear](../wiki/systems/character-stats.md#gear).
- [Inventory](../wiki/systems/inventory.md).

## Implementation inventory

- `scripts/item_instance_data.py` — native-table producer for a supplied existing SQLite copy; canonical application is lead-owned.
- `scripts/dps_parity.py` — explicit current-tier/prior-tier profile extraction and SimC action normalization.
- `crates/server/src/item_instance.rs` — pure instance-level stat budget resolution, consumed by production equipment Stats.
- `crates/server/src/item_instance/{bonuses,effects,load,store}.rs` — native bonus application/SQLite loading and additive GUID-keyed redb state. Type23 preserves ItemEffect IDs; base/bonus resolution retains native spell IDs, trigger types and cooldown sentinels. Equip-proc execution is not yet integrated.
- `crates/server/src/stats.rs` — production stat consumer of slot-bound instance properties.
- `crates/server/src/{inventory,inventory_model,persistence_inventory,item_stats,persistence}.rs` — owned GUID hydration/projection, initial/delta delivery, startup catalogs and additive redb initialization.

## Tests asserting this spec

- `scripts/tests/test_item_instance_data.py` — concrete signed rows, idempotence, unrelated-table retention and duplicate-ID rollback.
- `scripts/tests/test_dps_parity.py` — exact item identities/modifiers, reference-tier selection, missing-ID rejection, selected-talent metadata and direct/periodic action accounting.
- `crates/server/src/item_instance.rs::tests` — two passing item-level budget/weapon-damage and invalid-level regressions.
- `crates/server/src/item_instance/{bonuses,store}.rs::tests` — six native allocation/configuration/socket/crafted-stat and durable-reopen/owner-transfer/rollback regressions. Owned inventory hydration/projection is additionally proved at the persistence/Stats boundary.
- `crates/server/src/item_instance/load.rs::tests` — native SQLite configuration and enchant/gem join regressions; both passed within the 15-case library proof.
- `crates/server/src/stats_instance_tests.rs` — actual StatsPlugin modifier-only change on native chest 271468/enchant 7987; genuine RED missed +50 Strength before consumer wiring; subsequent native GREEN and unchanged legacy Arcane control passed.

## Known gaps (current cycle)

- [ ] Remaining profile bonus options, equip-spell aura/proc lifecycle and complete instance-movement qualification; basic Stats derivation and additive persistence have targeted GREEN proof.
- [ ] Real300s server dummy execution and equal-gear parity reports.
- [ ] Shipped105/257 healer BiS/HPS references are absent upstream.

## Out of scope

Rotation/decision implementation belongs to the lead/rotation agents. No system install, native-table re-export, canonical data mutation or protocol change without demonstrated need. Reference SimC is the official local CLI-only source build at76f8118cf3f7f41dad20708e8328e1f4ab2deebc (embedded70391); server inputs remain70448. Durable evidence lives in `game-engine/data/diagnostics/dps-parity-2026-10-21/`.
