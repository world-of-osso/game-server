# Merchant

Server-side vendors: the list, buy, sell, buyback and repair, following AzerothCore rules. The contract is shared-protocol `protocol/merchant_messages.rs` (`MerchantChannel`). The client frame is game-engine `docs/specs/merchant-frame.md`. For how it works, see [merchant](../wiki/systems/merchant.md).

## What it must do

- [x] Opening the vendor role (`ActiveInteraction.role == Vendor`, either directly or from the gossip vendor option) sends `VendorInventory`, then `BuybackList`.
  - The list comes from `content_npc_vendor` in `slot, item` order. Negative `item` rows inline the referenced vendor.
  - Rows with a `maxcount` but no `incrtime`, or the reverse, are ignored. At most 150 items.
  - Items are hidden when they are bind-on-pickup and the player's class can't use them, when they carry the other faction's `ITEM_FLAG2_FACTION_*` flag, or when they are sold out.
  - Each item keeps its catalog slot index.
  - Prices come from `content_item.BuyPrice`, with one purchase = `VendorStackCount` items.
  - `usable` means the class, race and level requirements are met.
  - `can_repair` is set from `NPCFlags::REPAIR`.
- [x] Every request must name the NPC whose vendor frame is open, otherwise it gets `MerchantFailed(NotInteracting)`. Range, combat and death are handled by `npc_interaction`, which closes the frame.
- [x] Buy: `count` purchases cost `BuyPrice * count`.
  - Refused with `ItemNotFound` when the slot does not hold that item or the item is filtered out.
  - Refused with `SoldOut`, `NotEnoughMoney`, `CantCarryMore` (`MaxCount`) or `InventoryFull`.
  - A refused buy changes nothing.
  - Items first top up existing stacks, then fill empty backpack slots.
- [x] Limited stock (AzerothCore `VendorItemCount`) is per vendor creature and slot. Each `incrtime` seconds after the last purchase it restocks one purchase, capped at `maxcount`. A limited buy re-sends `VendorInventory`.
- [x] Sell: `SellItem { item_guid, count }`, where count 0 means the whole stack.
  - Pays `SellPrice * count`.
  - Refused with `NotInterested` when `SellPrice` is 0 or `count` is more than the stack. Refused with `TooMuchGold` at the money cap (`0x7FFFFFFE`).
  - A partial sale splits the stack. The sold part gets a new guid.
- [x] Buyback: 12 slots per session in the world, not persisted.
  - A sale goes into the current slot, or replaces the oldest sale when that slot is taken.
  - Buying back costs what the sale paid, restores the item (stacking) and frees the slot.
  - Refused with `ItemNotFound` (empty slot), `NotEnoughMoney` or `InventoryFull`.
  - `BuybackList` (filled slots in slot order) is sent after every change.
- [x] Repair (`RepairItem { item_guid }`, where `None` means all equipped items) works only at a `REPAIR` NPC.
  - Each item costs `lost * DurabilityCosts[ItemLevel][class/subclass] * DurabilityQuality[(Quality+1)*2]`, at least 1 copper.
  - Items the remaining money can't cover stay damaged and the request answers `NotEnoughMoney`.
- [x] Durability belongs to equipped items with `item_template.MaxDurability > 0`, matched by `content_item.SourceEntry`. A newly equipped item starts at full durability.
- [x] Changing from the old per-slot profile: at login a saved current value is capped to the item's max, an equipped item without `MaxDurability` (e.g. Recruit's Boots 40) loses its durability slot, and a newly equipped item starts at full durability instead of taking over the slot's value.
- [x] Admin `set-gold <character> <copper>` sets the money and replicates it live. `set-durability <character> <percent>` sets every equipped item to that percent of its max.
- [ ] No reputation price discount (`GetReputationPriceDiscount`): NPC factions are not mapped to reputation factions.
- [ ] Items with a Wrath `ExtendedCost` (12,887 rows) are skipped, because Retail `ItemExtendedCost` is not imported.
- [ ] No vendor conditions, no refundable purchases, no guild-bank repair, and no Sell All Junk.
- [ ] Bag items have no durability, so selling damaged items and repairing bag items do not apply.

## Tests asserting this spec

- `crates/server/src/merchant_tests.rs`: real world.db vendors.
  - Godric Rothgar 1213: armor list and prices, buy the vest for 89 copper, refusals, a purchase that doesn't fit, repair all for 16 copper, partial repair.
  - Brother Danil 152: water at 5 per purchase stacking to 20, no sell price, no repair.
  - Tharynn Bouden 66: the limited pattern sells out.
  - Also: linen sale and buyback, restock timing, 13th sale, admin commands.
- `crates/server/src/durability.rs` tests: MaxDurability per item, cost formula, world.db DB2 costs.
- `crates/server/src/bin/admin.rs` tests: set-gold / set-durability parsing.
