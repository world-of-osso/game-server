# Auction house

Current server source inspected October 1, 2026 at `8193d7e`, including server/shared integration `b0153e4`. The [spec](../../specs/auction-house.md) owns requirements. CLI proof exists; native AH/mail source is implemented but live engine proof is IN PROGRESS, not complete.

## Current implementation

Shared browse messages remain distinct from flat search: `QueryAuctionBrowse` returns `AuctionBrowseResults` with ItemID, metadata, lowest unit price and total quantity. A grouped row has no AuctionID. Selecting its ItemID uses exact-item `QueryAuctions`; transactions use a real returned AuctionID.

`browse.rs` filters expired/zero-count offers and applies item, text, quality, level, session house and viewer class/usability eligibility before global aggregation. Price comparisons retain rational stack prices until copper ceiling at serialization; quantities cover all eligible listings. Distinct-item paging follows aggregation with ItemID tie-breaking. `handlers.rs` requires the auctioneer session and overrides the requested house. Flat `queries.rs` retains listing results and AuctionID tie-breaking, using a bounded heap for first pages and partial selection for later pages.

## Saved proof ledger

Evidence root: `/home/osso/.cache/economy-cli-20260930/a54a0ad0`. Files are saved observations, not commands rerun by this documentation update. The directory name is a run label, not proof that every artifact was built at revision `a54a0ad0`. Current source includes importer `13241e0`, runtime `cd9b690` and fixture follow-up `8193d7e`; saved build outputs alone do not stamp every workload with an exact Git SHA. No current-revision broad-suite certification is inferred.

| Capability | Exact evidence | Proof / limits |
|---|---|---|
| Real load population and meaningful second pages | `load-producer-final.json`: 300000 offers, 311 real ItemIDs; grouped pages 50/50; ItemID 4264 has 965 offers and second page 50; next auction/item/character IDs 300002/300002/3 | Producer test passed, full reopen/counter/helper assertions; not network timing |
| CLI grouped browse, text search and flat paging | `perf-group-page0.json`, `perf-group-page1.json`, `perf-group-text.json`, `perf-flat-name.json`, `perf-flat-item-page1.json` | Actual CLI request/response; workload details below |
| Posting and real AuctionID actions | `create-short.json`, `create-medium.json`, `create-long.json`, `owned-after-post.json`; `buyer-bid.json`, `buyer-buyout.json`, `seller-cancel.json` | Short/medium/long posts; bid and buyout 107, cancel 108; long 109 remains active |
| Refusal boundary | `reject-own-buyout.json`, `reject-underbid.json`, `reject-out-of-range.json` | Exit 1 with own-auction, minimum-next-bid and too-far-away errors respectively |
| Restart and mail settlement | `restart-boundary.json`, `restart-buyer-inventory.json`, `restart-seller-inventory.json`, mailbox and claim artifacts | Buyer 99800, seller 99909 after restart. Buyer mail 10 attachment claims one Linen Cloth; seller mail 9 money 203 claims to gold 100112; mail 11 return claims to total 10 Linen Cloth. Normal restart, not crash injection |
| Native AH/mail | Saved native build/run/pane evidence is partial | Source implemented; live engine browse/actions/mail proof IN PROGRESS |

### Request-response performance

`perf-summary.json` records client request-to-matching-response milliseconds, not model/helper time. All samples returned 50 rows, page size 50, session faction 1, no class/quality/level/usability restriction.

| Workload | Page / sort | Matching total | Milliseconds |
|---|---|---:|---:|
| Grouped all items | 0 / buyout ascending | 311 distinct items | 60.790 |
| Grouped all items | 1 / buyout ascending | 311 distinct items | 60.626 |
| Grouped text Leather | 0 / name ascending | 75 distinct items | 90.548 |
| Flat all listings | 0 / name ascending | 300075 offers | 80.651 |
| Flat ItemID 2318 | 1 / buyout ascending | 970 offers | 70.409 |

The producer seeded 300000 offers; the live flat output reports 300075, not exactly 300000. Every sampled workload is below 200 ms on one owned loopback server. This is neither a universal concurrency SLA nor proof of every filter/sort combination, remote latency, percentiles or current-revision reproducibility. Exact workload revision stamping remains a gap.

## Sources

- [Spec](../../specs/auction-house.md) — acceptance contract.
- [Grouped browse](../../../crates/server/src/auction_house/browse.rs), [behavioral tests](../../../crates/server/src/auction_house/browse_tests.rs) — source inspected; test existence alone is not an execution receipt.
- [Handlers](../../../crates/server/src/auction_house/handlers.rs), [flat queries](../../../crates/server/src/auction_house/queries.rs), [operations](../../../crates/server/src/auction_house/operations.rs) — current session/search/transaction boundaries.
- [CLI](../../../crates/client/src/bin/game_cli.rs) — client acceptance surface; saved JSON artifacts above retain exact outputs.

## See Also

- [[auction-load-fixture]] — producer ownership and reopen assertions.
- [[bot-economy]] — supplier/merchant proof, auction terms and remaining acceptance.
- [[mail]] — settlement implementation.
- [[npc-interaction]] — session validation.
- [[persistence]] — durable state.
