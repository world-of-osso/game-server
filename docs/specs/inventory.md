# Inventory and equipment networking

Owner-only bag and equipment state for the in-game bag and character frames (game-engine in-game UI plan, phase 4). Contract: `shared::protocol` `inventory_messages.rs` (shared-protocol). Server: `crates/server/src/inventory.rs`, `inventory_model.rs`, `persistence_inventory.rs`. See [inventory system](../wiki/systems/inventory.md) for the mechanism.

## What it must do

- [x] On enter world send the owner `InventorySnapshot` (containers 0-4 with size and occupied slots: item id, count, durability, soulbound, instance guid) and `EquipmentSnapshot` (occupied Retail equipment slots).
- [x] After every persisted change send the owner an `InventoryDelta` listing each changed bag or equipment location.
- [x] `SwapItem{from, to}` moves, equips, unequips or swaps by slot validity; `EquipItem{from}` auto-equips into the first allowed slot (or unequips into the first free bag slot).
- [x] `SplitItem{from, to, count}` splits a bag stack into an empty bag slot with a new instance guid.
- [x] `DestroyItem{location, count}` removes `count` items; 0 or at least the stack size removes the stack. Persisted.
- [x] `UseItem` replies `EQUIP_ERR_CANT_DO_RIGHT_NOW` with a detail: the server has no item spell data.
- [x] `SortBags` compacts bag items by quality, item id, stack size.
- [x] Rejections reply `InventoryError{reason, detail}` with Retail `EQUIP_ERR_*` names: invalid bag slot (`INTERNAL_BAG_ERROR`), empty source (`ITEM_NOT_FOUND`), wrong slot (`WRONG_SLOT`), unknown/non-equip type (`NOT_EQUIPPABLE`), level (`CANT_EQUIP_LEVEL_I`), one-hander to off hand (`CANT_DUAL_WIELD`), two-hand conflict with full bags (`INV_FULL`), bad split count (`TOO_FEW_TO_SPLIT`), non-bag split (`SPLIT_FAILED`), split onto an occupied slot (`CANT_STACK`).
- [x] Two-handers displace the off hand (and off-hand items displace an equipped two-hander) into a free bag slot.
- [x] Equipment changes update the persisted and replicated `EquipmentAppearance` (hidden flag kept) and physical `character_equipment` ownership.
- [x] Trade completion, auction/mail claims and profession craft/gather results emit deltas.

## Tests asserting this spec

- `crates/server/src/inventory_tests.rs` — enter-world snapshots, swap, main-hand equip + appearance, wrong-type equip, split 20 into 15+5, destroy persistence, invalid slot, use-item error, trade-completion deltas.
- shared-protocol `src/protocol_inventory_tests.rs` — wire round trips, Retail slot ids, inventory-type slot map, registration.

## Known gaps (current cycle)

- [ ] No item template data on the server: inventory type is known only for admin-granted equipment (from its appearance entry, then `item_instances`); other items are not equippable.
- [ ] Stack limits unknown: same-item stacks swap instead of merging; splits need an empty target.
- [ ] Bags 1-4 always report size 0 (no container data); items beyond 16 backpack slots stay owned but are not addressable.
- [ ] Dual wield, class/race proficiency and binding types are not stored; `soulbound` is always false.
- [ ] Neck, finger, trinket and ranged slots have no server storage (`WRONG_SLOT`).
- [ ] Bag-item durability is not tracked (`None`); equipped durability comes from the per-slot durability state.
- [ ] Items offered in an open trade are not locked; trade completion re-validates them.
- [ ] Equipped items do not feed `stats::EquippedItems`: the server has no item stats (phase 4 blocker).

## Out of scope

Bank, reagent bank, bag containers, item use spells, vendor buy/sell, loot.
