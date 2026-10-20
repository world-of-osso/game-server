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

### Exact character-bank slots (2026-10-08)
`inventory::process_item_moves` handles the existing SwapItem/SplitItem messages on InventoryChannel. When either location is `Bank { tab, slot }`, it checks the connection's open Banker role, attaches character tabs and GUID-qualified definitions/binding to InventoryModel, then applies the same cell operations used for bags. A successful move writes both records through `save_bank_move`; ChangedBanks triggers a fresh BankContents while GoldUpdateQueue drives ordinary bag/equipment deltas. No parallel bank swap/split messages or persistent-row encoding changes.

TrinityCore **a352b1fa**: ItemHandler.cpp `HandleSwapItem` (130-174, IsBankPos/CanUseBank), `HandleSplitItemOpcode` (33-67); Player.cpp `SplitItem` (12473-12588), `SwapItem` (12589-12930), `CanBankItem` (10997-11147), `CanStoreItem_InSpecificSlot` (10045-10125), `StoreItem`/`BankItem`; Item.cpp `CanBeMergedPartlyWith` (1460-1474). Stack caps and partial merges preserve remainder; full targets take the real-swap path. Both swap directions validate specialty containers. Nonempty equipped bags cannot be stored in bank cells. Character banks accept soulbound items. Unique ownership is unchanged by same-owner moves: TC excludes the moved item from CanTakeMoreSimilarItems and decrements a split's source before checking its clone. These requests neither acquire nor duplicate items. Currency-token, foreign-owner and child-item runtime flags are not modeled here; no new support for those types is claimed.

Tests and externally visible contract: [banks spec](../../specs/banks.md). Warband/guild exact-slot operations remain out of scope.

### Closed-bank protocol rejection (2026-10-09)

`8ac442f` proves that `CloseInteraction` followed by a valid character-bank `SwapItem` on InventoryChannel returns `CantDoRightNow` without changing the complete persisted inventory or bank; an open-bank control accepts the same request. This is distinct from BankChannel's `NotInteracting` response. [Inventory contract](../../specs/inventory.md#tests-asserting-this-spec) owns the exact test and persistence assertions; `104ae6a` lands the slice.

## Openings
- **Bankers:** they use gossip menu 699 (option type 9 → `NpcRole::Banker`). `bank::send_bank_contents` reacts to `Changed<ActiveInteraction>`.
- **Vaults:** they open through `UseGameObject` (see [game objects](game-objects.md)). `guild_bank::send_opened_guild_banks` sends the bank, or `NotInGuild`.
- After any guild change, `broadcast_changed_guild_banks` sends each member with that guild's vault open their own rank view.

## Daily reset
- `DailyReset::day(unix)` = `floor((unix - hour*3600) / 86400)`, with the hour from `GAME_SERVER_DAILY_RESET_HOUR_UTC`.
- Usage from another day or another guild counts as zero.

## Player rank controls

`guild_ranks.rs` handles authenticated `GuildRankRequest` packets on GuildChannel.
`save_guild_ranks` commits the rank list and materialized member positions together;
`ChangedGuilds` refreshes open vaults after edits. `guilds` layout 3 uses pinned TC
`a352b1fa` right values. Layout-2 compact flags translate bit by bit, preserving
mixed officer masks; layouts 0/1 keep their original default grants. Each `load_guild` restores the Guild Master's modeled social mask, matching pinned TC LoadFromDB, so newly modeled rights also appear on already-layout3 guilds. Other ranks and authoritative bank booleans/allowances are unchanged. The public-note bit exposed a startup/reopen regression in the existing migration test; updated code/reopen proof is tracked by the [rank contract](../../specs/guild-ranks.md). Startup writes
rows and the version together once, preserving bank state and member positions. Chat gates speaker and listener
before recording recipients in history. Social permission checks include the persisted
guild ID, not only a cached component; hidden tab settings stay stored independently
from the view/action gate.

## Sources

- [Guild ranks contract](../../specs/guild-ranks.md) — Retail controls, authority and persistence guarantees.
- [guild_ranks.rs](../../../crates/server/src/guild_ranks.rs) — request decisions and recipient snapshots.
