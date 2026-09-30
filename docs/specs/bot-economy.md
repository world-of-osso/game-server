# Bot economy

Gatherer bots supply auction-house materials; merchant bots buy ingredients, craft real recipes and sell finite stock through the merchant window. Server integration belongs in `crates/server/src/economy/`, auction-house and merchant handlers; reference-data import belongs in `scripts/`. See [system inventory](../wiki/systems/bot-economy.md). Requirements below are user-approved; implementation and end-to-end proof remain unverified.

## What it must do

### Auction supply and terms

- [ ] Gatherer bots sell materials through the auction house.
- [ ] Bot and player auctions offer durations of 1, 7 and 14 days, with identical fee and deposit rules for both actors.
- [ ] An unsold expired auction returns its item and refunds its deposit. No bot-only exemption or duplicate return/refund is permitted.

### Merchant production and inventory

- [ ] Merchants purchase actual ingredients and craft real ingredient recipes, recording actual ingredient expenditure attributable to the resulting stock.
- [ ] Crafted output is restricted to the merchant's configured expansion. Midnight expansion ID 11 and new Silvermoon are the requested example; unavailable current NPC spawns must be reported as a data gap, not silently substituted or claimed available.
- [ ] Prices vary within configured bounds. A data-driven configurable core and importer supply recipes, output-expansion eligibility and market policy; fixed examples must not substitute for real reference data.
- [ ] All products compete for finite shared merchant inventory capacity. No per-product reserved stock or guaranteed availability is allowed.
- [ ] Players purchase that stock through the merchant window; player and simulated purchases consume the same finite supply.
- [ ] Simulated customers buy and consume stock, bounded by quantity and gold budgets, with popularity-weighted selection.
- [ ] Unsold stock expires after 24 hours. Refund only the actual recorded ingredient expenditure attributable to the remaining stock, never sale price or prospective profit; sold units cannot also receive expiry refunds.
- [ ] Merchant capital is capped; overflow is withdrawn rather than available for further spending.

### Durability and acceptance

- [ ] Ingredient purchases, crafting, stock sales, expiry refunds and capital withdrawals persist transactionally. Restart preserves stock, costs, balances and deadlines without lost or duplicated effects.
- [ ] First prove the economy through `game-cli`, adding necessary actions, before engine acceptance. Exercise actual auctioneer/vendor interactions and persisted transactions, not only pure model helpers.
- [ ] Then prove the fully working auction house in the engine, including browse/search/paging, create, bid, buyout, cancel and resulting inventory/gold/mail state. [Auction house contract](auction-house.md) defines global grouped browse through the new distinct endpoint, ItemID drilldown and real AuctionID selection; existing flat shapes remain unchanged.
- [ ] Auction queries complete in less than 200 ms. Record request/response measurement boundary, population, query and paging workload, revision and results; model-only timing is not client acceptance proof.

Only the durations, stock lifetime and query limit above are approved numerical policy. Price bounds, capacity, customer budgets, popularity weights, capital caps and simulation cadence require explicit configuration; example/test values are not approved tuning.

## How it works

- [Bot economy system inventory](../wiki/systems/bot-economy.md)
- [Global browse and auction selection](../wiki/systems/auction-house.md)
- [Merchant flow](../wiki/systems/merchant.md)
- [Profession recipe data and crafting](../wiki/systems/professions.md)
- [Persistence](../wiki/systems/persistence.md)

## Implementation inventory

Observed integration points, not a claim that the bot pipeline is connected:

- `crates/server/src/economy/model.rs` — pure merchant stock, ingredient-cost accounting, price bounds, consumers and capital helpers.
- `crates/server/src/auction_house/operations.rs` — auction creation, deposits and sale operations.
- `crates/server/src/auction_house/queries.rs` — filter, sort and page listings.
- `crates/server/src/merchant.rs`, `merchant/catalog.rs`, `merchant/stock.rs` — existing vendor requests, catalog and stock.
- `crates/server/src/profession/data.rs` — existing profession/recipe/reagent reference loading.
- `crates/client/src/bin/game_cli.rs` — headless client acceptance surface; concurrent additions require separate verification.
- `scripts/` — existing reference import integration area; economy importer path and coverage not established by this audit.

## Tests asserting this spec

Candidate coverage only; no commands were run in this docs-only audit:

- `crates/server/src/economy/model_tests.rs` — model tests include shared capacity, recorded-cost expiry and crafting; not persistence, live protocol or importer proof.
- `crates/server/src/auction_house_tests.rs` — auction tests; concurrent edits are not accepted proof until executed against their exact revision.
- Live CLI/restart and engine acceptance artifacts — not supplied.

## Known gaps (current cycle)

- [ ] Connect and prove configurable importer/reference data, bot auction supply and merchant transactions.
- [ ] Establish real expansion-11 recipe/output data and eligible merchant/NPC spawn availability.
- [ ] Prove equal auction terms and unsold deposit refund for bot and player paths.
- [ ] Prove restart/crash-boundary accounting, CLI-first flow, engine auction flow and query latency. Grouped performance remains pending; reported optimized flat 300k-listing/144 ms helper timing is not request/response acceptance.

## Out of scope

No additional economy policy is approved here. This document does not authorize invented numerical tuning, fabricated recipes/NPC spawns, reserved product availability or a substitute for CLI-before-engine verification.
