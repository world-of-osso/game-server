# Crafting origin

Record original recipe identity on authoritative crafted item instances, using TC a352b1fa modifier39 SkillLineAbilityID semantics rather than a template-to-recipe guess. [Shared contract](../../../shared-protocol/docs/specs/crafting-origin.md) owns the wire shape.

## What it must do

- [ ] A real2153 cast produces2303 with ability1355 joined to recipe2153 on its actual final GUID; fresh DB reopen and owner inventory snapshots preserve it.
- [ ] An identical non-crafted item remains unknown; class item-creation spells are not profession recipes. Crafting quality is recorded only from actual known results, never rarity or an invented default.
- [ ] CRAFTING_ORIGIN_NOT_RECORDED_ON_STACKABLE_ITEMS: origin is recorded only on items whose template max stack is 1. Stackable craft output (e.g. 2881 Light Leather) carries no origin and merges with looted stacks of the same item, matching Retail commodities, which carry no crafter identity.
- [ ] Cleanup: `item_crafting_origins` rows written before this policy on stackable items are dropped at load: inventory, bank, guild bank and mail never project them, so those stacks merge normally. The rows stay in the table, inert (no write path re-reads them).
- [ ] Existing inventory/character serialized records remain unchanged; provenance persists additively with inventory changes.

## How it works

- [Native crafting system](../../../game-engine/docs/wiki/systems/native-professions-crafting.md).

## Implementation inventory

- `crates/server/src/profession.rs`, `profession/data.rs` — real craft origin and authored ability join.
- `crates/server/src/persistence_inventory.rs` — GUID-keyed provenance alongside unchanged legacy records.
- `crates/server/src/inventory.rs`, `inventory_model.rs` — final GUID projection and identity-preserving storage.

## Tests asserting this spec

- `crates/server/src/profession/crafting_origin_tests.rs` — real casts, original-vs-crafted items, database reopen, stackable output merging with looted stacks and legacy stackable-row drop.

## Known gaps (current cycle)

- [ ] Coordinated server/protocol/engine GREEN and integration gate.

## Out of scope

Recraft activation, operation evaluation, fabricated quality, installed reagent-slot encoding and skill watermark. Slots/watermark remain UNKNOWN and unwritten. This first slice does not claim complete positive Retail recraft eligibility.
