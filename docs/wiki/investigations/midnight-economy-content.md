# Midnight Economy Content

Corrected Midnight Leatherworking source diagnosis and bounded ingestion evidence. Verified: 2026-10-01. Seven reference tables and 2,978 genuine item templates are now published to local `world.db`; the default local economy catalog now combines preserved Classic data with Zaralda Midnight additions. Genuine Zaralda source identity and two unlimited suppliers are published locally with user-approved approximate placement derived from a Wowhead marker and client terrain. Source/data proof and private CLI receipts are bounded; artifact gate103 reports scoped PASS and native acceptance remains blocked (0/3).

## Root cause

The old catalog inspected CREATE_ITEM effect 24 and inferred missing recipes from absent SkillLineChild membership. Actual SkillLineAbility rows use parent profession 165 and SkillupSkillLineID 2915: 105 CRAFT_ITEM effect-288 spells reference CraftingData through EffectMiscValue0. Of these, 85 resolve direct expansion-11 outputs, nine direct housing outputs use expansion -3, and eleven require quality-output resolution. The historical six Fishing/109 unmapped effect-24 counts never measured this modern slice.

## Corrected staging evidence

Seven genuine pinned reference tables are imported into an explicit staging database by `crafting_data.py`. [[crafting-reference-import]] owns exact schema, pins, Python interface and CLI; do not duplicate that contract here. The additive content projection reports 3,414 referenced IDs, 427 existing rows, 2,978 additions and nine missing ItemSparse rows; existing content rows are untouched. Canonical publication uses the same verified import/projection tools. All 54,887 pre-existing item rows were preserved; `world-before-publication.db` retains the original snapshot. The publication manifest and reports are under the evidence directory below. No production service was restarted.

Published required-reagent evidence retains 105 raw HTML records and retrieval/hash metadata: 63 concrete records and 42 placeholders. DB2 cross-checks, TSM availability and content-template checks yield a diagnostic catalog with 15 Midnight Leatherworking recipes and 21 market entries. Published pages are **not native C_TradeSkillUI captures** and do not prove full quality simulation. No FlagsRequired semantics are assumed.

Saved skinned-from caches for raw items 238511 and 238513 each contain 200 positive NPC sources. This evidence permits two gatherables without fabricating creature loot or spawns. It proves published gathering provenance, not local placement of those NPCs.

`midnight-shadow-catalog.json` and `midnight-shadow-report.json` assign Lara Moore `1295:11:165` **DIAGNOSTIC ONLY**. Lara is not a substitute for a new Silvermoon NPC and this assignment is not publication acceptance.

## Native city-spawn source limitation

New Silvermoon is AreaTable 15969, ContinentID 0. Map 2552/zone 14717 identify Khaz Algar/Isle of Dorn, not this city. The audited full TDB release archive contains 262,623 creature spawn rows, including seven city quest/story spawns and no city merchants. Forty-five post-release SQL updates were checked without finding merchant rows. This bounds the inspected sources; it does not prove no merchant exists in Retail or future source releases.

## Browser merchant evidence (2026-10-01)

