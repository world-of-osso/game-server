# Bot economy

Gatherer bots supply auction-house materials; merchant bots buy ingredients, craft real recipes and sell finite stock through the merchant window. Server integration belongs in `crates/server/src/economy/`, auction-house and merchant handlers; reference-data import belongs in `scripts/`. See [system inventory](../wiki/systems/bot-economy.md). Requirements below are user-approved. Current source includes server/shared integration `b0153e4`, importer `13241e0` and supplier runtime `cd9b690`; bounded CLI proof is recorded in the wiki, and bounded native audit PASS is linked there. Fresh source checker agent52 remains pending; main decides full-goal readiness.

## What it must do

### Auction supply and terms

- [ ] Gatherer bots sell materials through the auction house.
- [ ] Bot and player auctions offer durations of 1, 7 and 14 days, with identical fee and deposit rules for both actors.
- [ ] An unsold expired auction returns its item and refunds its deposit. No bot-only exemption or duplicate return/refund is permitted.

### Merchant production and inventory

- [ ] Merchants purchase actual ingredients and craft real ingredient recipes, recording actual ingredient expenditure attributable to the resulting stock. Configured vendor-only ingredients are purchased directly from actual unlimited suppliers at their copper bundle price, never substituted with auction offers. Paid bundle leftovers persist for later crafting. Limited-stock and extended-cost suppliers are explicitly unsupported; genuine gathering resources remain auction-sourced.
- [ ] Crafted output is restricted to the merchant's configured expansion. Midnight expansion ID 11 and new Silvermoon are the requested example; unavailable current NPC spawns must be reported as a data gap, not silently substituted or claimed available.
- [ ] Prices vary within configured bounds. A data-driven configurable core and importer supply recipes, output-expansion eligibility and market policy; fixed examples must not substitute for real reference data.
- [ ] All products compete for finite shared merchant inventory capacity. No per-product reserved stock or guaranteed availability is allowed.
- [ ] Players purchase that stock through the merchant window; player and simulated purchases consume the same finite supply.
- [ ] Simulated customers buy and consume stock, bounded by quantity and gold budgets, with popularity-weighted selection.
- [ ] Unsold stock expires after 24 hours. Refund only the actual recorded ingredient expenditure attributable to the remaining stock, never sale price or prospective profit; sold units cannot also receive expiry refunds.
- [ ] Merchant capital is capped; overflow is withdrawn rather than available for further spending.

### Read-only diagnosis

- [x] `game-server-admin economy-status` reports last cycle, each merchant entry/gold/capital cap, stock batches with remaining ingredient cost, paid material lots, cumulative customer spending/refunds/withdrawals and gatherer actor totals through existing same-UID admin IPC. Missing initialization is an explicit error. No economic mutation, clock stepping or public protocol changes.

### Durability and acceptance

- [x] Bounded real supplier fixture produces three boots with recorded ingredient costs and permits a CLI purchase from the same finite stock, crediting capped merchant proceeds and recording excess. Exact fixture and receipts: [capability ledger](../wiki/systems/bot-economy.md). This does not certify all production recipes or policies.
- [x] Saved CLI auction flow posts short/medium/long, bids and buys out 107, cancels 108, rejects own buyout/underbid/out-of-range, and preserves observed buyer/seller balances through normal restart and mail claims. Exact proof: [auction ledger](../wiki/systems/auction-house.md).
- [x] Twelve pinned grouped/flat CLI samples across six workloads/two cycles are below 200 ms on one owned loopback server with live flat total 300068 offers/311 distinct items. Exact timings/population: [performance ledger](../wiki/systems/auction-house.md). No universal concurrency SLA is implied.

