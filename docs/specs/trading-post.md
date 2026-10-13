# Trading Post authority

Retail PerksActivity/PerksVendorItem rows from local CASC69933 and Mainline Blizzard_PerksProgram define source behavior. [Client contract](../../../../game-engine/docs/specs/trading-post.md) owns both-skin requirements. No invented catalog or rotations.

## What it must do

- [ ] Monthly progress uses real committed achievement events and current-account contributions; login reconstruction cannot generate historical credit.
- [ ] Calendar rollover is 00:00 UTC on the first, independent of the daily-reset hour; the existing injectable ServerClock controls it.
- [ ] Sourced default PerksActivityThreshold group1 awards once per account/month. Balance persists across month/relog; cap resets, not earned currency. Lead-approved October12 policy replaces the earlier daily-reset-hour rule.
- [ ] Atomic paid collection grants, receipt-bounded refunds, one frozen item and source-backed capital interaction.
- [ ] Zero-duration refund default leaves refund unavailable until an explicit policy is approved; no duration guessed.

## How it works

- [Existing monthly projection](../wiki/systems/adventure-guide.md).

## Implementation inventory

- `crates/server/src/adventure_guide.rs`: monthly gameplay projection and award boundary.
- `crates/server/src/adventure_guide_catalog.rs`: load sourced default thresholds.
- `crates/server/src/trading_post_store.rs`: durable account threshold receipts, atomic balance/collection purchases, refund deltas and freeze.
- `crates/server/src/trading_post.rs`, `trading_post_catalog.rs`: requests, source joins, NPC validation and account collection publication.
- `scripts/retail_content.py`: retains sourced Trading Post high service flag.

## Tests asserting this spec

- `trading_post_store::tests::trading_post_thresholds_once_per_account_month_survive_database_reopen`.
- `trading_post_store::tests::trading_post_purchase_atomic_collection_grants_and_refund_survive_reopen`: all four collection stores, duplicate/insufficient-funds refusal, zero-duration/expiry/repeated refund and database reopen.
- `trading_post_tests.rs`: real gameplay death credit/Tender/midnight reset, real message dispatch and sourced vendor gossip, paid purchase/refund refusal/distance closure.
- `scripts/tests/test_trading_post_npc.py`: production importer preserves authentic Seabraid flag and source spawn.
- Existing Adventure Guide live-event/alt projection tests.

## Known gaps (current cycle)

- [ ] Monthly featured catalog and seasonal reward/calendar binding.
- [ ] Refund duration sourcing/policy.
- [ ] Collections/vendor/transport/client/live proof in progress.

## Out of scope

None of requested section excluded.
