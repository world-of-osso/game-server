# Headless keystone runner

The runner executes a selected keystone dungeon and a five-spec class party in the transport-free dungeon simulator. It uses the real follower APLs, Mythic population, Font activation, challenge lifecycle and configured scaling. It writes one versioned JSON run record, including an unsuccessful run, without administrative damage or synthetic completion.

## What it must do

- [ ] Accept a dungeon, keystone level and explicit tank, healer and three damage specialization IDs. Reject unsupported maps, profiles, roles and levels explicitly.
- [ ] Form the class party with the selected key-band equipment and talents; enter the Mythic copy, activate the owned key at its Font and run the native APL and dungeon systems.
- [ ] Emit a record with `schema_version`, dungeon map and challenge IDs, key level and affixes, party members' specialization IDs and native equipped item levels, and terminal status and reason.
- [ ] Record the timer limit, elapsed time including configured death penalties, in-time result and awarded upgrade when a run completes. Use null when the result is not established; do not report a failed run as a completion.
- [ ] Record each party death with simulated elapsed milliseconds, victim, killer NPC entry and killer spell when actually known; use null for an environmental, melee or unavailable cause.
- [ ] Choose an input route consisting of the authored boss route plus enough weighted trash packs to meet the challenge's force criterion. Greedily select reachable packs by additional native-graph travel distance per force point; include the chosen route and planned forces in the run record.
- [ ] Record each pull's start and duration from actual party combat transitions, and per-actor damage and healing by specialization. Attribute controlled-unit output to its owner and state whether amounts are effective or raw.
- [ ] Derive timestamps from the simulation clock, not wall-clock combat-log timestamps. Keep numeric challenge policy in the existing configuration.
- [ ] Prove a complete low-key Azure run using real APLs and the landed key-level scaling. A timeout, wipe, blocked route or partial record is not a clear.

## Run-record schema, version 1

The JSON object has `schema_version`, `map_id`, `challenge_id`, `key_level`, `affixes`, `party`, `route`, `outcome`, `deaths`, `pulls` and `actors`. Entity IDs are decimal strings. Each party member carries `spec_id`, per-slot item IDs and native `level`, plus their arithmetic mean; that mean is not a Retail UI item-level calculation. `route` contains ordered `steps`, `added_packs` and `planned_forces`.

`outcome` contains `status`, `reason`, `elapsed_ms`, `timer_ms`, `in_time`, `upgrade_levels` and `resulting_key_level`; completion-only values are null for non-completions. A death contains simulated `elapsed_ms`, victim ID and spec, and nullable `killer_npc_entry` and `killer_spell_id`. Pulls contain simulated `start_ms`, `duration_ms` and `ended`; an open terminal pull has `ended: false`. Actor totals contain raw and effective damage and healing. Effective amounts subtract overkill or overhealing. Controlled-unit amounts are attributed to the owning party member.

## How it works

- [Dungeon simulator](../wiki/systems/dungeon-simulator.md).
- [Mythic+ lifecycle](mythic-plus.md).

## Implementation inventory

- `crates/server-combat/src/followers/simulator.rs` — explicit class-party selection and key-band production spawning.
- `crates/server-combat/src/mythic_plus_headless.rs` — isolated bag, authored Font, real challenge lifecycle and simulated ServerClock bridge.
- `crates/server-combat/src/spell_cast/test_harness.rs` — opt-in fatal-spell capture for observed death events, without changing wire combat logs.
- `crates/server-simulation/src/dungeon_simulator/keystone_route.rs` — deterministic weighted force-pack route planner.
- `crates/server-simulation/src/dungeon_simulator/keystone_record.rs` — schema and event-based simulation-time aggregation.

## Tests asserting this spec

- `mythic_plus::tests::headless_keystone_uses_authored_font_and_real_clock` checks actual Font activation, key depletion and clock advancement.
- `followers::profile_behavior_tests::native_follower_lethal_spell_is_captured_without_changing_wire_death` checks fatal-spell provenance and unchanged wire output.
- `dungeon_simulator::runtime::keystone_simulator_spawns_selected_class_party_with_real_apls` checks the selected specs, equipped items and actual APL registration.
- `dungeon_simulator::keystone_route::tests` checks greedy route choice and the Azure source-force threshold.
- `dungeon_simulator::keystone_record::tests` checks the record shape, event timing, raw/effective amounts and controlled-unit attribution.

## Known gaps (current cycle)

- [ ] The existing Azure simulator selects the Normal follower population and Journal gear. Its historical Azure cohorts did not establish a clear.
- [ ] Native combat-event timestamps are wall-clock values and cannot serve as accelerated simulation time.

## Out of scope

- Fabricated scaling, timer, affix, loot, kill or scoring rules; the runner uses sourced catalog data and existing configuration.
