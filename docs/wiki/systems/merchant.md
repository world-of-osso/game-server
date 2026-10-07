# Merchant

Bot-crafted dynamic inventory is a separate, unverified integration: see [[bot-economy]] and its [contract](../../specs/bot-economy.md). Existing vendor restocking below does not establish bot stock behavior.

Vendors in `crates/server/src/merchant.rs`, with `merchant/catalog.rs` (the world.db vendor lists) and `merchant/stock.rs` (limited stock and buyback). The spec is [merchant](../../specs/merchant.md). Opening the vendor role is covered by [npc-interaction](npc-interaction.md).

## Flow

1. The interaction system inserts `ActiveInteraction { role: Some(Vendor) }` on the connection.
2. `send_vendor_lists` reacts to `Changed<ActiveInteraction>` and sends `VendorInventory` + `BuybackList`, as AzerothCore sends `SMSG_LIST_INVENTORY` right after the vendor gossip option.
3. Each request system (`process_buy/sell/buyback/repair_requests`) runs through `process`, which pairs a request with its `Customer`: connection, character, player entity, open vendor and `BuybackSlots`.
4. A refusal answers `MerchantFailed { npc, error }`. `MerchantError::message` is the Retail GlobalStrings text.

A buy with `destination` (the client's merchant cursor dropped on a bag slot) goes through `InventoryModel::store_item_at`: the slot takes what fits, the rest goes through `store_item`, and the room check covers both before anything changes. `sell_all_junk` lists the poor-quality bag stacks with a sell price from `VendorCatalog.goods`, then runs the ordinary `sell` on each, so every sale fills a buyback slot and re-sends `BuybackList`.

Money and bags change in one `InventoryModel` (load → mutate → `save_inventory_record`) and are recorded in `GoldUpdateQueue`. That queue replicates `Gold` and makes `send_inventory_deltas` send the owner an `InventoryDelta`. Equipped repair also mutates the player's `EquippedDurability`, whose change detection persists it and sends `DurabilityStateUpdate`. Per-item bag repair saves the item's durability and gold in the same inventory transaction; the existing `ItemStack.durability` in `InventoryDelta` carries the repaired state.

## Data

- `VendorCatalog.vendors`: creature entry → `VendorSlot { item_id, maxcount, incrtime }` in AzerothCore load order (`ORDER BY slot, item`, references inlined once each, `ExtendedCost = 0` only).
- `VendorCatalog.goods`: every `content_item` row: name, quality, `BuyPrice`, `SellPrice`, `VendorStackCount`, `MaxCount`.
- Stack size, bonding, class/race masks and `Flags_1` (faction flags) come from `ItemTemplates`.
- `VendorStock`: `(vendor entity, slot) → count, last increment` in server seconds. `available()` implements `GetVendorItemCurrentCount` and `consume()` implements `UpdateVendorItemCurrentCount`.
- `BuybackSlots` (a component on the connection) follows `AddItemToBuyBackSlot` / `RemoveItemFromBuyBackSlot`. Difference: AzerothCore's scan replaces the oldest sale even when an earlier slot is empty. Here an empty slot (sold-at time 0) counts as the oldest, so holes are filled first.

## Sources

- [Merchant contract](../../specs/merchant.md)
- [Request handlers](../../../crates/server/src/merchant.rs)
- [Buyback ordering](../../../crates/server/src/merchant/stock.rs)
- [Item storage](../../../crates/server/src/persistence_inventory.rs)

## Durability and repair cost

`durability.rs` loads `ItemDurability` at startup from world.db:

- `max` = AzerothCore `item_template.MaxDurability` via `content_item.SourceEntry`.
- `cost_per_point` = Retail `durability_costs` row `ItemLevel`, column `ItemSubClassToDurabilityMultiplierId`, times `durability_quality` row `(Quality+1)*2`.
  - The column is the subclass for weapons, `21+subclass` for armor, and 0 otherwise.
  - The Retail item level and quality come from `content_item`.

Example: Tarnished Chain Vest 2379 (Retail item level 2, quality 1, mail) has multiplier 1 × 0.8, max 55. Losing 11 points costs 8 copper.

Players get `EquippedDurability` through an entity command at spawn (`insert_equipped_durability`), because the character-select pipeline has no resource access. `sync_durability_from_equipment_appearance` restores the saved current value. Equipment moves preserve `(current, max)` by item guid in the separate `item_durabilities` redb table, without changing existing item-instance rows; bag snapshots expose that state and re-equipping restores it. `BuybackList` uses sale order (oldest first, newest last), retaining each entry's physical request slot and same-tick request order.
