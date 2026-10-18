# Inventory and equipment networking

Owner-only bag and equipment state for the in-game bag and character frames (game-engine in-game UI plan, phase 4). Contract: `shared::protocol` `inventory_messages.rs` (shared-protocol). Server: `crates/server/src/inventory.rs`, `inventory_model.rs`, `persistence_inventory.rs`, `item_templates.rs` (world.db `content_item`). See [inventory system](../wiki/systems/inventory.md) for the mechanism.

## What it must do

- [x] Source item-template `MaxCount` limits ownership independently of `Stackable` and unique-equipped limits. Non-positive values are unlimited. All shared storage entry points reject an excess with `InventoryErrorReason::ItemMaxCount`, preserving the original GUID/quantity and leaving the inventory unchanged; same-GUID owned moves exclude that moving instance. Count same-source equipment, bags, overflow and personal bank, including while the bank is closed. Read a passive ownership snapshot without granting bank-location access; account/guild banks are not personal ownership. Existing vendor/loot/charter prechecks use the same bank-aware owned count. Source: pinned TC `Player.cpp:9490–9512,9928–9954`, `SpellEffects.cpp:1426–1440`. Runtime/consumer verification is recorded separately from this contract.

- [x] On enter world send the owner `InventorySnapshot` (containers 0-4 with size and occupied slots: item id, count, durability, soulbound, instance guid) and `EquipmentSnapshot` (occupied Retail equipment slots).
- [x] After every persisted change send the owner an `InventoryDelta` listing each changed bag or equipment location.
- [x] `SwapItem{from, to}` moves, equips, unequips or swaps by slot validity; `EquipItem{from}` auto-equips into the first allowed slot (or unequips into the first free bag slot).
- [x] `SplitItem{from, to, count}` splits a bag or character-bank stack into an empty bag or character-bank slot with a new instance guid, or onto a same-item stack that has room. Splits preserve binding and total count.
- [x] `SwapItem` / `SplitItem` address character-bank cells as `ItemLocation::Bank { tab, slot }`: bank↔bags and same/across-tab bank↔bank moves share stack limits and container restrictions. An open banker is required (`CantDoRightNow` otherwise); unbought tabs/invalid bank cells return `WrongSlot`. Bank and inventory changes persist atomically, and the owner receives refreshed `BankContents` plus changed bag/equipment state. See [banks](banks.md).
- [x] Swapping a stack onto a same-item stack merges up to the `content_item.Stackable` limit.
- [x] Equip checks `content_item` inventory type, required level and class/race masks (`CANT_EQUIP_EVER`); bind-on-pickup/quest items are soulbound when owned, bind-on-equip items when equipped.
- [x] `DestroyItem{location, count}` removes `count` items; 0 or at least the stack size removes the stack. Persisted.
- [x] `UseItem` replies `EQUIP_ERR_CANT_DO_RIGHT_NOW` with a detail: the server has no item spell data.
- [x] `SortBags` compacts bag items by quality, item id, stack size.
- [x] Rejections reply `InventoryError{reason, detail}` with Retail `EQUIP_ERR_*` names (`InventoryErrorReason::message()` gives the client's `ERR_*` text; `CantEquipLevelI` carries the required level): invalid bag slot (`INTERNAL_BAG_ERROR`), empty source (`ITEM_NOT_FOUND`), wrong slot (`WRONG_SLOT`), unknown/non-equip type (`NOT_EQUIPPABLE`), level (`CANT_EQUIP_LEVEL_I`), class/race (`CANT_EQUIP_EVER`), one-hander to off hand (`CANT_DUAL_WIELD`), two-hand conflict with full bags (`INV_FULL`), bad split count (`TOO_FEW_TO_SPLIT`), split involving a non-storage location such as equipment (`SPLIT_FAILED`), split onto a different item or a full stack (`CANT_STACK`).
- [x] Two-handers displace the off hand (and off-hand items displace an equipped two-hander) into a free bag slot, unless native learned Titan's Grip permits that two-hander to occupy one hand. Two-hand weapons (inventory type17) need both Dual Wield and Titan's Grip for off-hand placement; subclass restrictions come from the learned effect155 spell. Ranged inventory types do not gain an off-hand slot. Automatic off-hand removal on unlearning remains unimplemented.
- [x] Equipment changes update the persisted and replicated `EquipmentAppearance` (hidden flag kept) and physical `character_equipment` ownership.
- [x] Trade completion, auction/mail claims and profession craft/gather results emit deltas.
- [x] Equipped item stats and tooltip values use one pure `shared::item_scaling` implementation: newest ItemSquishEra curve, RandPropPoints budget, socket cost, stamina/rating multipliers, armor and rounded weapon damage. Server loads source-local world.db DB2 tables and generated local-CASC game tables; client loads corresponding local CSV/game-table exports. No protocol message change or server formula change. Forest Chain (1273) contributes +4 Stamina and 10 Armor; cross-repo regressions also cover Pioneer Tunic (6268) and Black Duskwood Staff (937).

## Tests asserting this spec

- `crates/server/src/inventory_tests.rs` — enter-world snapshots, swap, main-hand equip + appearance, wrong-type equip, sword onto the head slot (`WRONG_SLOT` text), main-hand sword unequipped onto an empty bag slot, class-restricted equip + bind-on-pickup, split 20 into 15+5, split stack merged back, destroy persistence, invalid slot, use-item error, trade-completion deltas.
- `crates/server/src/bank_move_tests.rs`, `bank_move_rule_tests.rs` — exact bank/bag moves, swaps, capped merges, partial splits, closed/unbought/oversized rejection, specialty/nonempty-container rejection and bound GUID/count persistence after database reopen. `a_move_request_after_closing_the_bank_rejects_without_changing_inventory` sends `SwapItem` on `InventoryChannel` after `CloseInteraction`, expects `CantDoRightNow`, and compares the complete persisted inventory and character bank with their pre-request values; an open-bank positive control proves the same slot/request is valid.
- `crates/server/src/item_templates_tests.rs` — real world.db `content_item` rows (skips without the DB).
- shared-protocol `src/protocol_inventory_tests.rs` — wire round trips, Retail slot ids, inventory-type slot map, registration.

## Known gaps (current cycle)

- [ ] Bags 1-4 always report size 0 (containers are not equippable yet; `item_sparse.ContainerSlots` has the sizes); items beyond 16 backpack slots stay owned but are not addressable.
- [ ] Dual wield and weapon/armor skills are not tracked: one-handers equip to the main hand only.
- [ ] Items arriving by trade, mail or loot take a free slot rather than stacking onto existing stacks; soulbound items are not blocked from trade.
- [x] Neck, both fingers, both trinkets and the legacy relic inventory slot persist beside visual equipment without changing existing character encodings. Equipment snapshots/deltas and physical stat inputs include them. Ranged weapons retain Retail Mainline's main-hand mapping; Mainline exposes no ranged paperdoll button.
- [x] ItemSparse.Flags_0 `ITEM_FLAG_UNIQUE_EQUIPPABLE` rejects a second same-source item with `ItemUniqueEquippable`, excluding the replaced slot and the moving GUID. Paired slots support explicit swaps and first-empty auto-equip; when both are occupied, automatic equip replaces the valid lower ItemLevel occupant (TrinityCore a352b1fa Player::FindEquipSlot). ItemSparse.LimitCategory applies the authenticated Retail ItemLimitCategory.Quantity across distinct same-source definitions; replacement and paired movement exclude displaced slots/moving GUIDs. Existing catalog has no non-visual category using ItemLimitCategoryCondition. ItemEffect on-equip triggers remain outside the existing item-stat system; existing StatModifiers continue to apply to these gear stats.
- [ ] Bag-item durability is not tracked (`None`); equipped durability is the saved per-item state (max from `item_template.MaxDurability`, see [merchant](merchant.md)).
- [ ] Items offered in an open trade are not locked; trade completion re-validates them.

## Out of scope

Warband/guild-bank exact-slot moves ([banks](banks.md)), reagent bank, bag containers, item use spells, vendor buy/sell ([merchant](merchant.md)), loot.
