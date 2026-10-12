# Realm Adventure Guide

Authoritative Suggested Content and Traveler's Log snapshots for the [client contract](../../../game-engine/docs/specs/adventure-guide.md). Retail Mainline sources are the pinned local AdventureJournal/Perks tables and existing world.db criteria catalog. [Runtime](../../../game-engine/docs/wiki/systems/adventure-guide.md#realm-activities-and-suggestions).

## What it must do

- [ ] Authenticated in-world queries return source rows or explicit source errors; never invented suggestions/progress.
- [ ] Apply supported PlayerConditions; hide/count unsupported suggestion gates. Eligible authored cards remain display-only; unsupported activations are absent and counted as a subset of visible cards.
- [ ] Apply the [World of Osso monthly access/reset policy](../../../game-engine/docs/specs/adventure-guide.md#world-of-osso-monthly-policy) to every character; preserve tracking and reset only monthly counters.
- [ ] Reuse existing criteria tracking, attribute real events once across overlapping activity trees, persist/account-aggregate contributions, and exclude historical login reconstruction from monthly credit. Unsupported criteria show unavailable, not zero.

## How it works

- [Realm Adventure Guide](../../../game-engine/docs/wiki/systems/adventure-guide.md#realm-activities-and-suggestions)
- [Client monthly policy](../../../game-engine/docs/specs/adventure-guide.md#world-of-osso-monthly-policy)

## Implementation inventory

- `crates/server/src/adventure_guide*.rs` — source catalog, supported gates, UTC interval, request and criteria snapshot producer.
- `crates/server/src/achievement*.rs` — existing criteria engine and durable contributions.
- `scripts/adventure_guide_catalog.py` — read-only source preparer; emits SQL for an owned copy only.

## Tests asserting this spec

- `crates/server/src/adventure_guide_tests.rs`
- `scripts/tests/test_adventure_guide_catalog.py`

## Known gaps (current cycle)

- [ ] Source interval field and reward group calendar epoch remain unverified; approved all-source-activity policy is explicit, not an inferred month binding.
- [ ] Unsupported condition/actions/criteria remain censused; no synthetic RPE/quest activation or currency/reward grant.

## Out of scope

Trading Post storefront purchases are separate from Adventure Guide browsing/progress.