- [ ] Ingredient purchases, crafting, stock sales, expiry refunds and capital withdrawals persist transactionally. Restart preserves stock, costs, balances and deadlines without lost or duplicated effects.
- [ ] First prove the economy through `game-cli`, adding necessary actions, before engine acceptance. Exercise actual auctioneer/vendor interactions and persisted transactions, not only pure model helpers.
- [x] Bounded native auction/mail sequences pass the saved seller/buyer/large/mail runs and independent agent51 audit; [native receipt links and limits](../wiki/systems/auction-house.md#bounded-native-audit) record browse/search/paging, create, bid, buyout, cancel and resulting inventory/gold/mail state. This is bounded acceptance, not full conversion or clean assets. [Auction house contract](auction-house.md) defines global grouped browse through the new distinct endpoint, ItemID drilldown and real AuctionID selection; existing flat shapes remain unchanged.
- [ ] Auction queries complete in less than 200 ms. Record request/response measurement boundary, population, query and paging workload, revision and results; model-only timing is not client acceptance proof.

Only the durations, stock lifetime and query limit above are approved numerical policy. Price bounds, capacity, customer budgets, popularity weights, capital caps and simulation cadence require explicit configuration; example/test values are not approved tuning.

## How it works

- [Bot economy system inventory](../wiki/systems/bot-economy.md)
- [Global browse and auction selection](../wiki/systems/auction-house.md)
- [Merchant flow](../wiki/systems/merchant.md)
- [Profession recipe data and crafting](../wiki/systems/professions.md)
- [Persistence](../wiki/systems/persistence.md)

## Implementation inventory

Current implementation inventory (source inspection is distinct from acceptance proof):

- `crates/server/src/economy/model.rs` — pure merchant stock, ingredient-cost accounting, price bounds, consumers and capital helpers.
- `crates/server/src/auction_house/operations.rs` — auction creation, deposits and sale operations.
- `crates/server/src/auction_house/queries.rs` — filter, sort and page listings.
- `crates/server/src/merchant.rs`, `merchant/catalog.rs`, `merchant/stock.rs` — existing vendor requests, catalog and stock.
- `crates/server/src/profession/data.rs` — existing profession/recipe/reagent reference loading.
- `crates/client/src/bin/game_cli.rs` — headless client acceptance surface; concurrent additions require separate verification.
- `scripts/economy_catalog.py` — real recipe/output eligibility, loot/supplier sourcing and timestamped market provenance; reports unsupported content.
- `crates/server/src/economy/mod.rs`, `cycle.rs`, `store.rs` — configuration, scheduled gatherer/procurement/crafting/customer/expiry cycles and atomic market persistence.
- `crates/server/src/merchant/economic.rs` — finite stock listing and player purchase with capped proceeds.

## Tests asserting this spec

Read-only diagnosis tests: `crates/server/src/economy/tests.rs` asserts exact concrete state and missing initialization; `crates/server/src/bin/admin.rs` asserts command parsing and response decoding.

Additional behavioral source and saved proof (no suites rerun for this docs-only update):

- `crates/server/src/economy/model_tests.rs` — model tests include shared capacity, recorded-cost expiry and crafting; not persistence, live protocol or importer proof.
- `crates/server/src/auction_house_tests.rs` — auction tests; concurrent edits are not accepted proof until executed against their exact revision.
- Saved JSON receipts under `/home/osso/.cache/economy-cli-20260930/a54a0ad0` — exact [merchant](../wiki/systems/bot-economy.md) and [auction](../wiki/systems/auction-house.md) ledgers; checked bullets above cover only these bounded observations. Bounded native audit PASS is linked there; final fresh source gate remains pending.

## Known gaps (current cycle)

- [ ] Extend bounded importer/runtime/CLI proof to full-catalog merchant behavior, simulated customers and expiry; do not infer guaranteed products from the restricted fixture.
- [ ] Establish real expansion-11 recipe/output data and eligible merchant/NPC spawn availability.
- [ ] Prove equal auction terms and unsold deposit refund for bot and player paths.
- [ ] Prove merchant restart/crash-boundary accounting beyond the scoped receipts. Native AH/mail and pinned CLI latency pass only their recorded bounded sequences; no unconditional full-goal or concurrency claim.
- [ ] Reconcile already-over-cap wallets when configuration lowers the cap. Credits are capped; existing over-cap balances are not reduced.
- [ ] Await fresh source checker agent52; main owns all-source/full-goal-ready decision.

## Out of scope

No additional economy policy is approved here. This document does not authorize invented numerical tuning, fabricated recipes/NPC spawns, reserved product availability or a substitute for CLI-before-engine verification.
