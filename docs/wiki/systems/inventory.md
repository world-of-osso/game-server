# Inventory

Requirements and data gaps: [inventory spec](../../specs/inventory.md).

Item-use scaling and regression evidence: [[item-healing-scaling]].

## Storage

`InventoryData.items` (table `inventories`) remains the owned bag-item list that trade, auction, mail and profession code edit. Two tables added beside it:

| Table | Key | Value |
|-------|-----|-------|
| `inventory_layouts` | `CharacterId` | `Vec<ItemPlacementData{item_guid, bag, slot}>` |
| `item_instances` | item guid | `ItemInstanceData{soulbound}` |

Instance state is keyed by guid, so it follows an item through trade, mail and auctions. Item rules (inventory type, `Stackable`, required level, class/race masks, bonding) come from world.db `content_item`, loaded at startup into the `ItemTemplates` resource (`item_templates.rs`). Equipped items stay in `character_equipment` (visual-slot keyed, see [[persistence]]). `persistence_inventory.rs` loads and saves all of these plus `CharacterData` in one transaction.

## Definition source

Contract: [item definition source](../../specs/item-definition-source.md). The additive global `item_definition_sources` table stores `GUID -> u8` (0 Retail, 1 Forever70205), independently of owner rows. Missing legacy markers mean Retail; invalid tags and changes to existing tags panic with the GUID. `InventoryData` and `ItemInstanceData{soulbound}` encodings are unchanged.

`load_item_definition_source(db, guid)` reads identity. `write_item_definition_source(txn, guid, source)` marks a newly minted GUID in its creation transaction, accepting repeated identical writes. Creation callers must tag Forever explicitly. `InventoryRecord.definition_sources` loads identities for bag, equipment and equipped-bag GUIDs. Record saves only add/validate markers; they never erase markers when ownership changes. The inventory model preserves the map and copies provenance for GUIDs it mints. Incoming transfer GUIDs keep their global marker even when the receiving pre-transfer record lacks that map entry; runtime/catalog and wire lookup are separate integration work.

## Model

`InventoryModel::from_record` builds a grid (backpack 16 slots, bags 1-4 size 0). Items with a valid placement keep it; unplaced items (new loot, trade, mail) take the first free slot; a guid duplicated within one character (trade/auction partial stacks keep the source guid) gets a fresh guid; items with no free slot go to `overflow` so saving never drops them. Bind-on-pickup and quest items are marked soulbound on load; bind-on-equip items when equipped. Same-item stacks merge up to `Stackable`.

Operations mutate the model; `with_appearance_sync` then rewrites the appearance entry of each equipment slot whose item changed, keeping `hidden`. `into_record` writes placements in bag order.

## Sources

- [Inventory persistence](../../../crates/server/src/persistence_inventory.rs)
- [Inventory model](../../../crates/server/src/inventory_model.rs)
- [Definition source tests](../../../crates/server/src/persistence_item_source_tests.rs)

## Networking

- `send_initial_inventory` (`Added<InWorld>`) sends both snapshots, stores the view in `SentInventoryView` on the connection, and persists new placements.
- Request systems load the model, apply the operation, persist, and call `GoldUpdateQueue::record_items`. Errors reply `InventoryError` on `InventoryChannel`.
- `GoldUpdateQueue::record` (trade, auction, barber, mail) also marks the character's inventory changed; profession craft/gather call `record_items`.
- `send_inventory_deltas` (PostUpdate, before replication send) drains the changed set, diffs the persisted view against `SentInventoryView`, sends `InventoryDelta`, and copies the persisted appearance onto the player's replicated `EquipmentAppearance` and the equipped item ids onto `EquippedItems` ([[character-stats]] gear input; `send_initial_inventory` inserts it on world entry).
