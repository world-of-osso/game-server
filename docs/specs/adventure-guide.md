# Realm Adventure Guide

Authoritative Suggested Content and Traveler's Log snapshots for the [client contract](../../../game-engine/docs/specs/adventure-guide.md). Retail Mainline sources are installed `data/db2/active/AdventureJournal.csv`, imported Perks tables and the existing world.db criteria catalog. Missing/malformed CSV is an explicit source error, never a fallback to the old SQLite AdventureJournal payloads. [Runtime](../../../game-engine/docs/wiki/systems/adventure-guide.md#realm-activities-and-suggestions).

## What it must do

- [x] Authenticated in-world queries return source rows or explicit source errors; never invented suggestions/progress.
- [x] Send a full Guide on initial login/reconnect, existing explicit queries, and changed content. Send only `AdventureGuideClock` when the countdown time alone changes; suppress unchanged refreshes. Preserve every monthly activity, tracking flag, suggestion and error.
- [x] Apply supported PlayerConditions; hide/count unsupported suggestion gates. Eligible authored cards remain display-only; unsupported activations are absent and counted as a subset of visible cards.
- [x] Apply the [World of Osso monthly access/reset policy](../../../game-engine/docs/specs/adventure-guide.md#world-of-osso-monthly-policy) to every character; preserve tracking and reset only monthly counters.
- [x] Reuse existing criteria tracking, attribute real events once across overlapping activity trees, persist/account-aggregate contributions, and exclude historical login reconstruction from monthly credit. Unsupported criteria show unavailable, not zero.

## Suggested provider

- Load the installed CSV records and evaluate supported primary PlayerConditions. ClientExecutable is metadata; CurrentPvpFaction0 is unset. Nonzero unknown faction/other predicates remain unknown, never met.
- Evaluate authored bonus values separately from primary eligibility; zero-valued bonuses cannot affect rank. Reuse existing earned character/account achievement IDs for the single-achievement predicate (including33559/39935); missing authority is not an unmet or fulfilled condition.
- Current source has equal PriorityMin/Max. Reject an unverified range rather than invent its weight. **Named ordering inference:** descending fixed authored priority plus met bonuses, then ascending ID; Retail Lua3159 forwards opaque C_AdventureJournal output without a client sort. This is not a claim about its hidden rotation/selection algorithm.
- Return the ranked eligible vector; the existing client maps its first three records to lower/upper/large source panes. Unknown nonzero bonuses and AdventureJournal flags remain censused and excluded; do not fill slots by bypassing gates. Type/LFG/quest/battleground metadata grants no activation.
- Frame and provider require joint both-skin native proof with all three panes populated. Current bounded proof, source/binary identities and remaining gaps are recorded in the linked runtime page; it does not certify opaque Retail rotation or unsupported predicates.

## Journeys watched faction

Renown requests use Faction IDs, not Covenant IDs, for Retail`C_Reputation.SetWatchedFactionByID`. Zero clears the existing character reputation watch. Queries read live`CharacterReputation`; accepted writes persist through the existing reputation store. Refusals retain both the previous watch and catalog, with an explicit watch error. Unevaluated unlock gates never establish eligibility; all29 current source factions remain gated. The existing owner-only ReputationStateUpdate carries a small normalized watched-bar payload on login, watch/clear acknowledgements and existing reputation updates. It reads persisted renown standing/threshold separately from the reward catalog, or standard reputation tier progress, and clears explicitly with None. This is backing, not certification of native hover controls or the [main-screen reputation bar](../../../game-engine/docs/specs/reputation-status-bar.md).

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
