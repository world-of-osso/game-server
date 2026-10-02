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
  - `guild_repair_money` (at a repairer) is the guild money the player may spend on repairs now (TrinityCore `GetMemberAvailableMoneyForRepairItems`, Guild.cpp:3040-3047: the guild money capped by the rank's daily allowance left; unlimited for the Guild Master). `None` outside a guild or for a rank without `GR_RIGHT_WITHDRAW_REPAIR` (Retail `CanGuildBankRepair`).
  - `max_durability` is the item's `MaxDurability` (`None` without durability), for the Retail merchant tooltip's `DURABILITY_TEMPLATE`.
- [x] Every request must name the NPC whose vendor frame is open, otherwise it gets `MerchantFailed(NotInteracting)`. Range, combat and death are handled by `npc_interaction`, which closes the frame.
- [x] Buy: `count` purchases cost `BuyPrice * count`.
  - Refused with `ItemNotFound` when the slot does not hold that item or the item is filtered out.
  - Refused with `SoldOut`, `NotEnoughMoney`, `CantCarryMore` (`MaxCount`) or `InventoryFull`.
  - A refused buy changes nothing.
  - Items first top up existing stacks, then fill empty backpack slots.
  - [x] `BuyItem.destination: Some(bag slot)` (a merchant cursor dropped on that slot; TrinityCore `HandleBuyItemOpcode` ContainerGUID + Slot, `CanStoreNewItem(bag, slot)`): the purchase goes there when the slot is empty or holds the same item with room; what does not fit there is stored as above. Another item or a full stack there refuses with `CantStack` ("This item cannot stack."). An equipment destination is refused (`InventoryFull`; TrinityCore equips single items bought onto an equipment slot, the client never sends one).
- [x] Limited stock (AzerothCore `VendorItemCount`) is per vendor creature and slot. Each `incrtime` seconds after the last purchase it restocks one purchase, capped at `maxcount`. A limited buy re-sends `VendorInventory`.
- [x] Sell: `SellItem { item_guid, count }`, where count 0 means the whole stack.
  - Pays `SellPrice * count`.
  - Refused with `NotInterested` when `SellPrice` is 0 or `count` is more than the stack. Refused with `TooMuchGold` at the money cap (`0x7FFFFFFE`).
  - A partial sale splits the stack. The sold part gets a new guid.
- [x] Sell All Junk (`SellAllJunkItems { npc }`, TrinityCore `HandleSellAllJunkItems`, ItemHandler.cpp:427-484): every bag stack of `content_item.Quality` 0 (poor) with a `SellPrice` is sold whole in bag order, each into a buyback slot. The first refused sale (money cap) ends it silently, as TrinityCore discards the error; a closed vendor answers `NotInteracting`.
- [x] Buyback: 12 slots per session in the world, not persisted.
  - A sale goes into the current slot, or replaces the oldest sale when that slot is taken.
  - Buying back costs what the sale paid, restores the item (stacking) and frees the slot.
  - Refused with `ItemNotFound` (empty slot), `NotEnoughMoney` or `InventoryFull`.
  - `BuybackList` (filled slots in slot order) is sent after every change.
- [x] Repair (`RepairItem { item_guid }`, where `None` means all equipped items) works only at a `REPAIR` NPC.
  - Each item costs `lost * DurabilityCosts[ItemLevel][class/subclass] * DurabilityQuality[(Quality+1)*2]`, at least 1 copper.
  - Items the remaining money can't cover stay damaged and the request answers `NotEnoughMoney`.
  - `Some(guid)` repairs only that equipped item. Any other guid (a bag item, which has no durability) changes nothing and answers nothing, as TrinityCore `HandleRepairItemOpcode` (NPCHandler.cpp:748).
- [x] Guild bank repair (`RepairItem { item_guid: None, guild_bank: true }`, Retail `RepairAllItems(true)`; TrinityCore `HandleRepairItemOpcode` `UseGuildBank`, NPCHandler.cpp:389-420, applies it to repair-all only, so `Some(guid)` with the flag pays personally):
  - The player's rank needs `GR_RIGHT_WITHDRAW_REPAIR`, else `GuildPermissions` ("You don't have permission to do that."); a player without a guild gets the same.
  - Damaged items are repaired cheapest first while their running total fits `guild_repair_money`; the total is taken from the guild money once (TrinityCore `Player::DurabilityRepairAll(guildBank = true)`, Player.cpp:4636-4669). The player's own money is never touched.
  - The repair counts against the rank's daily allowance (`gold_per_day`), the same allowance gold withdrawals use (`_GetMemberRemainingMoney`, Guild.cpp:3358-3371), and writes a `RepairMoney` entry to the guild bank money log (`GUILD_BANK_LOG_REPAIR_MONEY`, `HandleMemberWithdrawMoney(repair = true)`, Guild.cpp:2080-2124).
  - Items left damaged answer `GuildNotEnoughMoney` ("The guild bank does not have enough money"); when nothing fits, nothing changes and nothing is logged. TrinityCore answers neither error; the personal repair's `NotEnoughMoney` is the precedent.
  - After a guild repair the vendor list is sent again with the new `guild_repair_money`, and guild members with the vault open get the new bank.
- [x] Every decoded vendor request is logged once (`merchant request character=<id> <request>`).
- [x] Durability belongs to equipped items with `item_template.MaxDurability > 0`, matched by `content_item.SourceEntry`. A newly equipped item starts at full durability.
- [x] Changing from the old per-slot profile: at login a saved current value is capped to the item's max, an equipped item without `MaxDurability` (e.g. Recruit's Boots 40) loses its durability slot, and a newly equipped item starts at full durability instead of taking over the slot's value.
- [x] Admin `set-gold <character> <copper>` sets the money and replicates it live. `set-durability <character> <percent>` sets every equipped item to that percent of its max.
- [ ] No reputation price discount (`GetReputationPriceDiscount`): NPC factions are not mapped to reputation factions.
- [ ] Items with a Wrath `ExtendedCost` (12,887 rows) are skipped, because Retail `ItemExtendedCost` is not imported.
- [ ] No vendor conditions, no refundable purchases; no per-bag junk-sell exclusion flags (`BagSlotFlags::ExcludeJunkSell`).
- [ ] Bag items have no durability, so selling damaged items and repairing bag items do not apply.

