# FIFO persistence

Tick-triggered character, achievement/account and Adventure Guide persistence must not wait for durable disk writes. Sources are `crates/server/src/persistence_writer.rs`, `achievement_account.rs` and `persistence_runtime.rs`; see [implementation and restoration evidence](../wiki/investigations/server-tick-lag.md#fifo-persistence-core-restoration-2026-10-17).

## What it must do

- [ ] Preserve submission order through one durable writer and report commit failures explicitly.
- [ ] Queue owned character/account snapshots without waiting for a held database writer; preserve live account aggregation and character-scope contributions.
- [ ] Keep achievement contributions and corresponding guild news atomic, without duplicate news across pending updates or reopen.
- [ ] Queue the existing 15-second character save and Guide monthly/Tender persistence without changing their gameplay clocks.
- [ ] Drain ordinary shutdown; a crash must retain acknowledged writes and must not publish queued writes as durable.

## How it works

- [FIFO core restoration](../wiki/investigations/server-tick-lag.md#fifo-persistence-core-restoration-2026-10-17)

## Implementation inventory

- `crates/server/src/persistence_writer.rs` — FIFO jobs, durability receipts, failures and shutdown drain.
- `crates/server/src/persistence.rs` — database ownership and writer lifetime.
- `crates/server/src/achievement_account.rs` — pending account contributions and owned atomic character/account/news persistence.
- `crates/server/src/persistence_runtime.rs` — periodic, exit and departing-character saves; FIFO adaptation remains under verification.
- `crates/server/src/adventure_guide.rs` — monthly contribution and Tender settlement callers; full tick-path conversion remains under verification.

## Tests asserting this spec

- `crates/server/src/achievement_writer_tests.rs` — original `0ff7a921a` monthly contention reproduction and account contention control.
- `crates/server/src/persistence_writer_tests.rs` — receipts, FIFO snapshots and failure handling.
- `crates/server/src/persistence_tests.rs` — periodic-save contention and stored character/map/corpse state.
- `crates/server/src/achievement_tests.rs` — account migration, retained dates, character isolation and guild news.
- `crates/server/src/adventure_guide_tests.rs` — real monthly refresh under a held writer.

## Known gaps (current cycle)

- [ ] Complete and verify the minimal `b80131863` / `c391e8bbb` / `98cfbc490b` import against current master and landed netdelta.
- [ ] Verify periodic, achievement account, Guide month and tick Tender paths all use this writer.
- [ ] Verify FIFO crash/reopen and existing migration fixtures, then run the independently gated 11-peer release workload.

## Out of scope

Other unlanded performance topics (login off-thread, navigation, catalog/indexing, guild/companion/charter/PvP conversions) remain separate triage work. No second writer, netcode changes, gameplay timer changes or reduced population.