Saved browser-cli evidence from [Zaralda on Wowhead](https://www.wowhead.com/npc=243531/zaralda) identifies NPC **243531**, tag **Leatherworking Supplies**. The browser agent observed Midnight patch **12.0.0**; this is published-page context, not verification against the imported 12.1.0 reference build. The NPC listing was extracted as **all 34 sells rows**, without navigating to individual item pages.

| Item | ItemID | NPC listing monetary cost | Raw stack | Raw availability |
|------|--------|---------------------------|-----------|------------------|
| Silverleaf Thread | 251665 | 700 | `[1]` | `-1` |
| Embroidery Floss | 251691 | 700 | `[1]` | `-1` |

Both rows retain raw `cost: [[700, [], []]]`. A read-only canonical `world.db` query confirms both `content_item` rows have `BuyPrice = 700` and `VendorStackCount = 1`. A separately observed generic item listing shows Silverleaf Thread at **736**, versus **700** on the NPC listing; the reason for that listing discrepancy remains unverified. Follow-up browser inspection of both item pages shows Zaralda with **Stock `∞`**, **Stack `1`**, and **Cost 7 silver** in their Sold by tables. This explicitly supports unlimited supplier stock and single-item purchases; raw `avail: -1` is consistent with that display. No restock interval is shown, but limited-stock restock timing is not a prerequisite for these unlimited suppliers.

The saved mapper places Zaralda in **Silvermoon City**, **uiMapId 2393**, under location **15969**, with marker **43.2, 55.8**. These are UI-map marker coordinates, not a Retail spawn capture. The user explicitly approved deriving approximate placement from this marker and client terrain; local publication is recorded below. Classic/catalog was unchanged at initial publication; the later additive combined configuration is recorded below.

The sibling engine's pinned `game-engine/data/db2/12.1.0.69933/UiMapAssignment.csv` has assignment 68573 for UI map 2393, map 0, area 15969, with UI bounds `(0,0)–(1,1)` and world-region bounds `(7902.083, -6141.666)–(9772.916, -3335.416)`. This supplies the horizontal transform, not a captured spawn. Native TDB template/model/difficulty rows exist for 243531; source display 138959 has scale 1, probability 1 and verified build 69497. Raw TDB `npcflag=0` is preserved; derived content `npcflag=128` comes from the verified Wowhead merchant listing, not native vendor flags. Approved mapper overrides merchant role and observed levels 80–90 while retaining other authentic native fields. Absence of a native `tdb_creature` spawn does not mean source identity or model is missing.

## Approved derived placement and local publication

Tracked [`derived-merchants.json`](../../../scripts/derived-merchants.json) owns placement and supplier provenance. Actual CASC build **12.1.0.69933** supplied ADT FileDataID **6173902**, `world/maps/azeroth/azeroth_40_15.adt`. Assignment **68573** transforms the marker to map 0 world **X 8728.991197265625, Y -4547.716015625**; terrain sampling yields **Z 23.438097**. This is approximate UI-marker/ground placement, not captured Retail coordinates or a building-floor capture. Facing **0 radians**, stationary movement and default respawn are local policy, not observed Retail behavior.

Commits `830157a` (terrain), `94e6d88` (publisher) and `e43f4d3` (integration) implement this bounded path. `content_policy.py --derived-merchants` loads the required manifest during `--check`, before destructive rebuild. Missing/invalid provenance fails preflight rather than silently omitting the merchant.

Canonical `data/world.db` publication adds **one template, one model, one spawn and two vendor rows**, with SQLite authorizer writes restricted to those four content tables. Only Silverleaf Thread **251665** and Embroidery Floss **251691** were imported: **700 copper per bundle of one**, explicit visible unlimited stock, runtime `maxcount=0/incrtime=0`. Unsupported currency rows from the wider listing were excluded; the full 34-row listing was not imported.

Evidence directory `data/reference/midnight-20261001/` retains `world-before-zaralda.db`, `world-zaralda-stage.db`, `zaralda-publication-manifest.json` and persistent raw `zaralda-wowhead-observed.json`. No active Classic catalog change or service restart occurred at publication. Subsequent combined-config and owned private-server receipts are recorded below; no existing server or production service was restarted.

The 15 currently staged eligible recipes require neither item. The published-reagent comparison found Silverleaf Thread in 30 other recipe records, outside this staged set. Zaralda therefore does not supply a missing ingredient for the currently eligible recipes; merchant publication and further recipe eligibility remain distinct work.

## Combined local catalog and private CLI proof (2026-10-01)

Default local `data/economy/config.json` now combines Lara Moore **1295**, Classic **223 recipes**, with genuine Zaralda **243531**, Midnight **15 recipes**: **238 recipes, 332 markets, 29 gatherables**. Existing merchants, recipes, metadata, market values, suppliers and global policy values are preserved exactly; only new data is added. Baseline [`classic-before-zaralda-config.json`](../../../data/reference/midnight-20261001/classic-before-zaralda-config.json), [`classic-zaralda-catalog.json`](../../../data/reference/midnight-20261001/classic-zaralda-catalog.json) and [`classic-zaralda-preservation.json`](../../../data/reference/midnight-20261001/classic-zaralda-preservation.json) retain comparison evidence. **Artifact gate103 reports scoped PASS** in `/tmp/zaralda-catalog-live-verify-20261001/report.md`: full Classic record preservation, candidate/default SHA equality, purchase conservation, complete saved restart-state equality and both live merchant entries. Its literal native-vendor-flag claim FAIL is corrected above: raw 0/content 128 is intentional derived policy, not an implementation failure.

Owned private server **5197** used current binaries compiled once in **22m08s**, exit **0**. Initial Zaralda config contained 15 eligible recipes; saved stock proves crafted outputs **244589/244586**, not all 15 recipes. Actual CLI list and buy of **244589** confirmed inventory at **4,122,451 copper**, with cap-overflow withdrawal state. Full private restart preserved exact economy JSON equality. After loading the combined config, both merchants were observed; subsequent `combined-economy-established.json` records **29 gatherer actors**, beyond the initial startup receipt's two item types. This does not prove every configured recipe crafted. These are bounded saved receipts, not native acceptance, general availability or production deployment. No existing server/prod restart occurred. Receipts: [`data/diagnostics/zaralda-20261001/`](../../../data/diagnostics/zaralda-20261001/), including `combined-economy-status.json`, `combined-economy-established.json`, `merchant-list.json`, `merchant-buy.json` and `restart-comparison.json`. Build receipt covers **ec047855**, not a current-HEAD compilation claim.

## Native acceptance blocker

Sibling fixture `game-engine/godot/tests/world_zaralda_merchant_flow.gd` at **7903cb5e** was prepared, parsed and executed, but did not pass its native window boundary. Three read-only private cage attempts using the existing extension produced:

| Attempt | Observed boundary | Merchant proof |
|---|---|---|
| `native` | Pre-authentication 5-second handshake timeout | None |
| `native-retry` | Authenticated, world attached two units and actual player coordinates; still Loading at 240 seconds, 22,199 objects queued / 4,643 spawned | None |
| `native-texture-fixed` | Same pre-authentication 5-second timeout after real local-CASC texture 6243047 extraction | None |

Artifacts: [`game-engine/data/diagnostics/zaralda-20261001/`](../../../../game-engine/data/diagnostics/zaralda-20261001/). Texture extraction is not proof of an auth/Loading fix. LiquidObject42 and wrapper/protocol warnings remain limits, not established causes of this merchant data issue. Native window/title, item tooltip and ray-pick remain **unproven**; general native root cause remains unresolved. No fourth blind probe, runtime code fix or Depot build was performed; successful authentication on attempt two does not establish a stale protocol. Overall goal remains open.

## Remaining gates and limits

- Independent ingestion gate passed 114 distinct targeted tests; `/tmp/midnight-ingestion-verify-357086b/report.md` records exact scope and caveats. A separate read-only publication gate checks the canonical data.
- Rebuild preflight is implemented and tested: eleven reference-import and six bounded rebuild tests passed. `CRAFTING_SOURCE_DIR` defaults to `data/reference/crafting/12.1.0.69933`; validation happens before database deletion. [[crafting-reference-import]] owns the command and schema details.
- Approved derived placement and two supplier rows are locally published; merchant identity, unlimited stock and lack of exact Retail capture are no longer blockers. Diagnostic Lara reassignment still cannot substitute for Zaralda.
- Latest source **e9d281f**: independent fixture/reimport closure **30/30**, zero warnings, and changed-file Ruff PASS in `/tmp/zaralda-closure-verify-20261001/report.md`. Prior unchanged-scope publisher **15/15**, terrain **11/11** and other-file Ruff remain valid in `/tmp/zaralda-final-verify-20261001/report.md`; that earlier report's fixture timeout/warnings are superseded only for the closure run. No intermittent host-stall remediation is claimed.
- Retained data proof: **225 tables**, identical prior schema and **10,081,961 prior rows preserved**, exactly **five authorized additions**. Persistent rebuilds retain authentic native TDB template/model and difficulty identity; two-full-rebuild fixture proof passes. Source-only thread/floss supplier boundary remains unchanged.
- Gate103's scoped artifact PASS and private CLI purchase/restart/combined-config receipts do not close native acceptance. Do not mark the full goal complete.
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
- [Derived manifest](../../../scripts/derived-merchants.json) — approved approximate placement, source hashes and two supplier rows.
- [Publication manifest](../../../data/reference/midnight-20261001/zaralda-publication-manifest.json) — revision, additive deltas, protected table scope and unchanged catalog/service state.
- [Persistent Wowhead observation](../../../data/reference/midnight-20261001/zaralda-wowhead-observed.json) — retained raw browser provenance.
- [Content policy](../../../scripts/content_policy.py) — required derived-manifest preflight and publication integration.
- [Zaralda NPC listing](https://www.wowhead.com/npc=243531/zaralda) — browser-observed merchant identity and sells listing, 2026-10-01.
- [Parsed browser evidence](/tmp/wowhead-midnight-merchants-20261001/zaralda-observed.json), [raw HTML/listing log](/tmp/wowhead-midnight-merchants-20261001/npc-html-listing-observation.log) and [raw sells observation log](/tmp/wowhead-midnight-merchants-20261001/npc-sells-observation.log) — all 34 extracted rows and UI mapper; temporary local artifacts, not durable repository data. Patch context and generic-listing 736 comparison are browser-agent observations supplied with this docs update.

Follow-up stock screenshots: `/tmp/wowhead-midnight-merchants-20261001/silverleaf-thread-visible.jpg` and `embroidery-floss-visible.jpg`. Source pages: [Silverleaf Thread](https://www.wowhead.com/item=251665/silverleaf-thread) and [Embroidery Floss](https://www.wowhead.com/item=251691/embroidery-floss).

## See Also

- [[economy-catalog]] — combined local catalog and historical effect-24 coverage.
- [[crafting-reference-import]] — seven-table import contract and schema.
- [[content-import]] — rebuild integration and required derived provenance.
- [[bot-economy]] — bounded runtime receipts, distinct from new content publication.
