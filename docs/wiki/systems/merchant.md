# Merchant

Vendors in `crates/server/src/merchant.rs`, with `merchant/catalog.rs` (the world.db vendor lists) and `merchant/stock.rs` (limited stock and buyback). The spec is [merchant](../../specs/merchant.md). Opening the vendor role is covered by [npc-interaction](npc-interaction.md).

## Flow

1. The interaction system inserts `ActiveInteraction { role: Some(Vendor) }` on the connection.
2. `send_vendor_lists` reacts to `Changed<ActiveInteraction>` and sends `VendorInventory` + `BuybackList`, as AzerothCore sends `SMSG_LIST_INVENTORY` right after the vendor gossip option.
3. Each request system (`process_buy/sell/buyback/repair_requests`) runs through `process`, which pairs a request with its `Customer`: connection, character, player entity, open vendor and `BuybackSlots`.
4. A refusal answers `MerchantFailed { npc, error }`. `MerchantError::message` is the Retail GlobalStrings text.

Money and bags change in one `InventoryModel` (load → mutate → `save_inventory_record`) and are recorded in `GoldUpdateQueue`. That queue replicates `Gold` and makes `send_inventory_deltas` send the owner an `InventoryDelta`. Repair also mutates the player's `EquippedDurability`, whose change detection persists it and sends `DurabilityStateUpdate`.

## Data

- `VendorCatalog.vendors`: creature entry → `VendorSlot { item_id, maxcount, incrtime }` in AzerothCore load order (`ORDER BY slot, item`, references inlined once each, `ExtendedCost = 0` only).
- `VendorCatalog.goods`: every `content_item` row: name, quality, `BuyPrice`, `SellPrice`, `VendorStackCount`, `MaxCount`.
- Stack size, bonding, class/race masks and `Flags_1` (faction flags) come from `ItemTemplates`.
- `VendorStock`: `(vendor entity, slot) → count, last increment` in server seconds. `available()` implements `GetVendorItemCurrentCount` and `consume()` implements `UpdateVendorItemCurrentCount`.
- `BuybackSlots` (a component on the connection) follows `AddItemToBuyBackSlot` / `RemoveItemFromBuyBackSlot`. Difference: AzerothCore's scan replaces the oldest sale even when an earlier slot is empty. Here an empty slot (sold-at time 0) counts as the oldest, so holes are filled first.

## Durability and repair cost

`durability.rs` loads `ItemDurability` at startup from world.db:

- `max` = AzerothCore `item_template.MaxDurability` via `content_item.SourceEntry`.
- `cost_per_point` = Retail `durability_costs` row `ItemLevel`, column `ItemSubClassToDurabilityMultiplierId`, times `durability_quality` row `(Quality+1)*2`.
  - The column is the subclass for weapons, `21+subclass` for armor, and 0 otherwise.
  - The Retail item level and quality come from `content_item`.

Example: Tarnished Chain Vest 2379 (Retail item level 2, quality 1, mail) has multiplier 1 × 0.8, max 55. Losing 11 points costs 8 copper.

Players get `EquippedDurability` through an entity command at spawn (`insert_equipped_durability`), because the character-select pipeline has no resource access. `sync_durability_from_equipment_appearance` keeps a slot's value while the same item stays equipped.
