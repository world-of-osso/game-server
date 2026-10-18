# Bot economy

Gatherer bots supply auction-house materials; merchant bots buy ingredients, craft real recipes and sell finite stock through the merchant window. Server integration belongs in `crates/server/src/economy/`, auction-house and merchant handlers; reference-data import belongs in `scripts/`. See [system inventory](../wiki/systems/bot-economy.md). Contracts follow the agreed design; implementation limits and provisional tuning are explicit below. Server/shared integration `b0153e4`, supplier runtime `cd9b690`, subsequent readability/query fixes and actor-parity tests `2ec5fb3` have revision-scoped proof. The [system ledger](../wiki/systems/bot-economy.md) links independent server, CLI/restart and native audits.

## What it must do

### Auction supply and terms

- [x] Gatherer bots sell materials through the auction house.
- [x] Bot and player auctions offer durations of 1, 7 and 14 days, with identical fee and deposit rules for both actors.
- [x] An unsold expired auction returns its item and refunds its deposit. No bot-only exemption or duplicate return/refund is permitted.

### Merchant production and inventory

- [x] Merchants purchase actual ingredients and craft real ingredient recipes, recording actual ingredient expenditure attributable to the resulting stock. Configured vendor-only ingredients are purchased directly from actual unlimited suppliers at their copper bundle price, never substituted with auction offers. Paid bundle leftovers persist for later crafting. Limited-stock and extended-cost suppliers are explicitly unsupported; genuine gathering resources remain auction-sourced.
- [x] Crafted output is restricted to the merchant's configured expansion. Midnight expansion ID 11 and new Silvermoon are the requested example; unavailable current NPC spawns must be reported as a data gap, not silently substituted or claimed available.
- [x] Prices vary within configured bounds. A data-driven configurable core and importer supply recipes, output-expansion eligibility and market policy; fixed examples must not substitute for real reference data.
- [x] All products compete for finite shared merchant inventory capacity. No per-product reserved stock or guaranteed availability is allowed.
- [x] Players purchase that stock through the merchant window; player and simulated purchases consume the same finite supply.
- [x] Simulated customers buy and consume stock, bounded by quantity and gold budgets, with popularity-weighted selection.
- [x] Unsold stock expires after 24 hours. Refund only the actual recorded ingredient expenditure attributable to the remaining stock, never sale price or prospective profit; sold units cannot also receive expiry refunds.
- [x] Merchant sale, customer and refund credits respect the configured capital cap; overflow is withdrawn rather than available for further spending. Lowering a cap below an existing persisted wallet remains a configuration-reconciliation limitation.

### Read-only diagnosis

- [x] `game-server-admin economy-status` reports last cycle, each merchant entry/gold/capital cap, stock batches with remaining ingredient cost, paid material lots, cumulative customer spending/refunds/withdrawals and gatherer actor totals through existing same-UID admin IPC. Missing initialization is an explicit error. No economic mutation, clock stepping or public protocol changes.

### Durability and acceptance

Scheduled cycles must not block fixed ticks on durable IO. ID ranges are reserved on the existing FIFO writer, with committed high-water marks before any reserved ID is used; restart may waste reserved IDs but must never reuse them. The prepared market transaction checks its expected economy, auction, inventory and replaced-mailbox inputs; concurrent auction payout or expiry mail must reject a stale cycle, never be overwritten or merged away. Market state, auction deltas and criteria publish only after its durable receipt, a few ticks after preparation rather than in the same tick. A failed/conflicting cycle warns with cycle and reason, publishes nothing, releases pending requests, and permits the next scheduled cycle without a retry loop.

While a market commit is pending, only economic-vendor requests are deferred. Requests survive transport inbox clearing in owned per-connection/type FIFO queues and retain the existing request-system order. Both successful and failed receipts release them against the current published state; normal vendors continue unaffected. No merchant actor conversion or network timeout relaxation is implied.