## Tests asserting this spec

- `crates/server/src/merchant_tests.rs`: real world.db vendors.
  - Godric Rothgar 1213: armor list and prices, buy the vest for 89 copper, refusals, a purchase that doesn't fit, repair all for 16 copper, partial repair, repairing only the equipped vest for 8 copper, a bag item's guid ignored.
  - Brother Danil 152: water at 5 per purchase stacking to 20, no sell price, no repair.
  - Tharynn Bouden 66: the limited pattern sells out.
  - Also: linen sale and buyback, restock timing, 13th sale, admin commands.
  - Buy into a slot: the vest onto backpack slot 10; water stacking onto its slot and refused on the linen slot; 17 water + 5 fills to 20 and stores 2, then a full stack refuses.
  - Sell All Junk: 2 Ruined Pelt, 1 Broken Fang, 3 Chipped Claw pay 28 copper into 3 buyback slots; refused with the vendor closed.
- `crates/server/src/merchant_guild_repair_tests.rs`: guild repair at Godric Rothgar: the Guild Master's 16 copper from a 1000-copper bank, logged; a Member without the right and a guildless player refused; a 10-copper allowance repairs the vest only, then 2 copper left covers nothing; a 5-copper bank refuses; withdrawals and repairs share one allowance; repairing one item with the flag pays personally.
- `crates/server/src/durability.rs` tests: MaxDurability per item, cost formula, world.db DB2 costs.
- `crates/server/src/bin/admin.rs` tests: set-gold / set-durability parsing.
