# Economy catalog importer

`scripts/economy_catalog.py` builds the bot economy JSON config from read-only `world.db` and public US Retail TradeSkillMaster CSV snapshots. No server code, spawns, reference tables or persistence are changed. The [bot economy spec](../../specs/bot-economy.md) owns requirements; runtime integration belongs to [[bot-economy]]. This page covers importer behavior only.

## Run

```sh
python3 scripts/economy_catalog.py --fetch --merchant 5565:0:165
python3 -m unittest discover -s scripts/tests -p test_economy_catalog.py -v
```

Default snapshots, config and report live in `data/economy/`. Offline imports reuse `items.csv` and `commodities.csv`; `--items`, `--commodities`, `--output`, `--report`, `--db` and `--snapshot-dir` accept explicit paths. Snapshots/config are generated data, not staged repository assets.

Repeat `--merchant entry:expansion:parentProfession` for each explicit business assignment. No assignment is inferred from a vendor's level-scaling `exp` field, goods or name. Merchant entries must have the vendor bit in `content_creature_template`, a `content_creature` spawn, and a parent `skill_line` ID. Invalid assignments produce a report and nonzero exit without writing config. Old output files are not replaced on failure; consumers must respect the exit status rather than load stale output.

The requested example `--merchant 3532:0:165` works in the concrete fixture. On the production database inspected September 30, 2026, Pyrewood Leatherworker 3532 has a vendor template and raw Wrath `creature` spawn, but no runtime `content_creature` spawn. It is rejected. Explicit replacement approved for the integration draft: Jillian Tanner 5565, expansion 0, Leatherworking parent 165. Jillian has vendor flag 129, spawn guid 20314239 on map 0, and actual leatherworking supplies including 2320/2321/2325. Godric Rothgar 1213 is also spawned (guid 20279954) but requires its own explicit assignment; names alone never select recipes.

## Market source

Official TSM public-data documentation publishes unauthenticated CSV endpoints. Importer uses:

- `https://public-data.tradeskillmaster.com/retail/us/region/items.csv`
- `https://public-data.tradeskillmaster.com/retail/us/commodities.csv`

The initially supplied `/retail/us/region/commodities.csv` returned 404. The documented commodity URL is not an alternate fallback: it is the sole commodity source. Requests identify the importer with a User-Agent; a headerless exploratory request returned 403. Four GET attempts maximum, 60-second request timeout, exponential delay plus jitter, numeric/date Retry-After honored; waits exceeding 120 seconds fail explicitly. Permanent HTTP failures are not retried. Fully read snapshots replace `.part` files under the persistent destination; `/tmp` downloads are rejected by the CLI.

Observed September 30, 2026 snapshot headers:

```text
items: itemId,name,marketValue,historical,avgSalePrice,saleRate,soldPerDay,updatedAt
commodities: itemId,name,marketValue,minBuyout,recent,historical,updatedAt
```

Region items contained 30,014 rows, timestamp `2026-09-30T08:19:11Z`; commodities contained 10,986 rows, timestamp `2026-09-30T15:23:54Z`. Both are public snapshots, not live transaction prices. All prices are copper.

Anchor is commodity `marketValue` when that item occurs in commodities, otherwise region `marketValue` for a noncommodity item. Invalid commodity prices reject the market, never substitute another source. Popularity is measured region `saleRate` multiplied by 10,000 and rounded half-up. Zero remains zero. `soldPerDay` must exist and be finite/nonnegative; it is not a second invented popularity weight. Commodity rows lacking a region popularity match are reported and excluded. There is no vendor-price fallback. `SellPrice` appears only in real runtime item metadata. Report `market_provenance` records each selected market's price source (`items` or `commodities`), raw `market_value`, `price_updated_at`, region `sale_rate`, `sold_per_day` and `popularity_updated_at`. Snapshot headers, row counts and timestamp sets remain separately recorded under `snapshots`; provenance never enters runtime config. Input-only auction markets retain original u64 prices, even above u32.

## Recipe and gathering boundary

Legacy recipes use difficulty-0 CREATE_ITEM effect 24 and `skill_line_ability` membership resolved through `skill_line.ParentSkillLineID`. Modern CRAFT_ITEM effect 288 instead references `crafting_data.ID` through `EffectMiscValue0`; parent profession and `SkillupSkillLineID` membership must both be inspected. [[midnight-economy-content]] owns the corrected source diagnosis and staging evidence; absence from SkillLineChild is not evidence of absent recipes. Expansion comes exclusively from the output's `item_sparse.ExpansionID`, including Midnight **11**, not 12 or `ParentTierIndex`. Selected merchant recipes must match both explicit expansion and parent profession.

