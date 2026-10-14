# Trading Post authority

Retail PerksActivity/PerksVendorItem rows from local CASC69933 and Mainline Blizzard_PerksProgram define source behavior. [Client contract](../../../../game-engine/docs/specs/trading-post.md) owns both-skin requirements. No invented catalog or rotations.

## What it must do

- [x] Monthly progress uses real committed achievement events and current-account contributions; login reconstruction cannot generate historical credit.
- [x] Calendar rollover is 00:00 UTC on the first, independent of the daily-reset hour; the existing injectable ServerClock controls it.
- [x] Sourced default PerksActivityThreshold group1 awards once per account/month. Balance persists across month/relog; cap resets, not earned currency. Lead-approved October12 policy replaces the earlier daily-reset-hour rule.
- [x] Atomic paid collection grants, receipt-bounded refunds, one frozen item and source-backed capital interaction.
- [x] Trading Post requests settle current monthly gameplay credit without publishing an unsolicited full Adventure Guide snapshot ahead of the vendor reply.
- [x] Zero-duration refund default leaves refund unavailable until an explicit policy is approved; no duration guessed.
- [x] Vendor replies evaluate existing item class/race/level restrictions for the querying player, carrying optional `usable` state for the source filter. Missing item/player facts remain unknown; usability is not a new purchase restriction.
- [x] Item icons follow C_Item.GetItemIconByID: lowest-order authored ItemModifiedAppearance → ItemAppearance.DefaultIconFileDataID; items without an appearance icon use their authored Item.IconFileDataID. Empty direct item icons must not erase available base-appearance icons.

## Test/admin tools (lead decision 2026-10-12)

Private-server setup only, on the existing admin socket (`GAME_SERVER_ADMIN_SOCKET`); no client message, chat command or player-reachable path reaches them. They exist so live proof can buy, collect, relog and refund without reachable activity content.

- `game-server-admin trading-post-grant-tender <username> <amount>`: credits Tender to the account. Does not touch monthly threshold receipts.
- `game-server-admin trading-post-refund-window <seconds>`: refund window for subsequent purchases until restart; `0` (startup default) disables refunds. Snapshots then report the window as administrator-set, never as a sourced duration.

## How it works

- [Existing monthly projection](../wiki/systems/adventure-guide.md).

## Implementation inventory

- `crates/server/src/adventure_guide.rs`: monthly gameplay projection and award boundary.
- `crates/server/src/adventure_guide_catalog.rs`: load sourced default thresholds.
- `crates/server/src/trading_post_store.rs`: durable account threshold receipts, atomic balance/collection purchases, refund deltas and freeze.
- `crates/server/src/trading_post.rs`, `trading_post_catalog.rs`: requests, source joins, NPC validation and account collection publication.
- `scripts/retail_content.py`: retains sourced Trading Post high service flag.
- `admin_ipc.rs` `dispatch_trading_post`, `bin/admin.rs`: the two test/admin tools above.

## Tests asserting this spec

- `trading_post_store::tests::trading_post_thresholds_once_per_account_month_survive_database_reopen`.
- `trading_post_store::tests::trading_post_purchase_atomic_collection_grants_and_refund_survive_reopen`: all four collection stores, duplicate/insufficient-funds refusal, zero-duration/expiry/repeated refund and database reopen.
- `trading_post_tests.rs`: real gameplay death credit/Tender/midnight reset, real message dispatch and sourced vendor gossip, paid purchase/refund refusal/distance closure.
- `trading_post_tests.rs` `trading_post_admin_tender_and_refund_window_enable_wire_refund`: admin grant by username (unknown account refused), administrator window, real wire purchase/refund restoring Tender and revoking the appearance.
- `bin/admin.rs` `parse_trading_post_setup_requests`.
- `scripts/tests/test_trading_post_npc.py`: production importer preserves authentic Seabraid flag and source spawn.
- Existing Adventure Guide live-event/alt projection tests.

## Known gaps (current cycle)

- [ ] Monthly featured catalog and seasonal reward/calendar binding.
- [ ] Refund duration sourcing/policy (the admin window is test setup, not policy).

## Out of scope

None of requested section excluded.