- [x] Bounded real supplier fixture produces three boots with recorded ingredient costs and permits a CLI purchase from the same finite stock, crediting capped merchant proceeds and recording excess. Exact fixture and receipts: [capability ledger](../wiki/systems/bot-economy.md). This does not certify all production recipes or policies.
- [x] Saved CLI auction flow posts short/medium/long, bids and buys out 107, cancels 108, rejects own buyout/underbid/out-of-range, and preserves observed buyer/seller balances through normal restart and mail claims. Exact proof: [auction ledger](../wiki/systems/auction-house.md).
- [x] Twelve pinned grouped/flat CLI samples across six workloads/two cycles are below 200 ms on one owned loopback server with live flat total 300068 offers/311 distinct items. Exact timings/population: [performance ledger](../wiki/systems/auction-house.md). No universal concurrency SLA is implied.

- [x] Ingredient purchases, crafting, stock sales, expiry refunds and capital withdrawals persist transactionally. Restart preserves stock, costs, balances and deadlines without lost or duplicated effects.
- [x] First prove the economy through `game-cli`, adding necessary actions, before engine acceptance. Exercise actual auctioneer/vendor interactions and persisted transactions, not only pure model helpers.
- [x] Bounded native auction/mail sequences pass the saved seller/buyer/large/mail runs and independent agent51 audit; [native receipt links and limits](../wiki/systems/auction-house.md#bounded-native-audit) record browse/search/paging, create, bid, buyout, cancel and resulting inventory/gold/mail state. This is bounded acceptance, not full conversion or clean assets. [Auction house contract](auction-house.md) defines global grouped browse through the new distinct endpoint, ItemID drilldown and real AuctionID selection; existing flat shapes remain unchanged.
- [x] Auction queries complete in less than 200 ms at the recorded loopback acceptance workloads. Record request/response measurement boundary, population, query and paging workload, revision and results; model-only timing is not client acceptance proof.

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
- `crates/server/src/auction_house_tests.rs` and `auction_house_tests/actor_parity.rs` — real gatherer/player creation, buyout, expiry, claims and redb reopen across all three durations. `2ec5fb3`: two actor-parity tests pass; deposits 60/120/240, fee 50 on a 1000-copper sale, unsold item/deposit returned once.
- Saved JSON receipts under `/home/osso/.cache/economy-cli-20260930/a54a0ad0` — exact [merchant](../wiki/systems/bot-economy.md) and [auction](../wiki/systems/auction-house.md) ledgers; checked bullets above cover only these bounded observations. Independent final delta PASS is recorded in `/tmp/economy-delta-verify-20261001/report.md`; native audit scope and retained content errors are linked there.

- [x] Former merchant source-data gap resolved locally: genuine Zaralda with approved approximate ground placement, authentic native identity and explicitly derived merchant role; no diagnostic NPC substitution. Combined default preserves Classic data and adds Midnight content; scoped catalog/CLI/restart artifact gate103 PASS. [Evidence and exact limits](../wiki/investigations/midnight-economy-content.md).
- [x] Native Zaralda observed flow: actual pick/Buy cursor, title, authored244586 stock1 tooltip, backpack and close; [saved proof](../wiki/investigations/midnight-economy-content.md#observed-native-success-task-evidence-2026-10-01). Not blanket world/recipe acceptance.

## Known gaps (current cycle)

- [ ] Support currently unresolved Spark requirements, quality-only outputs and native bonus/quality cases before claiming full Retail crafting coverage. Published reagent pages are not native C_TradeSkillUI captures.
- [ ] Existing over-cap wallets are not reduced when an operator lowers the cap. Normal credits are capped.

Acceptance covers the recorded configurations, atomic failure/reopen tests and actual CLI/native workflows—not every recipe, arbitrary crash timing, WAN/concurrent latency, or guaranteed product availability. No production deployment was performed.

## Out of scope

No additional economy policy is approved here. This document does not authorize invented numerical tuning, fabricated recipes/NPC spawns, reserved product availability or a substitute for CLI-before-engine verification.
