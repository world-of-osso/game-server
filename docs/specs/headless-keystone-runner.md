# Headless keystone runner

The runner executes a selected keystone dungeon and a five-spec class party in the transport-free dungeon simulator. It uses the real follower APLs, Mythic population, Font activation, challenge lifecycle and configured scaling. It writes one versioned JSON run record, including an unsuccessful run, without administrative damage or synthetic completion.

## What it must do

- [ ] Accept a dungeon, keystone level and explicit tank, healer and three damage specialization IDs. Reject unsupported maps, profiles, roles and levels explicitly.
- [ ] Form the class party with the selected key-band equipment and talents; enter the Mythic copy, activate the owned key at its Font and run the native APL and dungeon systems.
- [ ] Emit a record with `schema_version`, dungeon map and challenge IDs, key level and affixes, party members' specialization IDs and native equipped item levels, and terminal status and reason.
- [ ] Record the timer limit, elapsed time including configured death penalties, in-time result and awarded upgrade when a run completes. Use null when the result is not established; do not report a failed run as a completion.
- [ ] Record each party death with simulated elapsed milliseconds, victim, killer NPC entry and killer spell when actually known; use null for an environmental, melee or unavailable cause.
- [ ] Record each pull's start and duration from actual party combat transitions, and per-actor damage and healing by specialization. Attribute controlled-unit output to its owner and state whether amounts are effective or raw.
- [ ] Derive timestamps from the simulation clock, not wall-clock combat-log timestamps. Keep numeric challenge policy in the existing configuration.
- [ ] Prove a complete low-key Azure run using real APLs and the landed key-level scaling. A timeout, wipe, blocked route or partial record is not a clear.

## How it works

- [Dungeon simulator](../wiki/systems/dungeon-simulator.md).
- [Mythic+ lifecycle](mythic-plus.md).

## Implementation inventory

- `crates/server-combat/src/followers/simulator.rs` — explicit class-party selection and key-band production spawning.

## Tests asserting this spec

- `dungeon_simulator::runtime::keystone_simulator_spawns_selected_class_party_with_real_apls` checks the selected specs, equipped items and actual APL registration.

## Known gaps (current cycle)

- [ ] The existing Azure simulator selects the Normal follower population and Journal gear. Its historical Azure cohorts did not establish a clear.
- [ ] Native combat-event timestamps are wall-clock values and cannot serve as accelerated simulation time.

## Out of scope

- Fabricated scaling, timer, affix, loot, kill or scoring rules; the runner uses sourced catalog data and existing configuration.
