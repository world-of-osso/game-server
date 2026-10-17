# FIFO persistence

Tick-triggered character, achievement/account and Adventure Guide persistence must not wait for durable disk writes. Sources are `crates/server/src/persistence_writer.rs`, `achievement_account.rs` and `persistence_runtime.rs`; see [implementation and restoration evidence](../wiki/investigations/server-tick-lag.md#fifo-persistence-core-restoration-2026-10-17).

## What it must do

- [ ] Preserve submission order through one durable writer and report commit failures explicitly.
- [ ] Queue owned character/account snapshots without waiting for a held database writer; preserve live account aggregation and character-scope contributions.
- [ ] Keep achievement contributions and corresponding guild news atomic, without duplicate news across pending updates or reopen.
- [ ] Queue the existing 15-second character save and Guide monthly/Tender persistence without changing their gameplay clocks.
- [ ] A batch of20 real WorldPortAck arrivals plus a due periodic save must remain below50ms with a250ms held writer, retain all destination maps/positions and submission-time health across FIFO drain/reopen.
- [ ] Drain ordinary shutdown; a crash must retain acknowledged writes and must not publish queued writes as durable.
- [ ] Trading Post requests execute after preceding Guide contribution/Tender settlement on the same FIFO; publish replies and collection changes only after their own commit. Same-tick earned Tender must fund a purchase and survive reopen.
- [ ] Preserve purchase/cart/refund/freeze and UTC-month rollover behavior, including snapshot settlement; no tick-thread write initialization disguised as a read.
- [ ] Guild-news queries/sticky requests and dirty subscriber publications read behind preceding achievement commits on that same FIFO; no early stale reply or consumed-but-lost notification.
- [ ] Login and missing-component Guide loads read character contributions from the writer-fed account cache. Raw default/migration writes are unavailable in production; ordinary reopen drains the prior writer.

## How it works

- [FIFO core restoration](../wiki/investigations/server-tick-lag.md#fifo-persistence-core-restoration-2026-10-17)

## Implementation inventory

- `crates/server/src/persistence_writer.rs` — FIFO jobs, durability receipts, failures and shutdown drain.
- `crates/server/src/persistence.rs` — database ownership and writer lifetime.
- `crates/server/src/achievement_account.rs` — pending account contributions and owned atomic character/account/news persistence.
- `crates/server/src/persistence_runtime.rs` — owned periodic/exit saves on the FIFO. Existing direct single-character/logout callers are not converted by this bounded import.
- `crates/server/src/adventure_guide.rs` — owned monthly contribution followed by a receipt-dependent Tender settlement.
- `crates/server/src/trading_post_persistence.rs` — owned validated actions, ordered snapshot settlement and commit-gated per-connection replies.
- `crates/server/src/trading_post_store.rs` — atomic store operations; account loading is read-only, including missing-table state.

## Tests asserting this spec

- `crates/server/src/achievement_writer_tests.rs` — original `0ff7a921a` monthly contention reproduction and account contention control.
- `crates/server/src/persistence_writer_tests.rs` — receipts, FIFO snapshots and failure handling.
- `crates/server/src/persistence_tests.rs` — periodic-save contention and stored character/map/corpse state.
- `crates/server/src/achievement_tests.rs` — account migration, retained dates, character isolation and guild news.
- `crates/server/src/adventure_guide_tests.rs` — real monthly refresh under a held writer.
- `crates/server/src/trading_post_tests.rs` — held-writer request responsiveness/no early reply, same-tick real earning→purchase→reopen and existing vendor/purchase/cart/refund/month-reset controls.

## Known gaps (current cycle)

- [x] Minimal `b80131863` / `c391e8bbb` / `98cfbc490b` import integrated with master b8b6c2f9d and landed netdelta;13 core controls pass within28 affected controls.
- [x] Periodic/account/Guide/Tender and Trading Post request ordering, crash/reopen and migration controls pass; corrected20-ACK same-frame/due-save/reopen control passes separately.
- [ ] Gate441 found one real timing failure (news fixture0vs1) and prompted a production-reader audit. Four runtime ordering REDs prove stale guild request, lost subscriber notification, blocked raw login initialization and stale Guide overwrite; all four now GREEN. Reader coverage22 controls plus the corrected original atomic-news control passes with0ignored; independent targeted re-gate/check/fmt pending.
- [ ] Post-landing11-peer plain-release acceptance.
- [ ] Exact PvP release20 stall attribution; ACK handler has no direct character save. Other synchronous callers are not inferred to be its cause.

## Out of scope

Other unlanded performance topics (login off-thread, navigation, catalog/indexing, guild/companion/charter/PvP conversions) remain separate triage work. No second writer, netcode changes, gameplay timer changes or reduced population.
