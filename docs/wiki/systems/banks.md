# Banks

How the server implements the character bank, the Warband bank and the guild bank. The contract is in [banks spec](../../specs/banks.md).

## Storage (redb, bitcode)
Every table is defined in `persistence_banks.rs`.

| Table | Key → value |
|---|---|
| `character_banks` | character_id → `BankData { tabs, money }` |
| `account_banks` | account_id → `BankData { tabs, money }` |
| `guilds` | guild_id → `GuildData { ranks, money, tabs }` |
| `guild_member_ranks` | character_id → rank |
| `guild_bank_usage` | character_id → `GuildBankUsage { guild_id, day, tab_withdrawals, copper_withdrawn }` |
| `guild_bank_logs` | guild_id → `GuildBankLogs` |

- A bank tab is `BankTabData { name, icon, deposit_flags, text, slots: [Option<InventoryItemData>; 98] }`.
- Items keep their guid when they move, so `item_instances` (soulbound state) follows them.
- A missing guild record means the default five ranks with no bank tabs.

## Moves
- `bank_storage::deposit` checks that the whole stack fits across the tab order before it changes anything. It tops up same-item stacks first, then fills empty slots.
- The item is taken from `InventoryModel` only after the deposit fits.
- `save_bank_move` writes the bank, the guild log and usage, and the owner's `InventoryRecord` in one write transaction. It goes through `persistence_inventory::write_inventory_record`.
- `GoldUpdateQueue::record` then pushes `Gold` and `InventoryDelta`.
- Item guids are minted (`next_id`) before the write transaction, because redb allows one writer at a time.

## Openings
- **Bankers:** they use gossip menu 699 (option type 9 → `NpcRole::Banker`). `bank::send_bank_contents` reacts to `Changed<ActiveInteraction>`.
- **Vaults:** they open through `UseGameObject` (see [game objects](game-objects.md)). `guild_bank::send_opened_guild_banks` sends the bank, or `NotInGuild`.
- After any guild change, `broadcast_changed_guild_banks` sends each member with that guild's vault open their own rank view.

## Daily reset
- `DailyReset::day(unix)` = `floor((unix - hour*3600) / 86400)`, with the hour from `GAME_SERVER_DAILY_RESET_HOUR_UTC`.
- Usage from another day or another guild counts as zero.
