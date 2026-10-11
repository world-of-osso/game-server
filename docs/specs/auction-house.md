# Auction house

Global item browse selects an ItemID; existing flat search then selects a real AuctionID for bid or buyout. Server sources live in `crates/server/src/auction_house/`; shared-protocol commit `88bc3fe` adds the distinct browse API. See [implementation and evidence](../wiki/systems/auction-house.md). Current server/shared integration includes `b0153e4`. Checked bullets have behavioral or saved live proof. Independent final server-delta and native audits pass their recorded scopes; limits are retained in the linked ledger.

## What it must do

### Browse and listing selection

- [x] `QueryAuctionBrowse` / `AuctionBrowseResults` provide global per-item browse alongside existing `QueryAuctions` / `AuctionSearchResults`; both queries share `AuctionSearchQuery`.
- [x] Apply exact-item, text, quality, level, class, usable-only and session-house faction filters before aggregation and pagination. Usability uses the actual viewer; house selection comes from the auctioneer session, not a client-selected cross-house proxy.
- [x] Return one row per matching ItemID, with name, quality, required level, lowest effective unit price rounded up to copper, and total available quantity across all matching listings, not only one fetched flat page.
- [x] Sort and paginate distinct items after global aggregation; `total_results` counts distinct matching items before pagination. Verify a second page with enough distinct items and listings spanning multiple flat pages.
- [x] Selecting an ItemID issues the existing exact-item flat query. Bid/buyout acts on a real returned AuctionID; grouped rows are not synthetic auctions or purchase targets.

### Subcategory and displayed-column sorting

- Extend the existing browse/item query, not a parallel request: category nodes supply OR alternatives of item class, optional subclass and optional inventory type; other query constraints are ANDed with that selection.
- Apply category alternatives against authoritative source-local templates before aggregation/paging. No template means no match when template filters are requested.
- Current Bid sorts the displayed current bid, or starting minimum when no bid exists; `MinBid` remains a separate starting-bid sort. Available sorts actual auction-stack counts for flat listings and global `u64` totals for browse groups.
- Both directions retain deterministic ascending AuctionID ties (flat), and ItemID/source ties (browse), before pagination. Prove full concatenated first/second pages, not just within-page order.

### Posting eligibility

- Reject soulbound item instances, bind-on-pickup items and quest items through the existing auction operation response, without changing inventory, money or listings.
- Unbound bind-on-equip items remain auctionable. Binding checks use server template and persisted item-instance state; no wire or persistence schema change is required.

### Refund notifications

- Retail Mainline owns auction text. Send the GlobalStrings subject tag and item name, not final English; outbid/removed/expired letters have empty bodies. Sold/won bodies carry invoice data (participant, bid, buyout, deposit, consignment, count) for the native client to render. See [mail-text contract](auction-mail-text.md).
- Outbid still credits the exact `u64` refund directly; it does not invent a refund-denomination letter.
- The notification does not attach the refund again: the balance credit and mail are settled together.

### Expiry clock and settlement

- The expiry sweep reads an injected server clock, defaulting to wall time; tests can pin it without sleeping or rewriting auction deadlines.
- At the deadline, no-bid auctions return the item to the seller in Auction House mail with subject tag `AUCTION_EXPIRED_MAIL_SUBJECT` plus the item name. Existing [unsold deposit refund policy](bot-economy.md#auction-supply-and-terms) remains unchanged.
- Bid auctions settle the actual winning bid: seller mail contains the bid minus the house cut plus the existing deposit refund; buyer mail contains the auctioned item. Auction removal and both mails persist atomically and survive database reopen without duplicate settlement.
- The existing same-UID `clock-advance <seconds>` admin command also advances auction expiry. It still advances instance locks; use only an owned private server for acceptance. It does not shift mail delivery/expiry, listing creation or browse clocks.

### Live acceptance

- [x] Saved CLI grouped all-item pages 0/1 and Leather text query, flat name search and ItemID 2318 page 1 return meaningful populated results. [Exact ledger](../wiki/systems/auction-house.md) records 311 distinct items and final live total 300068 offers (historical sample 300075); it does not certify every filter combination.
- [x] Saved CLI short/medium/long posts, bid107/buyout107/cancel108, three refusal cases, normal restart balances and mail claims succeed at the [recorded boundary](../wiki/systems/auction-house.md).
- [x] Twelve revision-pinned CLI request-response samples across six workloads/two cycles measure 60.282–121.623 ms on one owned loopback server. [Workloads and limits](../wiki/systems/auction-house.md) define this proof, not a universal concurrency SLA.

- [x] Prove browse/filter/paging/drilldown and create/bid/buyout/cancel through `game-cli` first, using actual auctioneer coordinates and interactions, actual listing prices and persisted inventory/gold/mail outcomes. No fallback global price proxy substitutes for listing evidence.
- [x] Bounded native seller/buyer/large/mail sequences pass saved exit-0 runs and independent agent51 audit, using server grouped results and real AuctionID actions. [Sibling engine proof](../wiki/systems/auction-house.md#bounded-native-audit) owns details and limits; not clean world assets or full conversion.
- [x] Auction query request-to-response latency is below 200 ms at the recorded acceptance workloads. Record revision, population, workload, paging, client and timing boundary; separately measure grouped browse and flat drilldown.
- The existing `query_300k_listings_under_200ms` model-query test enforces <200 ms in both default debug/test and release profiles; no release-only exemption or workspace-gate skip. This test measures flat Name page 0, MinBid page 2000 and mixed-case text Buyout page 1 over 300000 listings, not grouped browse or network latency. [Current unskipped gate proof](../wiki/systems/auction-house.md#unskipped-300k-query-gate-2026-10-10).

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

- `crates/server/src/auction_house_tests/clock.rs` — real create/bid, injected before/at-deadline sweep via admin clock advance, repeated sweep and reopened persisted seller/buyer mail. [Execution receipts](../wiki/systems/auction-house.md#injected-expiry-clock-2026-10-09); not live expiry acceptance.
- `crates/server/src/auction_house_tests.rs` — flat auction behavior and `query_300k_listings_under_200ms`; not grouped or network acceptance.
- `crates/server/src/npc_interaction_tests.rs` — actual auctioneer/session boundary.
- Shared-protocol `tests/auction_query.rs` — request/result serialization, including grouped fields; serialization does not prove global aggregation.
- `crates/server/src/auction_house/browse_tests.rs` — global aggregation, fractional-price ordering, distinct paging and viewer filters; existence alone does not certify execution.
- `load-producer-final.json`, `perf-summary.json` and CLI transaction/restart/mail receipts in `/home/osso/.cache/economy-cli-20260930/a54a0ad0` — [exact capability/proof ledger](../wiki/systems/auction-house.md).

## Known gaps (current cycle)

Final build and twelve CLI samples are pinned to `eb649f8`; tests-only `2ec5fb3` adds actor parity. Global aggregation/filter/paging behavior, live CLI/native transactions, and normal restart receipts have independent proof. No universal concurrency/WAN SLA, exhaustive native UI combination coverage, sending/COD, or clean-world-rendering claim is made.

## Out of scope

Favorites, tokens, commodity-specific purchase features and additional product policies are not authorized by this contract. This API retains the existing flat message types; the shared query shape evolves with protocol layout compatibility checks.
