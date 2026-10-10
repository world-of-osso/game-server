# Disenchant

Verified2026-10-10: gameplay merged to master; empty-material compatibility correction on `disenchant-empty`. [Contract](../../specs/disenchant.md). Retail bag-GUID cast13262/effect99 now validates, destroys, collects appearances and opens personal material loot.

## Content

`SpellCastIntent.target_item_guid` stays on the per-cast SpellInfo clone through queued and active casts. Effect99 uses a once-per-cast handler; item target26 never becomes a unit entity. Both start and completion load the owned bags, require learned profession spell13262, select eligible Retail metadata and compare Enchanting333 rank with SkillRequired. Ordinary spell-not-known errors retain their previous contract; disenchant uses Retail NotKnown.

Eligibility comes from local Retail12.1.0.69933 ItemDisenchantLoot, FDID1134087/layout5845A4AA:166 rows, every SkillRequired0. No low-skill rejection is authentic against this dataset; no synthetic threshold or low-skill test. `scripts/import-disenchant.py` imports the previously extracted DB2 without a full world rebuild; db2.py also registers it for future local extraction/full imports. Both import paths share signed8 Subclass/ExpansionID conversion. Legacy DisenchantID/RequiredDisenchantSkill are not consulted. Item727 selects ID3, not legacy21.

Material rolls use [the restored TDB source](content-import.md#disenchant-source-restoration-2026-10-10), not guessed materials. TDB ItemType0 is an item,1 a reference; the loader converts this explicitly and keeps TDB references separate from content-policy creature references. Existing grouped/ungrouped chance and count processing is reused. The source's disposal receipt and bag removal pass through one `save_inventory_record` transaction, which calls `learn_account_appearances_in` before commit. Unbound BoE appearance collection is the explicit [Warbands disposal rule](../investigations/appearance-acquisition.md).

ItemLoot plus personal slots attach to the player as a loot source; wire `corpse` is the existing loot-source handle, not an invented NPC/corpse. Existing LootResponse, LootSlotRequest, LootUnit(auto), removal and close messages handle materials. A failed store retains its slot. Release discards remaining non-openable item loot, matching pinned LootHandler.cpp:310–335; a later item can be disenchanted. Unclaimed loot is runtime-only, not durable across restart. Bonus-list level/quality overrides and ItemCurrencyCost-only candidates are unimplemented because their inputs are absent from the existing inventory model.

## Proof

Native agent-run tests: six real-item gameplay tests, metadata1, creature/shared-loot regressions27 and unchanged generic-unknown-spell1 pass. Item727 is an uncommon unbound BoE, materials are checked against entry3's actual item IDs and MinCount/MaxCount, and removal plus appearance survive redb close/reopen. Item25 is grey and retained on CantBeDisenchanted; missing Enchanting is retained on NotKnown. Completion ownership, manual taking, auto-taking and release/next-item are exercised. No synthetic nonzero skill test. Ledger: `/home/osso/.worktrees/handoff-disenchant.md`.

## Stale material compatibility — 2026-10-10

Private native client proof on server2aa9da0 found Dragonmaw Shortsword753 selects TDB14 but all materials11082/11083/11084 are absent from the active Retail build. The pre-fix cast destroyed753/collected216, sent empty personal loot hidden by the client's empty LootFrame, then locked the next754 cast with InProgress. Relog clears that runtime session. Canonical world.db was never modified.

The compatibility correction checks the existing loot probability traversal against active `ItemTemplates` at cast start and completion. An entry with zero possible deliverable material is refused with CantBeDisenchanted before disposal; no ItemLoot/ActiveLoot is created. Completion additionally rolls once before costs, CastSuccess or SpellGo. An empty result is refused at that boundary; a nonempty result is held on the caster until effect99 consumes that same roll for disposal/loot. No reroll or changed probabilities. This is an explicit data-compatibility policy for stale TDB/build joins, not an assertion that Retail itself contains this inconsistent dataset. Pinned TC Spell.cpp7796–7814 validates eligibility before cast; SpellEffects.cpp3493–3508 fills personal loot and defers item disposal. Those sources do not directly specify the stale-build case. No replacement materials or changed drop rates.

Read-only census of TDB1210.26091 versus Retail12.1.0.69933:180 rows/100 entries;72 rows reference16 item IDs absent from `content_item` and both `Item.csv`/`ItemSparse.csv`. Fifty entries affected:40 wholly unavailable,10 partially unavailable; no reference-type rows. Missing IDs:10978,10998,11082,11083,11084,11134,11135,11137,11138,11139,11174,11175,11176,11177,11178,20725. Wholly unavailable entries:9–30,38–41,48–49,54–55,60–61,72–79. Partial entries:5–8,31–32,44–45,50–51. Existing probabilities/filter behavior remain; their empty outcomes are also refused rather than disposing items. Missing materials remain unavailable; this does not certify full material parity.

Regression uses real753 and its three missing material IDs: refusal retains its GUID and leaves appearance216 unlearned; no loot response/session; real727 casts next and produces material loot while753 remains. RED reproduced missing refusal on unmodified code. Actual partial entry5 high-roll test uses real1190, authentic20%10939/75%10940 and absent5%10978; checked empty roll returns refusal and retains item/unlearned appearance. This controlled-RNG completion test complements the real753 message-path regression. Evidence/census and live logs: `game-engine/data/diagnostics/disenchant-live-2026-10-10/`; handoff `/home/osso/.worktrees/handoff-disenchant-live.md`. Targeted GREEN and post-fix live proof pending.

## Sources
- Pinned TC a352b1fa `Spell.cpp:7796–7814`, `SpellEffects.cpp:3493–3508`; downloaded primary-source excerpts in the diagnostics directory.
- `disenchant-census.json` in the diagnostics directory — exact TDB rows and independent content/Item/ItemSparse membership flags.
- [Contract](../../specs/disenchant.md)
- [Content import/provenance](content-import.md)
- [Atomic acquisition receipts](../investigations/appearance-acquisition.md)
- [Pinned Item eligibility](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Entities/Item/Item.cpp#L2226-L2288)
- [Pinned non-openable item release](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/Handlers/LootHandler.cpp#L310-L335)

## See Also
- [[professions]] — learned spell and skill state
- [[spell-casting]] — queued/cast snapshots and effects
- [[inventory]] — atomic disposal receipts
