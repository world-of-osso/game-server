# Item definition source persistence

Item definition identity belongs to the item GUID, not its owner or race. Server persistence uses `shared::item_data::ItemDefinitionSource` without changing the serialized inventory or soulbound-instance layouts. Mechanism: [inventory storage](../wiki/systems/inventory.md#definition-source).

## What it must do

- [ ] Distinguish Retail and Forever70205 GUIDs sharing item ID 2947, including database close/reopen.
- [ ] Preserve both identities when two owners exchange those GUIDs through the existing inventory-record write transaction.
- [ ] Interpret absent legacy markers as Retail by representation, without querying another catalog.
- [ ] Reject malformed markers and attempts to change an existing GUID's source explicitly.
- [ ] Preserve source metadata through inventory-model record conversion and copy it when the model mints a duplicate or split GUID.

## How it works

- [Inventory definition-source storage and API](../wiki/systems/inventory.md#definition-source).

## Implementation inventory

- `crates/server/src/persistence_inventory.rs` — GUID marker helpers and owned-record source map.
- `crates/server/src/persistence.rs` — additive table initialization.
- `crates/server/src/inventory_model.rs` — record preservation and minted-GUID provenance.

## Tests asserting this spec

- `crates/server/src/persistence_item_source_tests.rs` — concrete redb reopen, exchange, legacy and invalid-marker cases.

## Known gaps (current cycle)

- [ ] Creation callers must explicitly mark new Forever GUIDs in their creation transaction.
- [ ] Runtime lookup, mixed-source stack selection and wire population are separate integration slices.

## Out of scope

Catalog/cache selection, protocol edits, live database migrations, serialized-layout changes and race-based transfer inference.
