# Bot economy

Current source inspected October 1, 2026 at `8193d7e`, including server/shared integration `b0153e4`, importer `13241e0` and supplier runtime `cd9b690`. The [tracked spec](../../specs/bot-economy.md) owns approved behavior. Bounded live CLI proof exists; native AH/mail source is implemented but live engine proof is IN PROGRESS.

## Observed model

`crates/server/src/economy/model.rs` contains `MerchantState` with gold and a vector of `StockBatch` records. Each batch records remaining quantity, ingredient expenditure, unit price and expiry. `craft_stock` validates recipe expansion and configured membership, checks shared quantity capacity and affordability, and records stock. `sell_stock` removes sold quantity and its proportional ingredient cost, retaining rounding remainder in unsold stock. `expire_stock` removes expired batches and returns remaining ingredient cost; callers must credit and persist the refund. `credit_capital` credits available headroom and returns overflow for caller accounting. It does not reduce an already-over-limit wallet, so maintaining the cap across configuration/restart boundaries requires integration proof.

`quote_price` maps a supplied draw into price bounds. `consume_stock` applies quantity/budget limits to supplied draws and removes purchased units from a cloned state before publishing the result. These helpers do not themselves purchase auction ingredients, commit storage transactions, schedule cycles or serve vendor requests. Their existence cannot establish those behaviors.

## Integration boundaries

- Auction operations: `auction_house/operations.rs` creates listings and charges deposits; `auction_house/queries.rs` filters, sorts and pages flat listings. [[auction-house]] owns grouped browse → ItemID → flat real AuctionID and the saved actual request-response timing ledger, separate from helper benchmarks.
- Vendor requests: [[merchant]] documents the current catalog, limited vendor stock and player inventory persistence. `merchant/economic.rs` serves finite batches for assigned merchants and atomically commits player inventory, merchant stock and capped proceeds. Saved live proof below exercises this boundary; ordinary vendor restocking is not bot-stock evidence.
- Recipes: [[professions]] identifies `skill_line_ability`, `spell_reagents` and spell `CREATE_ITEM` data. These are useful integration sources, not proof of complete expansion-11 recipes/output eligibility or new Silvermoon merchant spawns.
- Persistence: [[persistence]] describes redb mutable storage and SQLite reference data. Model serialization is not atomic transaction/restart proof.
- Client: `crates/client/src/bin/game_cli.rs` is the headless protocol surface. Live CLI proof precedes engine acceptance; native auction UI documentation remains separately maintained in game-engine and untouched by this update.

## Direct supplier procurement

`Config.vendor_supplies` maps ingredient ItemID to a `VendorSupply` row imported from actual spawned suppliers; [[economy-catalog]] owns that provenance. `purchase_price` is the full copper price per `buy_count` bundle, not an auction unit quote. The runtime rejects limited/restocking rows, nonzero extended costs, invalid bundles and supplier/gatherable overlap before cycle work.

`economy/cycle.rs` selects configured supplier sourcing exclusively, including when an auction is cheaper or the supplier is unaffordable. It buys only whole bundles covering the recipe deficit within available capital. Output capacity is checked before procurement. Purchases become paid `MaterialLot` records; `craft_from_materials` consumes proportional costs and retains surplus quantity and rounding cost. Supplier payments, leftover lots, crafted stock, refunds and the cycle timestamp use the existing single `commit_market` transaction before publishing runtime state. No supplier stock/restocking framework is introduced.

Supplier tests in `economy/tests.rs` cover mixed auction/supplier costs, capital refusal, exclusive source selection, full output capacity, bundle leftovers through database reopen, exact capped refunds, unsupported supplier rejection and failure before market commit. These are bounded runtime tests, not live CLI/native acceptance or crash-injection proof.

## Read-only admin diagnosis

`game-server-admin economy-status` uses the existing same-UID admin socket and prints a JSON snapshot of initialized in-memory state, not a fresh database read. `last_cycle` is the stored Unix-seconds cycle timestamp. Each merchant includes entry, gold, configured capital cap, stock batches (remaining ingredient cost, quantity, price and expiry) and paid material lots (remaining quantity and cost). A persisted merchant absent from current config has a null cap. Cumulative `customer_spent`, `refund_issued` and `withdrawn` retain model field names; money is in copper. `gatherers` maps item IDs to seller character IDs; `gatherer_count` counts those actors. Auction listing totals are not included.

