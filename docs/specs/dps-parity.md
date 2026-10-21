# Native item-instance DPS parity

Compare native 70448 server combat with shipped MID2 SimulationCraft 76f8118 profiles; Evoker uses its explicitly approved prior-tier MID1 reference. Sources: `scripts/dps_parity.py`, `scripts/item_instance_data.py`, `crates/server/src/item_instance.rs`; [character stats](../wiki/systems/character-stats.md) describes the existing production stat path.

## What it must do

### Equipment

- [ ] Instances retain bonus lists, an optional explicit item level, permanent enchant and gem item IDs by GUID, with backward-compatible redb storage. Crafted stat selections survive where the profile specifies them. A pre-existing database without the additive modifier table loads as unmodified gear; unrelated storage errors and malformed modifier bytes still fail explicitly.
- [ ] Equipment stats resolve native70448 ItemBonus, ItemSparse, RandPropPoints and Midnight scaling/configuration tables, including stat allocation, sockets and enchant/gem effects. Physical-resistance enchants (type4/school0) contribute scaled, truncated, minimum-one armor, preserving their independent primary effects (TC a352b1fa Player.cpp12430–12447). Other resistance schools remain explicit gaps. Unsupported effects are named gaps, never invented stats.
- [x] The native-table producer preserves signed values, replaces only the requested table, supports repeated imports and rolls back a duplicate-ID failure without losing the prior table.
- [x] Shipped profile manifests preserve exact item IDs, bonus lists, item-level overrides, enchants, gems, crafted stats, talents, consumables and action text. Missing item IDs fail explicitly.
- [x] Reference selection uses MID2 for the eight available DPS/tank profiles and prior-tier MID1 for Evoker, labelled `SIMC_DEVASTATION_MID2_PROFILE_ABSENT`; it never silently substitutes another tier.
- [ ] Native base-item, bonus-added and enchant equip spells use the production aura path; unequipping an item or removing a modifier removes only its owned equip aura and linked child. Item-level changes replace stale aura amounts. Native LINKED/LINKED_2 zero amounts preserve child BP0; nonzero parent amounts override child BP0, with matching removal (TrinityCore a352b1fa `SpellAuraEffects.cpp5299–5335`).
- [ ] Instance-sensitive stack merging must not weaken catalog-based unique-equipped restrictions; different enchants do not permit a duplicate unique item.
- [ ] Selected-talent metadata preserves SimC's allocated node IDs, entry IDs and ranks, including each entry of a tiered node; absent metadata must not silently become a default build. The pinned debug log consumes tiered ranks before printing, so metadata comes from actual `player_traits` via the separate diagnostic CLI wrapper, not that log.

### Measurement

Three-target references omit the fight-style preset: pinned SimC `sim.cpp2318` clears explicit raid events for Patchwerk after parsing. The opt-in CLI integration test must observe three actual targets and an adds event; argument-string checks alone did not detect this failure.

- [ ] A headless300s follower dummy harness consumes exact profile gear through the production stat path and supported talents/consumables, on one target and a specified three-target DungeonSlice-like setup.
- [ ] Reports compare primary/crit/haste/mastery/versatility/AP/SP first, then action counts/average hit/damage share and buff uptimes. Flag every delta>5% and classify stats/gear, coefficients, missing proc/aura or rotation/decision.
- [x] SimC action normalization includes direct and periodic amounts without double-counting parent compound totals.
- [x] Stat comparison accepts identically-unitized attribute/stat maps, flags strictly greater than5% differences, distinguishes absent and zero-reference measurements, and rejects nonfinite inputs.
- [ ] Compare SimC70391 and native70448 effect rows before attributing a coefficient mismatch to a server bug. Rotation/decision edits remain lead-owned.
- [ ] Cover specs66/73/105/257/262/63/253/1467/269/266/103. Missing105/257 shipped healer profiles remain explicit HPS-reference gaps, not synthetic BiS profiles. Report first-three and all-spec summaries to the lead.

## How it works

