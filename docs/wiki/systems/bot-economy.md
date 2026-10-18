# Bot economy

Current source inspected October 1, 2026 at `8193d7e`, including server/shared integration `b0153e4`, importer `13241e0` and supplier runtime `cd9b690`. The [tracked spec](../../specs/bot-economy.md) owns approved behavior. Bounded live CLI and native AH/mail audit PASS exist. [[auction-house]] owns the pinned final CLI, native links and historical/focused source receipts; the final delta and actor-parity gates passed; the returned-goods follow-up has focused regression proof.

## FIFO cycle publication

A private current-master capture (`ce0fec81c`, protocol`11a058cf`) found main TID380605 in `fdatasync` through `persistence::next_id` (`persistence.rs:1088`) from `economy::cycle::take_or_gather_resource` (`cycle.rs:211` before this change), reached from FixedMain `update_economy`. The existing persistence writer was separately waiting for jobs. Evidence: `game-engine/data/diagnostics/server-live-frame-cost-2026-10-16/blocking-io-ce0-owned-pressure2-stack-05.log`. The owned bounded scratch writer generated16GiB total in a1GiB file; no canonical data or UID1001 processes were changed. All eleven peers completed; the prior3.246s timeout failure was not deterministically repeated, and stack attachment perturbs timing. This capture proves the synchronous caller, not a performance improvement.

`economy/ids.rs` reserves bounded per-cycle character/item/auction/mail ranges on the existing FIFO. A reservation receipt exposes IDs only after durable high-water marks. `cycle.rs` prepares state/row changes using those IDs, including auction settlement mail; `publication.rs` submits atomic market persistence and publishes only its successful receipt. Existing auction/inventory checks plus the expected economy and replaced-mailbox rows reject stale inputs. A real auction buyout between mailbox snapshot and queued commit reproduced lost11-copper seller mail; the expected-mailbox check rejects that cycle without merging or patching rows, preserving proceeds for the next scheduled collection. Failure warns with cycle and reason, clears pending state and waits for the next configured cycle, not a retry loop.

