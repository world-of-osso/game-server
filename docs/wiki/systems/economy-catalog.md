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

Anchor is commodity `marketValue` when that item occurs in commodities, otherwise region `marketValue` for a noncommodity item. Invalid commodity prices reject the market, never substitute another source. Popularity is measured region `saleRate` multiplied by 10,000 and rounded half-up. Zero remains zero. `soldPerDay` must exist and be finite/nonnegative; it is not a second invented popularity weight. Commodity rows lacking a region popularity match are reported and excluded. There is no vendor-price fallback. `SellPrice` appears only in real runtime item metadata. Report `market_provenance` records each selected market's price source (`items` or `commodities`), raw `market_value`, `price_updated_at`, region `sale_rate`, `sold_per_day` and `popularity_updated_at`. Snapshot headers, row counts and timestamp sets remain separately recorded under `snapshots`; provenance never enters runtime config.

## Recipe and gathering boundary

Recipes require difficulty-0 CREATE_ITEM effect 24 and `skill_line_ability` membership resolved through `skill_line.ParentSkillLineID`. Expansion comes exclusively from the output's `item_sparse.ExpansionID`, including Midnight **11**, not 12 or `ParentTierIndex`. Selected merchant recipes must match both explicit expansion and parent profession.

Conservative supported slice: one nonzero effect, finite `EffectBasePointsF` with rounded output fitting u32, no variable scaling/variance coefficients, neutral group-size coefficient 1, positive concrete reagent slots, no `ReagentSource` or recraft counts. Multiple effects, nonfinite/overflow quantities, special/negative/zero-count reagents, absent reagents or missing output expansion are reported. Output count follows the existing `profession.rs` CREATE_ITEM handler's raw-point semantics: round raw points (half away from zero), then clamp to at least one; 1.5 produces two, zero/negative points produce one, and spell 2149 points 1 produces one boot, **not points + 1**. This is not a full Retail crafting simulator.

Every output and ingredient must exist in `content_item` and a supported TSM market. Runtime `items` metadata comes directly from `content_item.Name/Quality/RequiredLevel/SellPrice`; invalid numeric ranges or missing names fail explicitly. No invented templates or names.

`gatherables` includes only ingredients actually used by assigned recipes whose `content_item.ClassID` is TradeGoods 7 and subclass is cloth 3, leather 6, metal/stone 7, or herb 9. Any output of a CREATE_ITEM/CREATE_ITEM2 (24/157) ability is excluded globally, even if that producer itself cannot be imported. This deliberately excludes conversion-produced materials such as some leathers: classification must not mint secondary crafted goods. Report separates `secondary_crafting_item_ids` and `vendor_or_other_supply_item_ids`; importer does not supply those inventories or prove vendor reachability for each reagent.

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

Config top-level fields: `cycle_secs`, `gatherer_listing_limit`, `gatherer_stack_max`, `gatherer_capital`, `gatherables`, `customer_budget`, `customer_quantity`, `markets`, `items`, `merchants`. Market and metadata maps use JSON item-ID string keys. Market fields: `item_id`, `anchor_price`, `min_price`, `max_price`, `popularity_bps`. Metadata fields: `name`, `quality`, `required_level`, `vendor_sell_price`. Merchants contain `entry`, `expansion`, `capital_limit`, `capacity`, `recipes`. Recipes contain `spell_id`, `expansion`, `output_item_id`, `output_count`, `ingredients` as `[item_id,count]` pairs. No report fields enter runtime config. Each report assignment includes `eligible_recipe_count`; zero emits `warning: no_supported_recipes` and a stderr warning. Such assignments retain an empty recipe list, and unsupported goods are never fabricated. An entirely unsupported selection has empty markets, metadata and gatherables: a safe production no-op, not evidence of supported content.

## Production coverage (verified September 30, 2026)

| Boundary | Measured result | Proof |
|---|---|---|
| All mapped profession/output candidates | 7,041; 2,600 eligible | Read-only production import report; counts are membership candidates, not guaranteed unique spells |
| Explicit Jillian 5565:0:165 | 224 recipes, 312 markets and metadata rows, 33 gatherables | Existing generated config and read-only in-memory importer inspection agree; runtime acceptance separate |
| Midnight effect-24 output spells | 115; 6 profession-mapped; 109 missing ability mappings | `output_spell_coverage["11"]` and `unmapped_output_spells` |
| Midnight mapped recipes | 6 Fishing candidates, zero eligible; every one lacks a content item | Per-recipe exclusion report |
| Midnight Leatherworking | Zero ability rows for skill line 2915 | Read-only SQLite query, not an invented alternate mapping |

There are no expansion-11 effect-157 outputs in this DB; additional output-bearing effects 169 (24 spells) and 296 (18 spells) are outside the existing profession CREATE_ITEM handler and this importer slice. Broader coverage needs actual profession ability data, content templates and explicit spawned vendor/expansion/profession assignments. Reassigning Jillian to Midnight would not fill missing source data. No claim of new Silvermoon merchant availability is made.

## Sources

- [Importer](../../../scripts/economy_catalog.py) and [concrete SQLite/CSV/HTTP tests](../../../scripts/tests/test_economy_catalog.py).
- [Existing CREATE_ITEM quantity handler](../../../crates/server/src/profession.rs).
- [TradeSkillMaster public-data documentation](https://tradeskillmaster.com/public-data).
- `data/world.db`, inspected through SQLite `mode=ro`; timestamped snapshots and report under `data/economy/`.

## See Also

- [[bot-economy]] — runtime integration and acceptance gates.
- [[professions]] — source profession tables and player crafting.
- [[content-import]] — runtime content templates and spawn selection.
