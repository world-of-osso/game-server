# Native item-instance DPS parity

Compare native70448 server combat with shipped MID1 SimulationCraft76f8118 profiles. Sources: `scripts/dps_parity.py`, `scripts/item_instance_data.py`, `crates/server/src/item_instance.rs`; [character stats](../wiki/systems/character-stats.md) describes the existing production stat path.

## What it must do

### Equipment

- [ ] Instances retain bonus lists, an optional explicit item level, permanent enchant and gem item IDs by GUID, with backward-compatible redb storage. Crafted stat selections survive where the profile specifies them.
- [ ] Equipment stats resolve native70448 ItemBonus, ItemSparse, RandPropPoints and Midnight scaling/configuration tables, including stat allocation, sockets and enchant/gem effects. Unsupported effects are named gaps, never invented stats.
- [x] The native-table producer preserves signed values, replaces only the requested table, supports repeated imports and rolls back a duplicate-ID failure without losing the prior table.
- [x] Shipped profile manifests preserve exact item IDs, bonus lists, item-level overrides, enchants, gems, crafted stats, talents, consumables and action text. Missing item IDs fail explicitly.

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
- `scripts/dps_parity.py` — shipped MID1 profile extraction and SimC action normalization.
- `crates/server/src/item_instance.rs` — pure instance-level stat budget resolution; production equipment integration remains pending.
- `crates/server/src/item_instance/{bonuses,load,store}.rs` — native bonus application/SQLite loading and additive GUID-keyed redb state.

## Tests asserting this spec

- `scripts/tests/test_item_instance_data.py` — concrete signed rows, idempotence, unrelated-table retention and duplicate-ID rollback.
- `scripts/tests/test_dps_parity.py` — exact item identities/modifiers, missing-ID rejection and direct/periodic action accounting.
- `crates/server/src/item_instance.rs::tests` — two passing item-level budget/weapon-damage and invalid-level regressions.
- `crates/server/src/item_instance/{bonuses,store}.rs::tests` — six passing native allocation/configuration/socket/crafted-stat and durable-reopen/owner-transfer/rollback regressions; runtime wiring remains pending.
- `crates/server/src/item_instance/load.rs::tests` — native SQLite configuration regression; genuine RED recorded before loader implementation, GREEN pending.

## Known gaps (current cycle)

- [ ] Runtime instance bonus/enchant/gem derivation and backward-compatible persistence.
- [ ] Real300s server dummy execution and equal-gear parity reports.
- [ ] Shipped105/257 healer BiS/HPS references are absent upstream.

## Out of scope

Rotation/decision implementation belongs to the lead/rotation agents. No system install, native-table re-export, canonical data mutation or protocol change without demonstrated need. Reference SimC is the official local CLI-only source build at76f8118cf3f7f41dad20708e8328e1f4ab2deebc (embedded70391); server inputs remain70448. Durable evidence lives in `game-engine/data/diagnostics/dps-parity-2026-10-21/`.