Merchant request systems own deferred decoded requests because Lightyear clears unread transport inboxes each frame. Only economic vendors wait during market commit; per-connection/type FIFO and existing system order are retained. Update publication precedes merchant processing, so success or failure releases requests against current state. [Visible behavior contract](../../specs/bot-economy.md#durability-and-acceptance).

Structural real-sync regression was genuinely RED: fixed ticks resumed only after the deadlock rescue released the real FileBackend sync. Current scoped proof covers57 distinct server economy controls (56 unchanged controls plus the corrected real-buyout fixture), one admin control and seven peer-example controls; no duplicate counts. The buyout fixture first genuinely lost11-copper mail; after the guard it preserves that mail, aborts the stale cycle and collects it exactly once on the next scheduled cycle. Its initial post-fix gold expectation exceeded the gatherer capital cap; the corrected fixture supplies sufficient configured headroom rather than changing product accounting. Workspace all-targets check and fmt pass, no compiler warnings. Independent artifact verification passes the mailbox fix and wiring; lead integration gate and post-landing live proof remain separate. **MAIN_THREAD_ASSET_READS** remains open: separate captured main-thread navigation reads (`navigation/query.rs:272` via `creature_ai.rs:556`) and LOS reads (`combat.rs:535`) are not fixed by this slice. Dormant-aura A/B stays parked.

### Whole-row input audit

| Market input/write | Async-cycle protection |
|---|---|
| Economy state row | Exact expected persisted state checked under the market write lock; receipt-gated memory publication; economic-vendor mutations deferred until resolution |
| Existing auctions purchased/deleted | Exact expected auction row checked under the same lock; publication applies only transaction deltas, never replaces the full live auction map |
| New gatherer inventories/new auctions/item and mail IDs | Fresh durable reserved high-water ranges; restart tests burn unused IDs and mixed allocators cannot collide |
| Existing gatherer inventory rows | Exact expected inventory before the full replacement, including zero-popularity/no-listing paths |
| Gatherer mailbox replacements | Exact expected mailbox read before collection; concurrent payout/expiry append rejects the cycle |
| Sale-mail appends | Existing mailbox read/append happens inside the write transaction, after replacements; no prepared full-row overwrite |
| Inventory records and definition-source rows | Not produced by the async cycle; existing synchronous merchant/AH callers remain outside the new window |

These are all `commit_market` row inputs. Failed/conflicting cycles use the existing failure-to-next-scheduled-cycle path, never merging or patching changed rows.

## Observed model

`crates/server/src/economy/model.rs` contains `MerchantState` with gold and a vector of `StockBatch` records. Each batch records remaining quantity, ingredient expenditure, unit price and expiry. `craft_stock` validates recipe expansion and configured membership, checks shared quantity capacity and affordability, and records stock. `sell_stock` removes sold quantity and its proportional ingredient cost, retaining rounding remainder in unsold stock. `expire_stock` removes expired batches and returns remaining ingredient cost; callers must credit and persist the refund. `credit_capital` credits available headroom and returns overflow for caller accounting. It does not reduce an already-over-limit wallet, so existing over-cap wallets after reducing the configured cap remain a known configuration-reconciliation limitation. Capped credits are not unconditional-cap proof.

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

## Returned gatherer goods

`16414b5` fixes collection after auction expiry: the old collector credited money and deleted delivered mail but dropped its attachments. Delivered items now move unchanged into the gatherer's inventory in the same market transaction. Relisting consumes retained goods before new gathering; whole stacks keep their GUID, splits get a new GUID, and deposits use the retained item's vendor value. Failed commits retain the original mail/inventory/state; retry claims once. Regression proof: `/tmp/claude/bot-return-proof-ledger.md`, 48 economy tests and 3 actor tests passed, including real expiry → collection → reopen and split/whole relisting.

## Current capability and proof ledger

Saved evidence root: `/home/osso/.cache/economy-cli-20260930/a54a0ad0`. [[auction-house]] owns exact auction action/restart/mail/performance receipts and revision limitations. No tests or live operations were rerun for this docs update.

| Capability | Exact receipt | Proof / limits |
|---|---|---|
| Gatherer supply and atomic cycle | `supplier-live-status-start.json`, `merchant-produced.json`; inspected `cycle.rs`/`store.rs` | Live gatherer 2318 and paid lots/stock; central commit precedes state publication |
| Direct supplier and actual recipe | `merchant-proof-catalog.json`, `merchant-produced.json` | Lara Moore 1295, classic expansion 0, recipe 2149: two Light Leather 2318 plus one Coarse Thread 2320 produce one boot 2302. Supplier entry 66/runtime slot 8 sells thread for exactly 10 copper per one-item bundle |
| Exact recorded costs and finite stock | `merchant-produced.json` | Three one-boot batches: ingredient costs 16160/14222/16448 copper; quotes 326982/345551/356166. Leather remainder one item/cost 8430; merchant gold 944740, cumulative withdrawn 36333 |
| Actual player merchant purchase | `cli-merchant-list-proof.json`, `cli-merchant-buy-proof.json`, `merchant-after-player-buy.json` | CLI bought slot 1 at actual price 326982, inventory confirms boot. Only batches 2/3 remain; merchant gold capped at 1000000, cumulative withdrawn 324102. Merchant-sale overflow is 271722 = 944740 + 326982 - 1000000. Global withdrawn increased 287769; purchase overflow is only 271722, leaving 16047 outside purchase attribution. Correction `0f88f87` preserves this distinction |
| Auction CLI/restart/mail and sampled latency | [[auction-house]] | Saved request/response and normal restart proof; not crash injection; native has separate bounded receipts |
| Simulated consumption, expiry and failure boundaries | `/tmp/economy-readability-25ee348/ledger.json` and saved output | 47 scoped economy/merchant tests passed; exact revision/hashes and limits in [[auction-house]]; not full-catalog live/crash acceptance |
| Modern content and native acceptance | [[economy-catalog]] and [[auction-house]] | Midnight/new Silvermoon data blocker; bounded native audit PASS, not full conversion |

The merchant catalog deliberately restricts real recipe 2149, shared capacity 3, customer budget/quantity 0 and cycle 2 seconds to bound the flow. These are test-fixture restrictions, not production policy, reserved product slots or guaranteed boots availability. Full catalog empty stock was diagnosed as a sourcing boundary; user explicitly chose direct vendor-only supply rather than fabricated gatherer thread or an AH fallback. Importer/runtime commits `13241e0`/`cd9b690` implement that choice. Full-catalog product availability remains unproven and is not guaranteed.

The normal auction restart proves the recorded buyer/seller state only. It does not establish restart/crash-boundary coverage for every merchant procurement, expiry, customer or withdrawal transition. Saved build receipts are not broad-suite acceptance, and the artifact directory label is not an exact revision stamp for every run. Canonical master was restored and server work remains isolated (user-reported state, not changed or recertified here).

## Final ledger boundary (October 1, 2026)

[[auction-house]] records twelve pinned CLI samples, six workloads/two cycles, historical default-profile failure and focused fixes, plus sibling native docs at `5b27878f`; independent final delta PASS is recorded in `/tmp/economy-delta-verify-20261001/report.md`, not a fresh whole-project suite. [[economy-catalog]] records generated default Lara Moore 1295 classic Leatherworking: 223 recipes, 27 gatherables, 11 suppliers. That explicit classic assignment is not the requested Midnight/new Silvermoon catalog. Missing source mappings/spawns remain a blocker, not an implementation exclusion or substitute acceptance. Limited suppliers and unsupported recipe effects are implementation exclusions; numeric importer defaults remain provisional tuning.

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

- [[auction-house]] — grouped API and exact CLI/restart/mail/sampled-performance ledger; bounded native proof and pending fresh source checker.
- [[economy-catalog]] — importer and measured source-price provenance, not live auction listing prices.
- [[merchant]] — current player purchase boundary.
- [[professions]] — recipe/reagent reference sources.
- [[persistence]] — durable transaction integration.
