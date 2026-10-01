# Auction house

Global item browse selects an ItemID; existing flat search then selects a real AuctionID for bid or buyout. Server sources live in `crates/server/src/auction_house/`; shared-protocol commit `88bc3fe` adds the distinct browse API. See [implementation and evidence](../wiki/systems/auction-house.md). Current server/shared integration includes `b0153e4`. Checked bullets cover exact saved CLI observations only; native AH/mail source is implemented, live engine proof IN PROGRESS.

## What it must do

### Browse and listing selection

- [ ] `QueryAuctionBrowse` / `AuctionBrowseResults` provide global per-item browse without changing existing `QueryAuctions`, `AuctionSearchQuery` or `AuctionSearchResults` shapes.
- [ ] Apply exact-item, text, quality, level, class, usable-only and session-house faction filters before aggregation and pagination. Usability uses the actual viewer; house selection comes from the auctioneer session, not a client-selected cross-house proxy.
- [ ] Return one row per matching ItemID, with name, quality, required level, lowest effective unit price rounded up to copper, and total available quantity across all matching listings, not only one fetched flat page.
- [ ] Sort and paginate distinct items after global aggregation; `total_results` counts distinct matching items before pagination. Verify a second page with enough distinct items and listings spanning multiple flat pages.
- [ ] Selecting an ItemID issues the existing exact-item flat query. Bid/buyout acts on a real returned AuctionID; grouped rows are not synthetic auctions or purchase targets.

### Live acceptance

- [x] Saved CLI grouped all-item pages 0/1 and Leather text query, flat name search and ItemID 2318 page 1 return meaningful populated results. [Exact ledger](../wiki/systems/auction-house.md) records 311 distinct items and live total 300075 offers; it does not certify every filter combination.
- [x] Saved CLI short/medium/long posts, bid107/buyout107/cancel108, three refusal cases, normal restart balances and mail claims succeed at the [recorded boundary](../wiki/systems/auction-house.md).
- [x] Five recorded CLI request-response samples measure 60.790/60.626/90.548/80.651/70.409 ms on one owned loopback server. [Workloads and limits](../wiki/systems/auction-house.md) define this proof, not a universal concurrency SLA.

- [ ] Prove browse/filter/paging/drilldown and create/bid/buyout/cancel through `game-cli` first, using actual auctioneer coordinates and interactions, actual listing prices and persisted inventory/gold/mail outcomes. No fallback global price proxy substitutes for listing evidence.
- [ ] Then prove the same fully working auction house in the native engine; replace page-local browse aggregates with server grouped results.
- [ ] Auction query request-to-response latency is below 200 ms. Record revision, population, workload, paging, client and timing boundary; separately measure grouped browse and flat drilldown.

Auction durations, equal bot/player fees and expiry returns are owned by [bot economy](bot-economy.md), not redefined here.

## How it works

- [Auction house system and proof matrix](../wiki/systems/auction-house.md)
- [Auctioneer interaction and house validation](../wiki/systems/npc-interaction.md)
- [Mail settlement](../wiki/systems/mail.md)

## Implementation inventory

- `crates/server/src/auction_house.rs` — server plugin/state and handler registration.
- `crates/server/src/auction_house/queries.rs` — flat filter, ordering and pagination.
- `crates/server/src/auction_house/browse.rs` — global eligible-item aggregation, rational prices and distinct paging.
- `crates/server/src/auction_house/handlers.rs` — auctioneer session, server house override and viewer class/usability filtering.
- `crates/server/src/auction_house/operations.rs` — real AuctionID transactions.
- `crates/client/src/bin/game_cli.rs` — CLI acceptance surface; concurrent grouped integration not verified here.
- Shared-protocol `src/protocol/gameplay_messages.rs`, `registration.rs` — browse/flat wire types and message directions; absolute source links live in the wiki.

## Tests asserting this spec

Behavioral source plus saved evidence; no suites rerun by this docs audit:

- `crates/server/src/auction_house_tests.rs` — flat auction behavior and `query_300k_listings_under_200ms`; not grouped or network acceptance.
- `crates/server/src/npc_interaction_tests.rs` — actual auctioneer/session boundary.
- Shared-protocol `tests/auction_query.rs` — request/result serialization, including grouped fields; serialization does not prove global aggregation.
- `crates/server/src/auction_house/browse_tests.rs` — global aggregation, fractional-price ordering, distinct paging and viewer filters; existence alone does not certify execution.
- `load-producer-final.json`, `perf-summary.json` and CLI transaction/restart/mail receipts in `/home/osso/.cache/economy-cli-20260930/a54a0ad0` — [exact capability/proof ledger](../wiki/systems/auction-house.md).

## Known gaps (current cycle)

- [ ] Prove server global aggregation, class/usability/faction filtering and distinct-item paging on the implemented revision.
- [ ] Complete subsequent native-engine grouped-to-flat transactions and mail acceptance; source implementation and partial native artifacts are not live completion.
- [ ] Stamp exact measured workload revisions and extend latency coverage beyond the five passing loopback samples; no concurrency or remote-client SLA is established.

## Out of scope

Favorites, tokens, commodity-specific purchase features and additional product policies are not authorized by this contract. This API does not replace the existing flat wire shapes.
