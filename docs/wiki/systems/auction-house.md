# Auction house

Read-only audit on September 30, 2026 separates the new global item-browse wire contract from existing flat auction search and unverified concurrent integration. [Auction house spec](../../specs/auction-house.md) owns browse/selection/latency requirements; [[bot-economy]] links economy terms.

## Wire contract

Shared-protocol commit `88bc3feb68054d3ef7e032af08f7a5f2e801b7de` adds `QueryAuctionBrowse { query: AuctionSearchQuery }` and `AuctionBrowseResults { query, total_results, items }`. `AuctionBrowseItem` contains `item_id: u32`, `name: String`, `quality: u8`, `required_level: u16`, `lowest_unit_price: u64` and `total_quantity: u64`. Source comments define the price as lowest effective unit price rounded up to integer copper, and the total as matching distinct items before pagination. This commit does not define an additional commodity policy or replace flat query/result fields.

Registration adds client-to-server browse requests and server-to-client browse responses. Existing `QueryAuctions` / `AuctionSearchResults` still return individual `AuctionListingSummary` rows with real `auction_id` values. Flow: grouped browse selects ItemID → exact-item flat query selects AuctionID → existing bid/buyout operation. The grouped row carries no AuctionID and cannot itself be bought.

## Observed server boundary

At audit time, `queries.rs` contained flat `search_auctions_matching`: filter all auctions, count matching listings, then select/sort a page. It did not contain grouped browse. Flat sorting uses AuctionID as a deterministic tie-breaker. `handlers.rs` contained `process_query_requests`, not a grouped request handler; it requires an open auctioneer session and overwrites the requested faction with the session house. `search_for_viewer` applies template class and required-level/race/class usability filtering before flat paging. Missing templates/viewers do not satisfy requested usability.

The grouped implementation must apply those eligibility boundaries before global per-item price/quantity aggregation, then sort/page distinct items. Global means all eligible listings in the auctioneer's house, not all houses and not a fetched-page aggregate. Main and agent22 are implementing this server boundary in `/home/osso/.worktrees/game-server-bot-economy`; this audit is not their verification gate.

## Integration and proof matrix

| Boundary | Observed state | Proof level / remaining work |
|---|---|---|
| Shared browse API | New types/directions in `88bc3fe`; flat shapes unchanged by that diff | Read-only commit inspection; serialization test exists, not run here |
| Server global distinct items, price, quantity, class/usability/faction, paging | Concurrent work; absent from inspected query/handler snapshot | Behavioral aggregation and live request proof pending |
| Flat search | Existing full-population filters and listing paging | Reported optimized helper benchmark: 300k listings, 144 ms; not independently rerun or network acceptance |
| CLI grouped → ItemID → real AuctionID | Agent24 owns CLI integration and `docs/game-cli.md` | CLI acceptance must run first; pending |
| Native grouped display | Agent23 replacing page-local aggregates; owns engine `auction-house-ui.md` | Native acceptance follows CLI; pending |

No acceptance box was checked. Shared serialization proves only encoded data survives a round trip, not server aggregation. Grouped performance is pending. Request/response proof must include actual NPC coordinates/session and actual listing prices, not a fallback global price proxy, plus revision/population/workload and persisted outcomes. The sub-200 ms goal includes client request-to-response timing, not just a pure helper.

User reports canonical master `a6a704f` saved clean and functional. This audit neither changes nor certifies that checkout. All server doc edits occur only in the server worktree; CLI/engine-owned docs remain untouched. Shared-protocol currently has only character-customization and protocol-check specs, no relevant AH documentation to update.

## Load fixture

[[auction-load-fixture]] documents the explicitly invoked disposable 300k-offer database producer, ownership guards and persistence/pagination assertions. Its scale is a test boundary, not production listing policy; actual CLI latency remains main's proof boundary.

## Sources

- [Auction house spec](../../specs/auction-house.md) — exact requirements and pending acceptance.
- [Shared gameplay messages](/syncthing/Sync/Projects/world-of-osso/shared-protocol/src/protocol/gameplay_messages.rs), [registration](/syncthing/Sync/Projects/world-of-osso/shared-protocol/src/protocol/registration.rs), [wire tests](/syncthing/Sync/Projects/world-of-osso/shared-protocol/tests/auction_query.rs) — inspected `88bc3fe` API additions.
- [Server queries](../../../crates/server/src/auction_house/queries.rs), [handlers](../../../crates/server/src/auction_house/handlers.rs), [operations](../../../crates/server/src/auction_house/operations.rs) — inspected flat/session/action boundaries.
- [Auction tests](../../../crates/server/src/auction_house_tests.rs) and [NPC tests](../../../crates/server/src/npc_interaction_tests.rs) — candidate behavior/benchmark/session coverage, not executed here.
- [CLI documentation](../../game-cli.md) and [native engine spec](/syncthing/Sync/Projects/world-of-osso/game-engine/docs/specs/auction-house-ui.md) — separately owned integration surfaces; concurrent updates not acceptance evidence.

## See Also

- [[bot-economy]] — supply and auction terms; CLI-first economy acceptance.
- [[npc-interaction]] — actual NPC validation, session house and faction routing.
- [[mail]] — settlement delivery.
- [[persistence]] — inventory/gold durability.
