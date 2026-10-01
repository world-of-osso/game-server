# Midnight Economy Content

Corrected Midnight Leatherworking source diagnosis and bounded ingestion evidence. Verified: 2026-10-01. Seven reference tables and 2,978 genuine item templates are now published to local `world.db`; the active Classic economy catalog is unchanged. Wowhead browser evidence now identifies a genuine Silvermoon supplies merchant; runtime spawn coordinates and facing remain unavailable.

## Root cause

The old catalog inspected CREATE_ITEM effect 24 and inferred missing recipes from absent SkillLineChild membership. Actual SkillLineAbility rows use parent profession 165 and SkillupSkillLineID 2915: 105 CRAFT_ITEM effect-288 spells reference CraftingData through EffectMiscValue0. Of these, 85 resolve direct expansion-11 outputs, nine direct housing outputs use expansion -3, and eleven require quality-output resolution. The historical six Fishing/109 unmapped effect-24 counts never measured this modern slice.

## Corrected staging evidence

Seven genuine pinned reference tables are imported into an explicit staging database by `crafting_data.py`. [[crafting-reference-import]] owns exact schema, pins, Python interface and CLI; do not duplicate that contract here. The additive content projection reports 3,414 referenced IDs, 427 existing rows, 2,978 additions and nine missing ItemSparse rows; existing content rows are untouched. Canonical publication uses the same verified import/projection tools. All 54,887 pre-existing item rows were preserved; `world-before-publication.db` retains the original snapshot. The publication manifest and reports are under the evidence directory below. No production service was restarted.

Published required-reagent evidence retains 105 raw HTML records and retrieval/hash metadata: 63 concrete records and 42 placeholders. DB2 cross-checks, TSM availability and content-template checks yield a diagnostic catalog with 15 Midnight Leatherworking recipes and 21 market entries. Published pages are **not native C_TradeSkillUI captures** and do not prove full quality simulation. No FlagsRequired semantics are assumed.

Saved skinned-from caches for raw items 238511 and 238513 each contain 200 positive NPC sources. This evidence permits two gatherables without fabricating creature loot or spawns. It proves published gathering provenance, not local placement of those NPCs.

`midnight-shadow-catalog.json` and `midnight-shadow-report.json` assign Lara Moore `1295:11:165` **DIAGNOSTIC ONLY**. Lara is not a substitute for a new Silvermoon NPC and this assignment is not publication acceptance.

## Genuine city-NPC gap

New Silvermoon is AreaTable 15969, ContinentID 0. Map 2552/zone 14717 identify Khaz Algar/Isle of Dorn, not this city. The audited full TDB release archive contains 262,623 creature spawn rows, including seven city quest/story spawns and no city merchants. Forty-five post-release SQL updates were checked without finding merchant rows. This bounds the inspected sources; it does not prove no merchant exists in Retail or future source releases.

## Browser merchant evidence (2026-10-01)

