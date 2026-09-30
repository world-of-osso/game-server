# Bot economy

Integration inventory observed September 30, 2026. The [tracked spec](../../specs/bot-economy.md) is the single source of truth for approved behavior. This page describes inspected code and unresolved connections, not completed live operation.

## Observed model

`crates/server/src/economy/model.rs` contains `MerchantState` with gold and a vector of `StockBatch` records. Each batch records remaining quantity, ingredient expenditure, unit price and expiry. `craft_stock` validates recipe expansion and configured membership, checks shared quantity capacity and affordability, and records stock. `sell_stock` removes sold quantity and its proportional ingredient cost, retaining rounding remainder in unsold stock. `expire_stock` removes expired batches and returns remaining ingredient cost; callers must credit and persist the refund. `credit_capital` credits available headroom and returns overflow for caller accounting. It does not reduce an already-over-limit wallet, so maintaining the cap across configuration/restart boundaries requires integration proof.

`quote_price` maps a supplied draw into price bounds. `consume_stock` applies quantity/budget limits to supplied draws and removes purchased units from a cloned state before publishing the result. These helpers do not themselves purchase auction ingredients, commit storage transactions, schedule cycles or serve vendor requests. Their existence cannot establish those behaviors.

## Integration boundaries

- Auction operations: `auction_house/operations.rs` creates listings and charges deposits; `auction_house/queries.rs` filters, sorts and pages flat listings. [[auction-house]] owns the new distinct grouped wire contract and integration proof matrix: global item browse → ItemID → flat real AuctionID. Query latency needs actual request/response measurement; reported flat helper timing is not acceptance.
- Vendor requests: [[merchant]] documents the current catalog, limited vendor stock and player inventory persistence. Bot-crafted stock must be connected and proven at this request boundary; ordinary vendor restocking is not bot-stock evidence.
- Recipes: [[professions]] identifies `skill_line_ability`, `spell_reagents` and spell `CREATE_ITEM` data. These are useful integration sources, not proof of complete expansion-11 recipes/output eligibility or new Silvermoon merchant spawns.
- Persistence: [[persistence]] describes redb mutable storage and SQLite reference data. Model serialization is not atomic transaction/restart proof.
- Client: `crates/client/src/bin/game_cli.rs` is the headless protocol surface. Live CLI proof must precede engine acceptance; native auction UI documentation is maintained separately in game-engine.

## Evidence limits

`economy/model_tests.rs` contains behavioral examples for crafting costs, expiry and shared capacity. No test execution or live acceptance evidence was supplied to this audit. All spec requirements therefore remain unchecked. Concurrent code changes must be evaluated at their own revision; this inventory is not a sign-off on those changes.

No economy importer path, complete bot runtime connection, current expansion-11 dataset coverage, restart proof or client query timing was established. Configuration examples and model fixture numbers are not approved economy policy.

## Sources

- [Bot economy spec](../../specs/bot-economy.md) — approved contract and acceptance gaps.
- [Auction house spec](../../specs/auction-house.md) — global browse, flat selection and CLI-first latency contract.
- [Economy model](../../../crates/server/src/economy/model.rs) — inspected accounting helpers.
- [Model tests](../../../crates/server/src/economy/model_tests.rs) — candidate model coverage, not executed here.
- [Auction queries](../../../crates/server/src/auction_house/queries.rs) — inspected search/paging boundary.
- [Auction operations](../../../crates/server/src/auction_house/operations.rs) — deposit/creation integration.

## See Also

- [[auction-house]] — grouped API, source links and pending CLI/native/performance proof.
- [[economy-catalog]] — importer and measured source-price provenance, not live auction listing prices.
- [[merchant]] — current player purchase boundary.
- [[professions]] — recipe/reagent reference sources.
- [[persistence]] — durable transaction integration.
