# Banks

The server side of three banks, following Retail with AzerothCore rules for the guild bank:

- the tabbed character bank and the account-wide Warband bank, both at bankers (`BankChannel`, shared-protocol `protocol/bank_messages.rs`);
- the guild bank at Guild Vault game objects (`GuildBankChannel`, shared-protocol `protocol/guild_bank_messages.rs`).

The client frames are specified in game-engine `docs/specs/bank-frame.md` and `docs/specs/guild-bank-frame.md`. For how it works, see [banks](../wiki/systems/banks.md) and [game objects](../wiki/systems/game-objects.md).

## What it must do

### Game objects
- [x] Guild Vaults (`content_gameobject` rows of type 34 on map 0) spawn with the Retail `GameObjectDisplayInfo` model box. There are 12: Stormwind 4, Ironforge 2, Undercity 4, Booty Bay 2.
- [x] `UseGameObject` on a vault opens `NpcRole::GuildBanker`. The player must be alive and within 5 yd of the rotated model box (AzerothCore `GameObject::IsAtInteractDistance`, using the blizzlike distance). Leaving that reach sends `InteractionClosed`.

### Character and Warband bank
- [x] Opening the banker role sends `BankContents` for both banks. Tab prices come from the Retail `BankTab` table:

  | Bank | Tab prices (gold) |
  |---|---|
  | Character | 1, 100, 500, 1000, 2500, 5000 |
  | Warband | 1000; 25,000; 100,000; 500,000; 2,500,000 |

  Every bank starts with 0 tabs.
