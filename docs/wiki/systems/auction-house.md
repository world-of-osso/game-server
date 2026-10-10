# Auction house

Current server source inspected October 1, 2026 at `8193d7e`, including server/shared integration `b0153e4`. The [spec](../../specs/auction-house.md) owns requirements. Bounded CLI/native audit PASS exists; independent final delta and actor-parity gates pass their recorded scopes. Final receipts below supersede historical gaps only within their scope.

## Current implementation

Shared browse messages remain distinct from flat search: `QueryAuctionBrowse` returns `AuctionBrowseResults` with ItemID, metadata, lowest unit price and total quantity. A grouped row has no AuctionID. Selecting its ItemID uses exact-item `QueryAuctions`; transactions use a real returned AuctionID.

`browse.rs` filters expired/zero-count offers and applies item, text, quality, level, session house and viewer class/usability eligibility before global aggregation. Price comparisons retain rational stack prices until copper ceiling at serialization; quantities cover all eligible listings. Distinct-item paging follows aggregation with ItemID/source tie-breaking. `handlers.rs` requires the auctioneer session and overrides the requested house. Flat `queries.rs` retains listing results and AuctionID tie-breaking, using a bounded heap for shallow prefixes and partial selection for deeper pages.

## Injected expiry clock (2026-10-09)

The sweep receives `Res<ServerClock>`; `AuctionHousePlugin` initializes it without overwriting an already pinned clock. Default time remains wall Unix time. The existing same-UID `clock-advance <seconds>` handler shifts that resource, now affecting auction expiry as well as instance locks; no new IPC command or process-global clock was added. After posting/bidding on an owned private server, advance beyond the longest desired listing duration, then allow the existing one-second sweep cadence to run. Never use the shared realm for this proof. Listing creation/browse and mail timestamps still use wall time.

[Clock regressions](../../../crates/server/src/auction_house_tests/clock.rs) post real inventory, optionally place a real bid, prove no settlement one second before the persisted deadline, invoke the admin handler to reach it, sweep twice and reopen redb. Assertions cover Auction House sender, exact returned item/stack, `Auction Expired`, seller proceeds (1001 minus 50 plus deposit), buyer mail and persisted bid debit. The [spec](../../specs/auction-house.md#expiry-clock-and-settlement) owns requirements; [bot policy](../../specs/bot-economy.md#auction-supply-and-terms) owns the retained unsold-deposit refund, not Retail parity.

Reference: TrinityCore `a352b1fa`, `AuctionHouseMgr.cpp`, `AuctionHouseObject::Update` branches to `SendAuctionExpired` when no bidder exists and `SendAuctionWon`/`SendAuctionSold` otherwise; the expired mail attaches the owner's item, and sold mail deducts the house cut and adds the deposit. [Pinned primary source](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/AuctionHouse/AuctionHouseMgr.cpp#L987-L1030). Exact English subject here is the requested client contract, not TrinityCore's encoded wire subject.

Execution evidence under `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/ahexpiry-2026-10-09/`: valid RED at `a999851`, both tests fail because the injected deadline is ignored (`red-valid.log`); final GREEN at `ab4cd69`, 2/2 pass (`green-final.log`). Full locked workspace gate at the same code: 1586 passed, 0 failed, 60 existing ignored, one required skip (`query_300k_listings_under_200ms`); server unit subset 1521 passed. No compiler warnings; changed-Rust format check passes. `proof-ledger.json` records exact commands and scope hashes. Interim fixture/table, Bevy API and original-vs-escrow GUID errors are retained but are not behavioral RED. No live expiry claim.

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

## Final returned-goods follow-up

`16414b5` corrects bot attachment collection without changing query, CLI or native sources. Final format/all-target check at `fad180f` passed without warnings; independent report: `/tmp/claude/bot-return-final-verify-20261001/`. Current binary hashes and source revision are in `returned-goods-build-manifest.json`. Six fresh CLI workloads at that binary returned in 60.268–90.744 ms, including both grouped pages, text, flat name, ItemID page 1 and category/usability; full receipts are `returned-final-performance.json` and `returned-final-*.json` under the evidence root. This remains owned-loopback sampled acceptance, not a universal SLA.

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

## Subcategory / Current Bid / Available — 2026-10-08

Existing `AuctionSearchQuery` now carries OR alternatives of class, optional subclass and optional inventory type, combined with the other filters. `handlers::auction_matches_viewer` checks authoritative source-local templates before either flat paging or global grouping. Class-only queries remain class-only; missing templates cannot match template-based restrictions. The Retail Chest node can send both inventory5 and20 without treating robes as a different class.

`Bid` orders flat listings by current bid or their displayed minimum before any bid, separate from the preserved `MinBid` key. Browse `Bid` uses the lowest rational displayed unit bid across all eligible offers, without rounding away fractional-copper order. `Quantity` compares actual flat stack counts or global `u64` group totals. Directed primary ordering retains ascending AuctionID ties (flat) and ItemID/source ties (groups), before selecting pages. No new messages, client-page sorting, or transaction changes.

Source: TrinityCore `a352b1fa` AuctionHouseMgr.cpp:1182-1198 filters class/subclass/inventory before browse result building;:243-273 bucket and:280-332 posting comparators handle directed primary sorts. TC posting ties use LIFO/raw-zero unbid amounts and its comparator omits Quantity; this project deliberately retains existing ascending IDs and the displayed minimum-before-first-bid contract. Cached Retail `Blizzard_AuctionHouseUI/Shared/Blizzard_AuctionHouseTableBuilder.lua:1033-1061,1094-1106` exposes `AuctionHouseSortOrder.Quantity` as Available and Bid as Current Bid; sibling [engine contract](../../../../game-engine/docs/specs/auction-house-ui.md) owns category/header actions and offline rendering.

At code44cf153, `/home/osso/.worktrees/logs/ahsort-server-green.log` EXIT0:4 targeted `ahsort_` tests passed. Initial RED at d3ff2c0 was0/3 failed on unfiltered counts; filter-only source a537db4 gave1 pass/3 order/tie failures in `ahsort-server-sort-red.log`. Tests assert full concatenated50/25 pages over75 eligible items and50/29 pages over79 source-local groups, both directions, starting-vs-current bid, aggregate quantities beyond the first flat page, Chest/Robe alternatives, excluded text/unknown templates and stable source ties. Scope excludes live runs, broad suite gates and performance claims. Protocol round-trip3 and layout5 passed in sibling shared-protocol. [Current spec](../../specs/auction-house.md#subcategory-and-displayed-column-sorting) defines the contract, not historical acceptance receipts.
