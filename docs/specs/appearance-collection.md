# Appearance collection

Account-wide appearance state uses shared-protocol `transmog::AppearanceCollection` and `AppearanceCollectionUpdate` on `TooltipChannel`. IDs currently represent Retail `ItemAppearance`, not individually collected `ItemModifiedAppearance` sources. Client tooltip contract: game-engine `docs/specs/unit-tooltip.md`.

## Retail reference (verified 2026-10-10)

- Blizzard [Warbands Preview, April 17, 2024](https://worldofwarcraft.blizzard.com/en-us/news/24061008/the-war-within-warbands-previe) and [Get the Band Together, July 11, 2024](https://worldofwarcraft.blizzard.com/en-us/news/24115313/get-the-band-together-for-warbands): collecting no longer requires the character to equip that armor type. Selling, deleting or disenchanting an unbound item also collects it. Collecting does not remove restrictions on *using* appearances.
- Blizzard [11.0.5 update notes, October 22, 2024](https://worldofwarcraft.blizzard.com/en-us/news/24145980/20th-anniversary-celebration-content-update-notes), Items: other classes collect non-cosmetic class-restricted gear; completing a quest grants every non-cosmetic reward appearance, including unseen/unselectable choices. Retail also applies these changes retroactively.
- Local Retail `GlobalStrings.csv` build 12.1.0.69933, lines 13499–13501: collected appearance, unknown appearance, and appearance collected from a different item are distinct tooltip states. `Blizzard_Collections/Mainline/Blizzard_Wardrobe.lua:1221–1241` distinguishes `visualID` and `sourceID`.

## Required behavior

- An item's default visual is its `item_modified_appearance` row with the lowest `ItemAppearanceModifierID`, then `OrderIndex`; no row means no visual to collect.
- A successful ownership/equipment write collects equipped and soulbound visuals. BoP and quest-bonded items bind on receipt; BoE items remain uncollected in bags until equipped or disposed of.
- Successful vendor sale and explicit destroy collect even an unbound BoE. Failed operations collect nothing. Generic transfer helpers do not collect an unbound item: mail, AH sales, player trade and bank transfers are not disposal.
- Collection grants merge into the account's redb row in the same transaction as inventory/equipment changes. A quest reward merges all reward-option visuals with the awarded inventory and completed quest log.
- Owner gets a sorted complete snapshot at world entry and after an inventory change changes collection state. Refresh from persisted state, not a stale per-connection copy.

## Scope gaps (not parity claims)

- No disenchant or scrapping acquisition handlers exist in the audited server. Explicit scrapping wording was not found in the Blizzard references; destruction equivalence is an inference, not verified Retail evidence.
- Refund/loot-trade timers, reversible collection, retroactive completed-quest backfill, cosmetic-specific eligibility, variant sources and source-level persistence remain missing.
- Native Wardrobe browsing/applying and collected toasts remain unimplemented; existing native tooltips receive appearance-level state.

## Behavioral proof

`transmogacq` tests cover BoP/BoE loot, BoP/BoE vendor purchase, unbound BoE sale/destroy, quest choices, player mail, AH won mail and player trade using real local item IDs. Every case closes/reopens redb. Existing appearance tests cover world-entry receipt, BoE equip, account-alt snapshots and reopen. Handoff contains the exact RED/GREEN commands and matrix; targeted proof does not replace the lead's full gate.