Conservative supported slice: one nonzero effect, finite `EffectBasePointsF` with rounded output fitting u32, no variable scaling/variance coefficients, neutral group-size coefficient 1, positive concrete reagent slots, no `ReagentSource` or recraft counts. Multiple effects, nonfinite/overflow quantities, special/negative/zero-count reagents, absent reagents or missing output expansion are reported. Output count follows the existing `profession.rs` CREATE_ITEM handler's raw-point semantics: round raw points (half away from zero), then clamp to at least one; 1.5 produces two, zero/negative points produce one, and spell 2149 points 1 produces one boot, **not points + 1**. This is not a full Retail crafting simulator.

Every output and ingredient must exist in `content_item`. Outputs and auction-sourced ingredients require a supported TSM market; direct supplier ingredients do not require an auction quote. Merchant output `max_price` must fit the existing u32 `VendorItem.price`; exactly 4,294,967,295 copper is supported. Larger ranges exclude the recipe with `reason: merchant_output_price_exceeds_u32`, the unchanged `output_market` and `vendor_price_limit` in `excluded_recipes`. Its raw price source/value/timestamp remains in `market_provenance` even when the excluded output is absent from config. No clamping, fabricated prices or input-market narrowing occurs. Runtime validation separately rejects oversized configured merchant output quotes and popularity above 10,000; hand-written configs cannot bypass the protocol bound. Runtime `items` metadata comes directly from `content_item.Name/Quality/RequiredLevel/SellPrice`; invalid numeric ranges or missing names fail explicitly. No invented templates or names.

### Ingredient sourcing

`ingredient_sources` records each selected recipe ingredient's route, direct loot rows, profession producer spell IDs and vendor candidates. `gatherables` requires TradeGoods class 7, subclass cloth **5**, leather 6, metal/stone 7 or herb 9, plus a nonquest direct row in `content_creature_loot_template`, `content_gameobject_loot_template` or `content_item_loot_template` (`Reference=0`, nonnegative chance, positive valid counts). Zero chance remains supported because loot groups use zero for equal-chance entries. No subclass-only or reference-only evidence creates gathered inventory.

Profession-produced intermediates remain `crafted_auction`, even if a loot/vendor row exists. Explicit `RAW_CONVERSION_ITEMS` distinguishes raw conversion outputs from intermediates; currently Light Leather **2318** is the approved exception. Direct loot is still required. Spell **2881** does not negate leather's creature/gameobject/item-container sources. The exception is inspectable importer policy, not an inference that every crafted drop is raw: bolts, cured hides and other secondary goods are not minted by gatherers. Uncrafted proven raw resources remain `gathered_auction` even when a vendor also sells them. Other supported market inputs use `market_auction`, with an explicit reason when raw gathering is unproven.

Vendor-only ingredients use `vendor_supplier`, never AH-availability fallback. Select the lowest `(vendor_entry, runtime_slot)` among supported **unlimited** suppliers. Supplier must have vendor flag 128 and an active `content_creature.id1` spawn, using the runtime's inactive-event selection. Expand negative-item vendor references once in SQL `entry,slot,item` order, filter missing item templates and invalid stock/restock pairs, and truncate to 150 slots exactly as `MerchantCatalog` does. Config `slot` is the zero-based expanded runtime index; report `database_slot` and `source_entry` retain original row provenance. Only `ExtendedCost=0`, valid exact BuyPrice/VendorStackCount and stock fields are supported. Limited-only rows, extended costs, unspawned/nonvendor entries, invalid counts/prices/restock and catalog overflow are explicit `unsupported_vendor_rows`. A vendor-only ingredient with no supported supplier becomes `unsupported`; dependent recipes are excluded as `unsupported_ingredient_source`, not silently converted into market demand or fake gathering.

`vendor_supplies` maps item-ID strings to `{vendor_entry, slot, purchase_price, buy_count, max_count, restock_secs}`. **`purchase_price` is exact `content_item.BuyPrice` copper per bundle**, not a rounded per-item price; `buy_count` is exact positive `VendorStackCount`. For example 31 copper buys three items as one purchase. Stock `max_count` and `restock_secs` originate in `content_npc_vendor.maxcount/incrtime`; this cycle selects only zero/zero unlimited stock. These fields are the contract coordinated with main; runtime purchasing is implemented in `cd9b690` after importer `13241e0`. [[bot-economy]] owns exact live supplier/merchant receipts; this importer page does not independently certify runtime transitions.

## Provisional tuning and format

`--tuning path.json` overrides positive integer values below. Unknown keys, out-of-range values and bounds not bracketing the anchor fail. These defaults support integration tests; **none is established user economy policy**.

| Key | Provisional value |
|---|---:|
| cycle_secs | 60 |
| gatherer_listing_limit | 8 |
| gatherer_stack_max | 20 |
| gatherer_capital | 1,000,000 copper |
| customer_budget | 1,000,000 copper |
| customer_quantity | 5 |
| capital_limit | 1,000,000 copper per merchant |
| capacity | 100 per merchant |
| min_price_bps / max_price_bps | 8,000 / 12,000 |

