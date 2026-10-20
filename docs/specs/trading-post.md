# Trading Post authority

Installed-build local CASC PerksActivity/PerksVendorItem rows and Mainline Blizzard_PerksProgram define source behavior. [Client contract](../../../../game-engine/docs/specs/trading-post.md) owns both-skin requirements. Monthly inventory has the explicit project-policy exception below; it is not a proven Retail calendar join.

## What it must do

- [x] Monthly progress uses real committed achievement events and current-account contributions; login reconstruction cannot generate historical credit.
- [x] Calendar rollover is 00:00 UTC on the first, independent of the daily-reset hour; the existing injectable ServerClock controls it.
- [x] Sourced default PerksActivityThreshold group1 awards once per account/month. Balance persists across month/relog; cap resets, not earned currency. Lead-approved October12 policy replaces the earlier daily-reset-hour rule.
- [x] Atomic paid collection grants, receipt-bounded refunds, one frozen item and source-backed capital interaction.
- [x] Retail cart checkout commits the complete selected offer list, Tender charge and per-item receipts atomically; any rejected offer leaves every charge/grant unchanged. Each receipt retains the existing individual refund policy and survives reopen. Batch publication sends each changed collection family once and includes every affected vendor/collection-alias row in the partial reply.
- [x] Trading Post requests settle current monthly gameplay credit without publishing an unsolicited full Adventure Guide snapshot ahead of the vendor reply.
- [ ] Every Query/Purchase/Checkout/Refund/Freeze/Balance snapshot identifies the request's actual reply kind, including refusal and persistence-error replies. Preserve existing FIFO/authority behavior; no request ID or client-side kind inference.
- [x] Zero-duration refund default leaves refund unavailable until an explicit policy is approved; no duration guessed.
- [x] Vendor replies evaluate existing item class/race/level restrictions for the querying player, carrying optional `usable` state for the source filter. Missing item/player facts remain unknown; usability is not a new purchase restriction.
- [x] Item icons follow C_Item.GetItemIconByID: lowest-order authored ItemModifiedAppearance → ItemAppearance.DefaultIconFileDataID; items without an appearance icon use their authored Item.IconFileDataID. Empty direct item icons must not erase available base-appearance icons.

## Monthly inventory source policy (lead decision 2026-10-19)

- [ ] `TP_MONTHLY_CATALOG_WOWHEAD_NEWS_BEST_GUESS` permits monthly full-inventory Wowhead news for PerksMonths 2–46, including bonus rewards and outlet inventories. [`content/trading-post-monthly-catalog.json`](../../content/trading-post-monthly-catalog.json) records source URLs, retrieval dates, HTML SHA-256s, literal item-to-vendor joins and unmatched references. Split bonus/outlet news retains separate provenance; news linkage is not proof of exhaustive Retail membership.
- [ ] NPC-only links remain unconverted. Unmatched component links, including October 2026 collection masks, are not standalone products. No name-based item resolution or invented vendor IDs.
- [ ] The permanent set requires presence in every sampled month from first appearance through month 46; first-seen46 singletons are recorded separately rather than treated as observed persistence. Returning/outlet appearances alone confer no permanence. The 133 vendor IDs without interval links remain the named open question `TP_133_VENDOR_ITEMS_WITHOUT_INTERVAL_LINKS`, not an assumed permanent set.
- [x] Keep the tracked source JSON outside ignored/symlinked `data/`; deployment installs it beside `world.db`. Load that installed JSON, validating schema/policy/build/vendor references and provenance. Missing/invalid data disables catalog membership; no all-catalog fallback. An uncovered UTC month has an empty monthly list and `TP_MONTHLY_CATALOG_MONTH_UNCOVERED` source gap.
- [x] Full queries expose current-month membership plus the strict permanent set and the account's durable frozen item. Purchase, checkout and freeze validate against that same membership inside the existing FIFO after account-state loading; mixed-cart refusal is atomic. Refunds remain receipt-based even if the catalog is unavailable.
- [x] UTC first-of-month 00:00 rollover and the existing `now_unix`/`month_end_unix` wire fields remain authoritative. Frozen membership survives account/database reopen. Activity-month binding is not inferred.

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
- `crates/server/src/trading_post_monthly.rs`: adjacent monthly JSON validation and UTC membership.
- `crates/server/src/trading_post_persistence.rs`: fresh FIFO account membership validation and filtered snapshots.
- `scripts/retail_content.py`: retains sourced Trading Post high service flag.
- `admin_ipc.rs` `dispatch_trading_post`, `bin/admin.rs`: the two test/admin tools above.

## Tests asserting this spec

- `trading_post_store::tests::trading_post_thresholds_once_per_account_month_survive_database_reopen`.
- `trading_post_store::tests::trading_post_purchase_atomic_collection_grants_and_refund_survive_reopen`: all four collection stores, duplicate/insufficient-funds refusal, zero-duration/expiry/repeated refund and database reopen.
- `trading_post_reply_kinds_identify_reads_and_refused_actions`: real typed reads and refused actions preserve all actual reply kinds; the shared request helper also checks kinds on existing successful operations.
- `trading_post_tests.rs`: real gameplay death credit/Tender/midnight reset, real message dispatch and sourced vendor gossip, paid purchase/refund refusal/distance closure. Cart wire test refuses unknown offers without partial charge, publishes all paid rows and one final appearance update, then refunds both receipts.
- `trading_post_store::tests::trading_post_checkout_*`: durable two-product checkout, per-item refunds after database reopen, late owned/duplicate-offer rollback and empty-cart refusal.
- `trading_post_tests.rs` `trading_post_admin_tender_and_refund_window_enable_wire_refund`: admin grant by username (unknown account refused), administrator window, real wire purchase/refund restoring Tender and revoking the appearance.
- `bin/admin.rs` `parse_trading_post_setup_requests`.
- `scripts/tests/test_trading_post_npc.py`: production importer preserves authentic Seabraid flag and source spawn.
- `trading_post_monthly_tests.rs`: exact October42-offer query and UTC uncovered-month replacement; off-month purchase/freeze and atomic mixed-cart refusal; frozen database-reopen/purchase/receipt-refund; held-writer preceding-freeze→query/purchase FIFO freshness; missing/malformed/policy/build/vendor/provenance/permanent loader refusal. Five monthly cases and ten retained Trading Post controls pass together; both-skin native proof remains separate.
- Existing Adventure Guide live-event/alt projection tests.

## Known gaps (current cycle)

- [ ] Both-skin post-landing native monthly rollover/countdown/frozen proof; monthly inventory remains the approved news BEST_GUESS, not Retail parity.
- [ ] Seasonal activity/reward calendar binding remains unresolved.
- [ ] Refund duration sourcing/policy (the admin window is test setup, not policy).

## Out of scope

None of requested section excluded.
