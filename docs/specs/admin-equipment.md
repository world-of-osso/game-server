# Administrative physical equipment

The local admin API can grant and equip trusted item definitions without placing equipped items in tradable bags. Source: `crates/server/src/persistence_equipment.rs` and the existing admin IPC/CLI. See [persistence](../wiki/systems/persistence.md#physical-equipment) for storage details.

## What it must do

- [x] `grant-equipment <character> <manifest.json>` accepts a JSON array of item metadata and atomically grants one owned item instance per supported slot.
- [x] Reject unknown characters, incompatible level/class/race, unsupported inventory types, duplicate items/slots, and occupied slots without modifying items, appearances, bags, gold, or the item-instance counter.
- [x] Allocate globally unique item GUIDs from the existing item-instance counter and persist slot ownership independently of the CharacterData encoding.
- [x] Derive item-based equipment appearances, preserve unrelated gear/overrides, publish live appearances, and retain ownership across normal saves and database reopen.
- [ ] Publish character-list updates through the existing roster channel (wired; transport proof pending).
- [ ] `list-equipment <character>` reports slot, GUID, item ID, and stored item metadata without writes.

## How it works

- [Physical equipment persistence](../wiki/systems/persistence.md#physical-equipment)

## Implementation inventory

- `equipment_admin_types.rs` — shared trusted metadata and ownership response records.
- `persistence_equipment.rs` — atomic validation, GUID allocation, ownership and appearance transaction.
- `admin_types.rs`, `admin_handlers.rs`, `admin_ipc.rs` — server request handling and live publication.
- `bin/admin.rs`, `admin_output.rs` — manifest input, grant/list commands and output.

Paths above are relative to `crates/server/src/`.

## Tests asserting this spec

- `crates/server/src/persistence_equipment_tests.rs` — file-backed transaction, reopen, rollback and inventory preservation.
- `crates/server/src/admin_ipc.rs` — live equipment appearance update and rejected-grant no-op.
- `crates/server/src/bin/admin.rs` — typed manifest and CLI parsing.

## Known gaps (current cycle)

- [ ] Main integration owns live grants and runtime appearance verification; this implementation does not mutate the running server.

## Out of scope

Combat damage, armor/stat calculations, durability synthesis, inventory unequip/move operations, catalog import, destructive slot replacement, and CharacterData format migration. The selected starter items have no bonus-stat claim; actual ownership and visual equipment do not imply combat-system integration.
