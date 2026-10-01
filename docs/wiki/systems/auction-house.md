# Auction house

Current server source inspected October 1, 2026 at `8193d7e`, including server/shared integration `b0153e4`. The [spec](../../specs/auction-house.md) owns requirements. Bounded CLI/native audit PASS exists; independent final delta and actor-parity gates pass their recorded scopes. Final receipts below supersede historical gaps only within their scope.

## Current implementation

Shared browse messages remain distinct from flat search: `QueryAuctionBrowse` returns `AuctionBrowseResults` with ItemID, metadata, lowest unit price and total quantity. A grouped row has no AuctionID. Selecting its ItemID uses exact-item `QueryAuctions`; transactions use a real returned AuctionID.

`browse.rs` filters expired/zero-count offers and applies item, text, quality, level, session house and viewer class/usability eligibility before global aggregation. Price comparisons retain rational stack prices until copper ceiling at serialization; quantities cover all eligible listings. Distinct-item paging follows aggregation with ItemID tie-breaking. `handlers.rs` requires the auctioneer session and overrides the requested house. Flat `queries.rs` retains listing results and AuctionID tie-breaking, using a bounded heap for shallow prefixes and partial selection for deeper pages.

## Saved proof ledger

Evidence root: `/home/osso/.cache/economy-cli-20260930/a54a0ad0`. Files are saved observations, not commands rerun by this documentation update. The directory name is a run label, not proof that every artifact was built at revision `a54a0ad0`. Current source includes importer `13241e0`, runtime `cd9b690` and fixture follow-up `8193d7e`; saved build outputs alone do not stamp every workload with an exact Git SHA. No current-revision broad-suite certification is inferred.

| Capability | Exact evidence | Proof / limits |
|---|---|---|
| Real load population and meaningful second pages | `load-producer-final.json`: 300000 offers, 311 real ItemIDs; grouped pages 50/50; ItemID 4264 has 965 offers and second page 50; next auction/item/character IDs 300002/300002/3 | Producer test passed, full reopen/counter/helper assertions; not network timing |
| CLI grouped browse, text search and flat paging | `perf-group-page0.json`, `perf-group-page1.json`, `perf-group-text.json`, `perf-flat-name.json`, `perf-flat-item-page1.json` | Actual CLI request/response; workload details below |
| Posting and real AuctionID actions | `create-short.json`, `create-medium.json`, `create-long.json`, `owned-after-post.json`; `buyer-bid.json`, `buyer-buyout.json`, `seller-cancel.json` | Short/medium/long posts; bid and buyout 107, cancel 108; long 109 remains active |
| Refusal boundary | `reject-own-buyout.json`, `reject-underbid.json`, `reject-out-of-range.json` | Exit 1 with own-auction, minimum-next-bid and too-far-away errors respectively |
| Restart and mail settlement | `restart-boundary.json`, `restart-buyer-inventory.json`, `restart-seller-inventory.json`, mailbox and claim artifacts | Buyer 99800, seller 99909 after restart. Buyer mail 10 attachment claims one Linen Cloth; seller mail 9 money 203 claims to gold 100112; mail 11 return claims to total 10 Linen Cloth. Normal restart, not crash injection |
| Native AH/mail | Seller/buyer/large/mail manifests and logs; sibling engine ledger `5b27878f` | Bounded audit PASS (agent51); links and limits below |

### Request-response performance

`perf-summary.json` records client request-to-matching-response milliseconds, not model/helper time. All samples returned 50 rows, page size 50, session faction 1, no class/quality/level/usability restriction.

| Workload | Page / sort | Matching total | Milliseconds |
|---|---|---:|---:|
| Grouped all items | 0 / buyout ascending | 311 distinct items | 60.790 |
| Grouped all items | 1 / buyout ascending | 311 distinct items | 60.626 |
| Grouped text Leather | 0 / name ascending | 75 distinct items | 90.548 |
| Flat all listings | 0 / name ascending | 300075 offers | 80.651 |
| Flat ItemID 2318 | 1 / buyout ascending | 970 offers | 70.409 |

The producer seeded 300000 offers; the live flat output reports 300075, not exactly 300000. Every sampled workload is below 200 ms on one owned loopback server. This is neither a universal concurrency SLA nor proof of every filter/sort combination, remote latency, percentiles or current-revision reproducibility. Historical samples lack workload revision stamps; final pinned samples below close that gap only for their exact workloads.

### Final pinned CLI measurements (October 1, 2026)

