# Appearance acquisition

Verified 2026-10-10. The acquisition audit corrected pre-Warbands assumptions and reproduced lost collection grants at inventory disposal boundaries. Retail rules and remaining parity gaps live in the [appearance contract](../../specs/appearance-collection.md).

## Root causes and fix

`AppearanceCollectionPlugin` originally learned only from surviving equipped/soulbound items sampled by `InventoryDeltas`. Selling or deleting an unbound appearance-bearing item removed it before that sample. New BoP items also stayed unbound until the *next* model load; acquisition writes could therefore persist neither binding nor appearance. Quest rewards learned only the selected physical item, not all non-cosmetic reward choices.

Inventory models now carry a transient account owner and appearance-grant receipt into `InventoryRecord`. Successful store operations bind BoP items before saving. The record writer merges grants with the existing account collection inside its inventory transaction; these receipt fields are not added to serialized inventory/character data. Explicit destroy and vendor sale add disposed-item visuals, while generic ownership transfers do not. Quest completion writes its non-cosmetic reward-option visuals, selected BoP cosmetics, inventory and quest log together; unchosen cosmetics remain uncollected. The connection refresh reads durable collection state so a removed item still produces an owner update.

## Proof and limits

Real IDs: Worn Shortsword 25 / visual154, Verigan's Fist6953 /2288, Gaudy Winter Veil Sweater70923 /15791, Large Round Shield2129 /684, Urchin's Pants2238 /763 and unchosen Militia Buckler2249 /356. `transmogacq` fixtures exercise actual message handlers or AH settlement/mail claims and reopen redb. Existing equip/world-entry tests exercise account-alt collection snapshots. Exact commands/revisions/results are in `/home/osso/.worktrees/handoff-transmog-acquire.md`; full integration gate belongs to the lead.

No disenchant/scrap gameplay handler, source-specific collection, refund/loot-trade reversal, retroactive quest backfill, native Wardrobe or collected-toast parity claim.

## Sources

- [Appearance contract](../../specs/appearance-collection.md) — Blizzard links and local Retail UI references.
- [Inventory model](../../../crates/server/src/inventory_model.rs) — binding and disposal receipts.
- [Inventory persistence](../../../crates/server/src/persistence_inventory.rs) — atomic receipt merge.
- [Appearance plugin](../../../crates/server/src/appearance_collection.rs) — durable-state owner refresh.
- [Quest requests](../../../crates/server/src/quest_requests.rs) — all-option reward acquisition.

## See Also

- [[inventory]] — bag/equipment request flow.
- [[persistence]] — unchanged stored ABI.
- [[mail]] — unbound ownership transfer.
- [[merchant]] — successful sale collection boundary.