- [x] Entering the world sends `BankContents` for both banks without a banker, and a craft that took reagents from a bank sends that bank's new contents: Retail clients count bank reagents anywhere ([professions](professions.md)).
- [x] Buying a tab charges the player. `CantAfford` changes nothing.
- [x] A deposit moves the whole bag stack into the chosen tab, then into later tabs. It tops up same-item stacks first. `TabLocked` means no such tab, and `BankFull` changes nothing.
- [x] A withdrawal moves the whole stack into the bags through `store_item`. `InventoryFull` changes nothing.
- [x] The Warband bank is keyed by account: a stack deposited on one character can be withdrawn by another character of the same account.
- [x] Soulbound items are refused by the Warband bank (`ERR_NO_SOULBOUND_ITEM_IN_ACCOUNT_BANK`).
- [x] Only the Warband bank holds money. Deposit and withdraw check both balances; the Character bank answers `NoMoneyTransfer`.
- [x] Deposit All: the character bank takes tradeskill items (class 7). The Warband bank takes Warbound items (bonding 7–9), plus tradeable reagents when asked. Tabs assigned a matching `Enum.BagSlotFlags` are filled first.
- [x] Tab settings store the name (1–15 letters, else `InvalidTabName`), the icon and the deposit flags.
- [x] BankChannel requests must name the open banker, otherwise `NotInteracting`. InventoryChannel bank moves after `CloseInteraction` return `CantDoRightNow` and preserve the entire persisted bank/inventory (`8ac442f`; [protocol test](inventory.md#tests-asserting-this-spec)). The bank and the bags are written in one transaction.
- [x] `SwapItem` and `SplitItem` on InventoryChannel accept `ItemLocation::Bank { tab, slot }` for character-bank slots ([inventory contract](inventory.md)). Moves require an open banker; refusals use `InventoryError`. Unbought tabs and out-of-range slots are invalid destinations. Bank↔bags and bank↔bank (same/across tabs) support exact-slot moves, swaps, merges capped at the item's stack size, and partial-stack splits. Specialty bag restrictions and nonempty-container checks apply before either direction of a swap; failure changes no item rows. GUID identity, binding and total owned count survive moves and database reopen.

### Guild bank
- [x] Ranks follow TrinityCore `_CreateDefaultGuildRanks` (Guild.cpp:3184-3188): Guild Master, Officer, Veteran, Member, Initiate. Guild Master and Officer hold `GR_RIGHT_ALL` (gold withdrawal and guild repair), the others neither.
  - `create-guild` makes the character Guild Master.
  - Members without a stored rank hold Initiate.
  - The guild roster shows the stored rank name.
- [x] The Guild Master has every right, unlimited. Other ranks have per-tab view, deposit and stacks-per-day rights, plus a gold-per-day allowance when `withdraw_gold` or `withdraw_repair` (TrinityCore `GR_RIGHT_WITHDRAW_REPAIR`) is set. Gold withdrawals and guild repairs (game-server `docs/specs/merchant.md`) spend that one allowance (`_GetMemberRemainingMoney`); withdrawing gold needs `withdraw_gold`.
- [x] Guilds stored before the repair right (layout version 0, `server_settings` `guild_layout_version` unset) are rewritten once at startup to version 1: rank 0 and 1 get `withdraw_repair`, the others don't, as TrinityCore's default ranks; everything else is kept.
- [x] Players without a guild get `NotInGuild`.
- [x] Deposit needs view and deposit rights, and refuses soulbound (`BoundItem`) and quest items.
- [x] Withdraw needs view rights and a remaining daily stack (`WithdrawLimit`).
- [x] Money deposit and withdraw follow the allowance and guild balance (`NotEnoughGuildMoney`). Daily counts reset at `GAME_SERVER_DAILY_RESET_HOUR_UTC` (default 15).
- [x] Only the Guild Master buys tabs (Retail `BankTab` guild prices: 100g–5000g, 6 tabs) and sets tab name, icon and text.
- [x] Item moves are logged per tab and money moves (deposits, withdrawals, tab purchases, guild repairs) in the money log. Each log keeps its last 25 entries (`GUILD_BANKLOG_MAX_RECORDS`). `GuildBankQueryLog` sends them, with the item name and age.
- [x] Admin commands `set-guild-rank <character> <rank>` and `set-guild-bank-rights <guild> <rank> tab|gold|repair ...` edit ranks and rights. The Guild Master rank itself can't be edited.

## How it works
- [banks](../wiki/systems/banks.md)
- [game objects](../wiki/systems/game-objects.md)
- [content import](../wiki/systems/content-import.md)

## Implementation inventory
| File | Role |
|---|---|
| `crates/server/src/game_objects.rs` | Vault spawns from `content_gameobject`; the reach box |
| `crates/server/src/npc_interaction.rs` | `UseGameObject`; open and close vault interactions |
| `crates/server/src/bank.rs` | Character and Warband bank requests; `BankTabPrices` |
| `crates/server/src/bank_storage.rs` | Pure tab deposit/take model shared by all banks |
| `crates/server/src/guild_bank.rs` | Guild bank requests, rights, daily reset, logs |
| `crates/server/src/persistence_banks.rs` | redb tables for banks, guilds, ranks, usage and logs |
| `crates/server/src/admin_handlers.rs` | `set-guild-rank`, `set-guild-bank-rights` |
| `scripts/import-azerothcore.sh`, `content_policy.py`, `db2.py` | gameobject / GameObjectDisplayInfo / BankTab import |

## Tests asserting this spec
- `crates/server/src/game_objects_tests.rs`
- `crates/server/src/bank_storage_tests.rs`
- `crates/server/src/bank_tests.rs`
- `crates/server/src/bank_move_tests.rs`, `bank_move_rule_tests.rs`: protocol-driven moves, split/merge limits, restrictions and atomic persistence/reopen. Rules reference TrinityCore a352b1fa `ItemHandler.cpp::HandleSwapItem` / `CanUseBank` and `Player.cpp::SwapItem` / `SplitItem`; fetched sources are retained in `/home/osso/.worktrees/logs/tc/`.
- `crates/server/src/guild_bank_tests.rs`
- `crates/server/src/persistence_banks_tests.rs`: version 0 guilds get the default repair rights once, logs written before `RepairMoney` still read; `canonical_game_database_migrates` (ignored; `GAME_SERVER_MIGRATION_DB`) on a copy of a real database
- `crates/server/src/bin/admin.rs` (`parse_guild_rank_and_bank_rights_requests`)
- `scripts/tests/test_import_azerothcore.py` (`test_stormwind_guild_vaults_keep_retail_display_and_spawns`, `test_retail_bank_tab_prices`)

- [x] Player gold is `u64` copper, capped at the Retail 9,999,999g 99s 99c (`gold::MAX_MONEY`). All five Warband tabs (3,126,000g in total) can be bought. The Warband bank's money is capped at the same limit.
- [x] Inventories written with `u32` gold (the `inventories` table, in either older layout) move into `character_inventories` at startup, and the old table is dropped.

## Known gaps (current cycle)
- (none)

## Out of scope
- Sorting, search, the expansion filter and the tab icon picker (deferred by decision).
- Exact-slot swaps/splits in Warband and guild banks; the Guild Control UI (ranks are set by admin).
- Game objects other than type 34; maps other than 0.
