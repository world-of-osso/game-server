# Quest sharing, resets and timed quests

Authoritative quest runtime extends `quest_requests.rs`, `quest_rules.rs` and redb quest persistence. [Persistence and clock design](../wiki/systems/quest-share-resets.md).

## What it must do

- [x] Only party members with an active authored sharable quest may push. Recipients explicitly accept/decline server-issued offers. Revalidate source quest, party membership and existing eligibility rules; replay cannot grant twice.
- [x] Return supported Retail sender/recipient results. Reject busy recipients, dead recipients and cross-faction sharing.
- [x] Persist per-character completion timestamps. Daily/weekly quests become eligible at the local realm reset, preserving prerequisite history and unlocking their exclusive alternatives.
- [x] Use the existing local instance schedule: daily 15:00 UTC, weekly Tuesday 15:00 UTC (`maps/locks.rs`), not a universal Retail-region claim.
- [x] Load authored TimeAllowed seconds; start timer on acceptance, preserve deadline offline, inherit it on sharing, fail at expiry and prohibit reward. Only one unfinished timed quest; abandonment permits retry.
- [x] Migrate legacy progress/watch/reward history transactionally. Stamp unknown legacy completion time once; legacy timed progress with no recoverable start time fails, not a fresh timer.

## How it works

- [Quest reset/timer persistence](../wiki/systems/quest-share-resets.md)

## Implementation inventory

- `crates/server/src/quest_requests.rs` — pushes, offer responses, acceptance and timestamped reward.
- `crates/server/src/persistence_quests.rs` — redb schema migration and deadlines.
- `crates/server/src/quest_time.rs` — injected ServerClock and expiry before requests.
- `crates/server/src/quest_rules.rs` — reset/exclusive-group eligibility and reward checks.
- `crates/server/src/{quest_data,quest_view,quest}.rs` — authored duration, timer samples and scheduling.

## Tests asserting this spec

- `quest_rules_tests.rs`: daily/weekly exact injected boundary, prerequisite history, daily exclusive alternatives.
- `quest_tests.rs`: real party message path, accept/replay/rejection/party change, persistent timer expiry/relog.
- `persistence_quests.rs`: legacy migration and restart idempotence.
- Private live proof: both skins, two fb_* accounts, UDP5517; authentic quests62,26153,938. [Evidence](../wiki/systems/quest-share-resets.md).

## Known gaps (current cycle)

No remaining requested behavior gap. Not every individual rejection branch has an isolated behavioral fixture; the common eligibility/message paths are covered.

## Out of scope

New scripted escorts, quest pools, world quests, reputation/expansion/covenant gates, world.db mutation and realm deployment. Existing eligibility limitations remain explicit; no invented content.
