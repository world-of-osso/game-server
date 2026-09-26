# Trade

How the server implements trade. The contract is in [trade spec](../../specs/trade.md).

- `shared::trade::TradeManager` (shared-protocol) holds sessions: `initiate_trade` → `accept_trade` → offer edits (each clears both accepts) → `confirm_trade`. A fully accepted session stays until the server calls `finish_trade` (executed) or `refuse_accepts` (refused).
- `trade.rs` systems read the eight request messages plus `CancelTradeAccept`; `enforce_trade_range` (FixedUpdate) cancels sessions whose players are apart or no longer spawned.
- Initiation checks (`initiation_refusal`) read `Health`, `ActiveAuras::is_stunned` (`SPELL_AURA_MOD_STUN`), `TaxiFlight` and the race faction of both `Player` entities.
- `trade_support::execute_completed_trade` loads both `InventoryModel`s, takes the traded slots (`take_bag_item`, partial stacks get a minted guid), checks gold, stores the received items (`store_item`), and writes both `InventoryRecord`s in one transaction; `GoldUpdateQueue` then sends `Gold` and `InventoryDelta` to both.
- Snapshots (`TradeStateUpdate.trade`) are built per viewer: `player` is the viewer's side, `other` the partner's.
