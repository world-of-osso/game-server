# Disenchant

Verified2026-10-10 on companion `disenchant` branches; unmerged. [Contract](../../specs/disenchant.md). Retail bag-GUID cast13262/effect99 now validates, destroys, collects appearances and opens personal material loot.

## Content

`SpellCastIntent.target_item_guid` stays on the per-cast SpellInfo clone through queued and active casts. Effect99 uses a once-per-cast handler; item target26 never becomes a unit entity. Both start and completion load the owned bags, require learned profession spell13262, select eligible Retail metadata and compare Enchanting333 rank with SkillRequired. Ordinary spell-not-known errors retain their previous contract; disenchant uses Retail NotKnown.

Eligibility comes from local Retail12.1.0.69933 ItemDisenchantLoot, FDID1134087/layout5845A4AA:166 rows, every SkillRequired0. No low-skill rejection is authentic against this dataset; no synthetic threshold or low-skill test. `scripts/import-disenchant.py` imports the previously extracted DB2 without a full world rebuild; db2.py also registers it for future local extraction/full imports. Both import paths share signed8 Subclass/ExpansionID conversion. Legacy DisenchantID/RequiredDisenchantSkill are not consulted. Item727 selects ID3, not legacy21.

Material rolls use [the restored TDB source](content-import.md#disenchant-source-restoration-2026-10-10), not guessed materials. TDB ItemType0 is an item,1 a reference; the loader converts this explicitly and keeps TDB references separate from content-policy creature references. Existing grouped/ungrouped chance and count processing is reused. The source's disposal receipt and bag removal pass through one `save_inventory_record` transaction, which calls `learn_account_appearances_in` before commit. Unbound BoE appearance collection is the explicit [Warbands disposal rule](../investigations/appearance-acquisition.md).

ItemLoot plus personal slots attach to the player as a loot source; wire `corpse` is the existing loot-source handle, not an invented NPC/corpse. Existing LootResponse, LootSlotRequest, LootUnit(auto), removal and close messages handle materials. A failed store retains its slot. Release discards remaining non-openable item loot, matching pinned LootHandler.cpp:310–335; a later item can be disenchanted. Unclaimed loot is runtime-only, not durable across restart. Bonus-list level/quality overrides and ItemCurrencyCost-only candidates are unimplemented because their inputs are absent from the existing inventory model.

## Proof

Native agent-run tests: six real-item gameplay tests, metadata1, creature/shared-loot regressions27 and unchanged generic-unknown-spell1 pass. Item727 is an uncommon unbound BoE, materials are checked against entry3's actual item IDs and MinCount/MaxCount, and removal plus appearance survive redb close/reopen. Item25 is grey and retained on CantBeDisenchanted; missing Enchanting is retained on NotKnown. Completion ownership, manual taking, auto-taking and release/next-item are exercised. No synthetic nonzero skill test. Ledger: `/home/osso/.worktrees/handoff-disenchant.md`.

## Sources
- [Contract](../../specs/disenchant.md)
- [Content import/provenance](content-import.md)
- [Atomic acquisition receipts](../investigations/appearance-acquisition.md)
- [Pinned Item eligibility](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Item/Item.cpp#L2226-L2288)
- [Pinned non-openable item release](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Handlers/LootHandler.cpp#L310-L335)

## See Also
- [[professions]] — learned spell and skill state
- [[spell-casting]] — queued/cast snapshots and effects
- [[inventory]] — atomic disposal receipts