Config top-level fields: `cycle_secs`, `gatherer_listing_limit`, `gatherer_stack_max`, `gatherer_capital`, `gatherables`, `vendor_supplies`, `customer_budget`, `customer_quantity`, `markets`, `items`, `merchants`. Market and metadata maps use JSON item-ID string keys. Market fields: `item_id`, `anchor_price`, `min_price`, `max_price`, `popularity_bps`. Metadata fields: `name`, `quality`, `required_level`, `vendor_sell_price`. Merchants contain `entry`, `expansion`, `capital_limit`, `capacity`, `recipes`. Recipes contain `spell_id`, `expansion`, `output_item_id`, `output_count`, `ingredients` as `[item_id,count]` pairs. Supplier contracts enter config; source evidence and unsupported-row reasons stay in the report. Each report assignment includes `eligible_recipe_count`; zero emits `warning: no_supported_recipes` and a stderr warning. Such assignments retain an empty recipe list, and unsupported goods are never fabricated. An entirely unsupported selection has empty markets, metadata and gatherables: a safe production no-op, not evidence of supported content.

## Production coverage (verified September 30, 2026)

| Boundary | Measured result | Proof |
|---|---|---|
| All mapped profession/output candidates (before vendor-price filtering) | 7,041; 2,600 eligible | Historical read-only production report; counts are membership candidates, not guaranteed unique spells |
| Classic Leatherworking vendor-price filtering, Lara 1295:0:165 | 223 supported recipes; one price exclusion: spell 10621/output 8345, anchor 5,302,341,052, max 6,362,809,263 copper | Read-only in-memory import of existing September 30 snapshots; saved `default-catalog-generation.json` now records successful default generation; bounded live startup/flows are linked below |
| Historical Midnight effect-24 slice | 115 output spells; six mapped Fishing candidates, zero eligible | Old report only; does not measure effect-288 Leatherworking |
| Corrected Midnight staging coverage | See [[midnight-economy-content]] | Modern membership, reference joins, published requirements and diagnostic catalog; publication pending |

The old claim of zero Leatherworking ability rows/missing Midnight recipe mappings was an importer/query error, not proof of missing source data. Effect-24-only coverage cannot establish modern crafting coverage. A genuine placed new Silvermoon merchant remains a separate source-content gap.

## Current acceptance boundary (October 1, 2026)

`default-catalog-generation.json` exit 0 writes `data/economy/config.json` and report: 311 markets, 27 gatherables, 223 recipes. Saved `catalog-current.json`/`catalog-current-report.json` explicitly assign Lara Moore 1295:0:165 (classic Leatherworking), with 11 supplier ingredients. This replaces the historical Jillian default (see `previous-default-config.json`), not the promised Midnight/new Silvermoon catalog. The active Classic catalog is unchanged. Corrected Midnight staging coverage and the remaining genuine city-NPC gap are recorded in [[midnight-economy-content]]; source publication remains pending. No fabricated or substitute content acceptance. [[auction-house]] owns final pinned CLI/native and pending-source-gate evidence; [[bot-economy]] owns cap-reconciliation limits.

Saved `catalog-current-report.json` under `/home/osso/.cache/economy-cli-20260930/a54a0ad0` records Lara Moore 1295:0:165 with one spawn and 223 eligible recipes. Its `output_spell_coverage["11"]` is exactly 115 CREATE_ITEM spells, six profession-mapped and 109 missing mappings; `coverage["11:356"]` has six candidates, zero eligible and six excluded. These historical effect-24 counts do not establish missing modern recipe data. [[midnight-economy-content]] supersedes that diagnosis; no explicit genuine new Silvermoon merchant assignment or publication acceptance is established.

Full-catalog empty stock led to the user-approved direct vendor-only sourcing choice, not invented thread gathering or guaranteed stock. The live acceptance catalog deliberately narrows actual recipe 2149 to shared capacity 3, disables customers and uses a two-second cycle. This is bounded test input, not production tuning or product availability policy. [[bot-economy]] owns costs and purchase proof; [[auction-house]] owns actual listing prices and latency, not TSM proxies.

## Sources

- [Importer](../../../scripts/economy_catalog.py) and [concrete SQLite/CSV/HTTP tests](../../../scripts/tests/test_economy_catalog.py).
- [Existing CREATE_ITEM quantity handler](../../../crates/server/src/profession.rs).
- [TradeSkillMaster public-data documentation](https://tradeskillmaster.com/public-data).
- `data/world.db`, inspected through SQLite `mode=ro`; timestamped snapshots and report under `data/economy/`.

## See Also

- [[bot-economy]] — runtime integration and acceptance gates.
- [[professions]] — source profession tables and player crafting.
- [[content-import]] — runtime content templates and spawn selection.
- [[midnight-economy-content]] — corrected modern source diagnosis and pending publication boundary.
- [[crafting-reference-import]] — separately owned seven-table schema/import interface.