Saved browser-cli evidence from [Zaralda on Wowhead](https://www.wowhead.com/npc=243531/zaralda) identifies NPC **243531**, tag **Leatherworking Supplies**. The browser agent observed Midnight patch **12.0.0**; this is published-page context, not verification against the imported 12.1.0 reference build. The NPC listing was extracted as **all 34 sells rows**, without navigating to individual item pages.

| Item | ItemID | NPC listing monetary cost | Raw stack | Raw availability |
|------|--------|---------------------------|-----------|------------------|
| Silverleaf Thread | 251665 | 700 | `[1]` | `-1` |
| Embroidery Floss | 251691 | 700 | `[1]` | `-1` |

Both rows retain raw `cost: [[700, [], []]]`. A read-only canonical `world.db` query confirms both `content_item` rows have `BuyPrice = 700` and `VendorStackCount = 1`. A separately observed generic item listing shows Silverleaf Thread at **736**, versus **700** on the NPC listing; the reason for that listing discrepancy remains unverified. Raw `avail: -1` does not establish runtime unlimited-stock semantics or restock behavior; `stack: [1]` is preserved without asserting the runtime purchase contract.

The saved mapper places Zaralda in **Silvermoon City**, **uiMapId 2393**, under location **15969**, with marker **43.2, 55.8**. These are UI-map marker coordinates, not runtime world x/y/z or facing. This closes the published merchant-identity/listing gap, not the genuine placed runtime merchant gate. No merchant data was imported or activated; Classic/catalog remains unchanged.

The 15 currently staged eligible recipes require neither item. The published-reagent comparison found Silverleaf Thread in 30 other recipe records, outside this staged set. Zaralda therefore does not supply a missing ingredient for the currently eligible recipes; merchant publication and further recipe eligibility remain distinct work.

## Remaining gates and limits

- Independent ingestion gate passed 114 distinct targeted tests; `/tmp/midnight-ingestion-verify-357086b/report.md` records exact scope and caveats. A separate read-only publication gate checks the canonical data.
- Rebuild preflight is implemented and tested: eleven reference-import and six bounded rebuild tests passed. `CRAFTING_SOURCE_DIR` defaults to `data/reference/crafting/12.1.0.69933`; validation happens before database deletion. [[crafting-reference-import]] owns the command and schema details.
- A genuine placed city merchant is still required: Zaralda's published identity and UI marker do not supply runtime x/y/z/facing; diagnostic reassignment cannot satisfy it.
- Availability/restock semantics remain unverified; the 34-row browser extraction is listing evidence, not runtime purchase proof. The two reagent prices and bundle counts match canonical item data, but vendor rows remain absent.
- Spark placeholders, quality-only outputs and native bonus/quality behavior remain unsupported; published HTML is not a native crafting capture.
- Nine missing ItemSparse rows remain reported, not fabricated or silently replaced.

## Sources

Publication evidence is retained under `data/reference/midnight-20261001/`; browser evidence is under `/tmp/wowhead-midnight-merchants-20261001/`. Neither was generated by this docs update.

- [Projection report](../../../data/reference/midnight-20261001/crafting-projection-report.json) — additive staging counts and missing templates.
- [Canonical publication](../../../data/reference/midnight-20261001/canonical-publication-manifest.json) and [canonical diagnostic report](../../../data/reference/midnight-20261001/canonical-midnight-diagnostic-report.json) — local publication and explicit diagnostic assignment, not activation of a city vendor.
- [Published requirements](../../../data/reference/midnight-20261001/published-reagents-2915.json) and `published-reagent-pages/` — raw pages, explicit published-page provenance, concrete requirements and unresolved placeholders.
- [Shadow catalog](../../../data/reference/midnight-20261001/midnight-shadow-catalog.json) and [report](../../../data/reference/midnight-20261001/midnight-shadow-report.json) — recipe eligibility, markets, gathering evidence and diagnostic assignment.
- [Silvermoon source proof](../../../data/reference/midnight-20261001/silvermoon-source-proof.json) — full archive, seven city spawns and 45-update audit.
- [Importer](../../../scripts/economy_catalog.py) — effect-288 resolution and eligibility boundary.
- [Zaralda NPC listing](https://www.wowhead.com/npc=243531/zaralda) — browser-observed merchant identity and sells listing, 2026-10-01.
- [Parsed browser evidence](/tmp/wowhead-midnight-merchants-20261001/zaralda-observed.json), [raw HTML/listing log](/tmp/wowhead-midnight-merchants-20261001/npc-html-listing-observation.log) and [raw sells observation log](/tmp/wowhead-midnight-merchants-20261001/npc-sells-observation.log) — all 34 extracted rows and UI mapper; temporary local artifacts, not durable repository data. Patch context and generic-listing 736 comparison are browser-agent observations supplied with this docs update.

## See Also

- [[economy-catalog]] — active Classic catalog and historical effect-24 coverage.
- [[crafting-reference-import]] — seven-table import contract and schema.
- [[bot-economy]] — bounded runtime receipts, distinct from new content publication.
