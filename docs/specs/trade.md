# Trade

Player-to-player trade following AzerothCore `TradeHandler.cpp` with Retail text. Messages use `TradeChannel` (shared-protocol `protocol/gameplay_messages.rs`, `shared::trade::TradeManager`). The client frame is specified in game-engine `docs/specs/trade-frame.md`; for how it works see [trade](../wiki/systems/trade.md).

## What it must do
- [x] `InitiateTrade` names an online player (case-insensitive, `ERR_CHAT_PLAYER_NOT_FOUND_S`). Refused in AzerothCore order: you dead, you stunned, either in flight (`ERR_TRADE_TOO_FAR`), target dead (`ERR_TRADE_TARGET_DEAD`), target stunned, other faction (`ERR_PLAYER_WRONG_FACTION`, `AllowTwoSide.Trade` 0), target busy (`ERR_PLAYER_BUSY_S`), yourself, beyond 11.11 yd. There is no combat check (AzerothCore has none; Retail's rule is unverified).
- [x] The target sees `PendingIncoming` and answers `AcceptTrade` (window opens) or `DeclineTrade` (both see `ERR_TRADE_CANCELLED`).
- [x] Each side has 7 slots; slot 7 ("Will not be traded") shows an item without trading it. An offered item must be a bag item with enough in the stack (`ERR_ITEM_NOT_FOUND`), not soulbound outside slot 7 (`ERR_TRADE_BOUND_ITEM`). Money must be owned (`ERR_NOT_ENOUGH_MONEY`).
- [x] Any change to either offer clears both accepts. `CancelTradeAccept` withdraws the player's own accept.
- [x] When both accept, both sides' items leave their bags, money moves, and each side stores what it receives (topping up stacks first) in one database transaction. A side without room (`ERR_TRADE_BAG_FULL` / `ERR_TRADE_TARGET_BAG_FULL`), past the gold cap (`ERR_TOO_MUCH_GOLD`), no longer holding its offered money (`ERR_NOT_ENOUGH_MONEY`, checked before the items, as TrinityCore `HandleAcceptTradeOpcode`), or no longer holding an offered item (`ERR_ITEM_NOT_FOUND`) refuses the exchange: nothing moves, both accepts clear and the window stays open. Success sends `ERR_TRADE_COMPLETE` to both.
- [x] Moving beyond 11.11 yd or leaving the world cancels the trade (`ERR_TRADE_TOO_FAR`). `CancelTrade` cancels it for both.
