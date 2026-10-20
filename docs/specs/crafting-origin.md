# Crafting origin

Record original recipe identity on authoritative crafted item instances, using TC a352b1fa modifier39 SkillLineAbilityID semantics rather than a template-to-recipe guess. [Shared contract](../../../shared-protocol/docs/specs/crafting-origin.md) owns the wire shape.

## What it must do

- [ ] A real2153 cast produces2303 with ability1355 joined to recipe2153 on its actual final GUID; fresh DB reopen and owner inventory snapshots preserve it.
- [ ] An identical non-crafted item remains unknown; class item-creation spells are not profession recipes. Crafting quality is recorded only from actual known results, never rarity or an invented default.
- [ ] Compatible crafted stacks retain origin on their surviving GUID; unknown-origin instances cannot be relabeled by craft-output merging. Split GUIDs preserve existing origin.
- [ ] Existing inventory/character serialized records remain unchanged; provenance persists additively with inventory changes.

## How it works

- [Native crafting system](../../../game-engine/docs/wiki/systems/native-professions-crafting.md).

## Implementation inventory

- `crates/server/src/profession.rs`, `profession/data.rs` — real craft origin and authored ability join.
- `crates/server/src/persistence_inventory.rs` — GUID-keyed provenance alongside unchanged legacy records.
- `crates/server/src/inventory.rs`, `inventory_model.rs` — final GUID projection and identity-preserving storage.

## Tests asserting this spec

- `crates/server/src/profession/crafting_origin_tests.rs` — real casts, original-vs-crafted items and database reopen.
- `crates/server/src/profession/crafting_provenance_fixture.rs` — ignored private seed and post-reopen assert for the [native proof](../../../game-engine/docs/wiki/systems/native-professions-crafting.md#native-provenance-proof--2026-10-20).

## Known gaps (current cycle)

- [ ] Coordinated server/protocol/engine GREEN and integration gate.

## Out of scope

Recraft activation, operation evaluation, fabricated quality, installed reagent-slot encoding and skill watermark. Slots/watermark remain UNKNOWN and unwritten. This first slice does not claim complete positive Retail recraft eligibility.