- [Character stats and gear](../wiki/systems/character-stats.md#gear).
- [Inventory](../wiki/systems/inventory.md).

## Implementation inventory

- `scripts/item_instance_data.py` — native-table producer for a supplied existing SQLite copy; canonical application is lead-owned.
- `scripts/dps_parity.py` — explicit current-tier/prior-tier profile extraction, allocated-trait JSON validation and SimC action/stat normalization.
- `scripts/simc_selected_traits.cpp` — diagnostic CLI wrapper around unchanged76f8118 engine objects; not the reference DPS executable. It replaces only the CLI entry object to inspect actual `player_traits` after one0.1s metadata iteration. Retire when the official CLI emits lossless allocated-trait metadata.
- `crates/server/src/dungeon_simulator/dps_parity/` — opt-in ignored300s native follower fixture; reuses dungeon runtime and single-follower initialization, real31146 dummy, existing diagnostic-run ownership (so normal orphan cleanup retains the follower), exact instance gear, strict native trait validation and actual combat/buff receipts. Set `DPS_PARITY_OUTPUT`, `DPS_PARITY_WORLD_DB`, `DPS_PARITY_SPEC`, `DPS_PARITY_TARGETS=1|3`. Unmodeled race0/consumables/item options and existing kit-vs-APL remain named measurement gaps; invalid traits fail instead of resetting the loadout. First execution is pending, not DPS parity proof.
- `crates/server/src/item_instance.rs` — pure instance-level stat budget resolution, consumed by production equipment Stats.
- `crates/server/src/item_instance/{bonuses,effects,load,store}.rs` — native bonus application/SQLite loading and additive GUID-keyed redb state. Type23 preserves ItemEffect IDs; base/bonus resolution retains native spell IDs, trigger types and cooldown sentinels. Equip-proc execution is not yet integrated.
- `crates/server/src/stats.rs` — production stat consumer of slot-bound instance properties.
- `crates/server/src/{inventory,inventory_model,persistence_inventory,item_stats,persistence}.rs` — owned GUID hydration/projection, initial/delta delivery, startup catalogs and additive redb initialization.

## Tests asserting this spec

- `scripts/tests/test_item_instance_data.py` — concrete signed rows, idempotence, unrelated-table retention and duplicate-ID rollback.
- `scripts/tests/test_dps_parity.py` — exact item identities/modifiers, reference-tier selection, missing-ID rejection, selected-talent metadata, direct/periodic action accounting and finite/missing/zero-reference stat comparisons.
- `crates/server/src/item_instance.rs::tests` — two passing item-level budget/weapon-damage and invalid-level regressions.
- `crates/server/src/item_instance/{bonuses,store}.rs::tests` — six native allocation/configuration/socket/crafted-stat and durable-reopen/owner-transfer/rollback regressions. Owned inventory hydration/projection is additionally proved at the persistence/Stats boundary.
- `crates/server/src/item_instance/load.rs::tests` — native SQLite configuration and enchant/gem join regressions; both passed within the 15-case library proof.
- `crates/server/src/spell_cast/linked_aura_tests.rs` — native zero/nonzero BP0, caster ownership, stacks/removal and trigger-depth controls. Nonzero parent10/8 overrides child-1/-1000 rather than rejecting the link.
- `crates/server/src/equipment_effects.rs::tests` — actual base/bonus/enchant equip-aura lifecycle and item-level replacement; enchant8039 records the resolved scaled parent/child amount, not just its zero DB2 base points.
- `crates/server/src/stats_instance_tests.rs` — actual StatsPlugin modifier-only change on native chest 271468/enchant 7987; genuine RED missed +50 Strength before consumer wiring; subsequent native GREEN and unchanged legacy Arcane control passed.

## Known gaps (current cycle)

- [ ] Remaining profile bonus options, equip-spell aura/proc lifecycle and complete instance-movement qualification; basic Stats derivation and additive persistence have targeted GREEN proof.
- [ ] Real300s server dummy execution and equal-gear parity reports.
- [ ] Shipped105/257 healer BiS/HPS references are absent upstream.

## Out of scope

Rotation/decision implementation belongs to the lead/rotation agents. No system install, native-table re-export, canonical data mutation or protocol change without demonstrated need. Reference SimC is the official local CLI-only source build at76f8118cf3f7f41dad20708e8328e1f4ab2deebc (embedded70391); server inputs remain70448. Durable evidence lives in `game-engine/data/diagnostics/dps-parity-2026-10-21/`.