Missing/uninitialized Economy returns `economy not initialized`; missing Config returns `economy config not loaded`. The snapshot handler borrows both resources read-only, never runs a cycle, changes the clock or persists data. At code revision `944df30`, `cargo test -p server --bins economy_status -- --nocapture` passed all three targeted tests (exact values/unchanged state, missing initialization, CLI parsing/decoding). Live server diagnosis remains main-owned.

## Current capability and proof ledger

Saved evidence root: `/home/osso/.cache/economy-cli-20260930/a54a0ad0`. [[auction-house]] owns exact auction action/restart/mail/performance receipts and revision limitations. No tests or live operations were rerun for this docs update.

| Capability | Exact receipt | Proof / limits |
|---|---|---|
| Gatherer supply and atomic cycle | `supplier-live-status-start.json`, `merchant-produced.json`; inspected `cycle.rs`/`store.rs` | Live gatherer 2318 and paid lots/stock; central commit precedes state publication |
| Direct supplier and actual recipe | `merchant-proof-catalog.json`, `merchant-produced.json` | Lara Moore 1295, classic expansion 0, recipe 2149: two Light Leather 2318 plus one Coarse Thread 2320 produce one boot 2302. Supplier entry 66/runtime slot 8 sells thread for exactly 10 copper per one-item bundle |
| Exact recorded costs and finite stock | `merchant-produced.json` | Three one-boot batches: ingredient costs 16160/14222/16448 copper; quotes 326982/345551/356166. Leather remainder one item/cost 8430; merchant gold 944740, cumulative withdrawn 36333 |
| Actual player merchant purchase | `cli-merchant-list-proof.json`, `cli-merchant-buy-proof.json`, `merchant-after-player-buy.json` | CLI bought slot 1 at actual price 326982, inventory confirms boot. Only batches 2/3 remain; merchant gold capped at 1000000, cumulative withdrawn 324102. Increase 287769 equals 944740 + 326982 - 1000000 |
| Auction CLI/restart/mail and sampled latency | [[auction-house]] | Saved request/response and normal restart proof; not crash injection or native acceptance |
| Simulated consumption, expiry and failure boundaries | `model_tests.rs`, `economy/tests.rs`, `merchant/economic_tests.rs` source | Behavioral tests exist; no fresh execution receipt established here for complete current-revision coverage |
| Modern content and native acceptance | [[economy-catalog]] and [[auction-house]] | Midnight/new Silvermoon content gap; live engine proof IN PROGRESS |

The merchant catalog deliberately restricts real recipe 2149, shared capacity 3, customer budget/quantity 0 and cycle 2 seconds to bound the flow. These are test-fixture restrictions, not production policy, reserved product slots or guaranteed boots availability. Full catalog empty stock was diagnosed as a sourcing boundary; user explicitly chose direct vendor-only supply rather than fabricated gatherer thread or an AH fallback. Importer/runtime commits `13241e0`/`cd9b690` implement that choice. Full-catalog product availability remains unproven and is not guaranteed.

The normal auction restart proves the recorded buyer/seller state only. It does not establish restart/crash-boundary coverage for every merchant procurement, expiry, customer or withdrawal transition. Saved build receipts are not broad-suite acceptance, and the artifact directory label is not an exact revision stamp for every run. Canonical master was restored and server work remains isolated (user-reported state, not changed or recertified here).

## Sources

- [Bot economy spec](../../specs/bot-economy.md) — approved contract and acceptance gaps.
- [Auction house spec](../../specs/auction-house.md) — global browse, flat selection and CLI-first latency contract.
- [Economy snapshot](../../../crates/server/src/economy/mod.rs) and [admin dispatch](../../../crates/server/src/admin_ipc.rs) — read-only diagnosis.
- [Economy cycle](../../../crates/server/src/economy/cycle.rs) and [supplier tests](../../../crates/server/src/economy/tests.rs) — direct sourcing and bounded persistence boundaries.
- [Economy model](../../../crates/server/src/economy/model.rs) — inspected accounting helpers.
- [Model tests](../../../crates/server/src/economy/model_tests.rs) — candidate model coverage, not executed here.
- [Auction queries](../../../crates/server/src/auction_house/queries.rs) — inspected search/paging boundary.
- [Auction operations](../../../crates/server/src/auction_house/operations.rs) — deposit/creation integration.

## See Also

- [[auction-house]] — grouped API and exact CLI/restart/mail/sampled-performance ledger; native proof pending.
- [[economy-catalog]] — importer and measured source-price provenance, not live auction listing prices.
- [[merchant]] — current player purchase boundary.
- [[professions]] — recipe/reagent reference sources.
- [[persistence]] — durable transaction integration.