`final-build-manifest.json` and `final-perf-summary.json` pin revision `eb649f8536867ad445270c981a4eaa9df255b418`; build exit 0. Binary SHA-256:
- `game-server`: `130a25ab148431af6dd678cd4fc13785d7abd60cd327a2ecec750d97f685fdbc`.
- `game-cli`: `d3925394a76a0d03ae739368abf63f21a2835a7dffb954032b2cd2a4ca9da2b3`.
- `game-server-admin`: `5589de70f6c3001e105ec8ecb69d9624f9abce5a6dc97ad90b81c3de44858d7e`.

Twelve actual CLI request-to-matching-response samples, six workloads/two cycles, owned loopback server. Exact output fields (session faction 1; ascending sorts):

| Cycle | Workload | ms | Page / size | Total / returned | Filters / sort |
|---|---|---:|---|---|---|
| 0 | group-page0 | 60.282 | 0 / 50 | 311 / 50 | all / Buyout |
| 0 | group-page1 | 60.366 | 1 / 50 | 311 / 50 | all / Buyout |
| 0 | group-text | 121.623 | 0 / 50 | 75 / 50 | Leather / Name |
| 0 | flat-name | 110.512 | 0 / 50 | 300068 / 50 | all / Name |
| 0 | flat-item-page1 | 60.455 | 1 / 50 | 969 / 50 | ItemID 2318 / Buyout |
| 0 | group-category | 80.846 | 1 / 25 | 86 / 25 | class 7 + usable-only / Name |
| 1 | group-page0 | 80.907 | 0 / 50 | 311 / 50 | all / Buyout |
| 1 | group-page1 | 60.617 | 1 / 50 | 311 / 50 | all / Buyout |
| 1 | group-text | 110.565 | 0 / 50 | 75 / 50 | Leather / Name |
| 1 | flat-name | 110.502 | 0 / 50 | 300068 / 50 | all / Name |
| 1 | flat-item-page1 | 60.518 | 1 / 50 | 969 / 50 | ItemID 2318 / Buyout |
| 1 | group-category | 60.329 | 1 / 25 | 86 / 25 | class 7 + usable-only / Name |

Range 60.282–121.623 ms, all below 200 ms. Final live totals are 300068 offers/311 distinct items, not historical 300075; ItemID 2318 has 969 offers, not historical 970. Bounded second-page/category/usability proof, not every filter, concurrency, remote latency or percentile SLA.

### Source gate and delta proof ledger

`/tmp/economy-final-verify-20261001/ledger.json` retains historical independent gate at `5c3ab29`: fmt exit 1, server/client all-target check exit 0, selected tests exit 101 (default shallow-page timing about 254 ms). Not a current final PASS.

Shallow heap correction `11374529`: `/tmp/economy-shallow-prefix-20261001/default-green.json` records 16 passed, exit 0; 300k name page 0 60.956102 ms, MinBid page 2000 152.447265 ms, iRoN buyout page 1 71.805398 ms. `optimized-green.json`: six passed, exit 0; corresponding 51.194362/31.698731/28.875537 ms. These are model-query tests, not CLI timing. `proof-ledger.json` records scope hashes and later ledger revision `0f88f87`.

Runtime readability delta `/tmp/economy-readability-25ee348/ledger.json`: executed revision `25ee348`, scoped economy/merchant tests exit 0, 47 passed; hashes and unchanged relevant scope through `eb649f85`. Client `/tmp/game-client-readability-tests-final.log`: 24 passed, exit 0. Independent final delta PASS: `/tmp/economy-delta-verify-20261001/report.md`; actor-parity PASS: `/tmp/economy-actor-parity-final-20261001/`. The later returned-goods regression has focused proof in `/tmp/claude/bot-return-proof-ledger.md`. Stable source and subset receipts are not a final whole-project gate.

### Bounded native audit

Native seller/buyer/large/mail manifests (`native-seller-run3.json`, `native-buyer-run.json`, `native-large-run.json`, `native-mail-run6.json`) and saved logs exit 0; independent agent51 bounded audit PASS. Detailed engine receipts at commit `5b27878f`: [auction runtime ledger](/syncthing/Sync/Projects/world-of-osso/game-engine/docs/wiki/systems/auction-house-ui.md#saved-native-runtime-proof-2026-10-01) and [mail receiving ledger](/syncthing/Sync/Projects/world-of-osso/game-engine/docs/wiki/systems/trade-and-mail.md#saved-native-receiving-proof-2026-10-01). Supersedes IN PROGRESS for those sequences only; no clean world rendering/assets or full Godot conversion claim.

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
