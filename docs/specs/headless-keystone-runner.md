# Headless keystone runner

The runner executes a selected keystone dungeon and a five-spec class party in the transport-free dungeon simulator. It uses the real follower APLs, Mythic population, Font activation, challenge lifecycle and configured scaling. It writes one versioned JSON run record, including an unsuccessful run, without administrative damage or synthetic completion.

## What it must do

- [ ] Accept a dungeon, keystone level and explicit tank, healer and three damage specialization IDs. Reject unsupported maps, profiles, roles and levels explicitly.
- [ ] Form the class party with the selected key-band equipment and talents, including recorded permanent and temporary enchants and gems; enter the Mythic copy, activate the owned key at its Font and run the native APL and dungeon systems. The route controller must wait for the native countdown to finish before admitting pulls.
- [ ] Emit a record with `schema_version`, dungeon map and challenge IDs, key level and affixes, party members' specialization IDs and native equipped item levels, and terminal status and reason.
- [ ] Record the timer limit, elapsed time including configured death penalties, in-time result and awarded upgrade when a run completes. Use null when the result is not established; do not report a failed run as a completion.
- [ ] Record each party death with simulated elapsed milliseconds, victim, killer NPC entry and killer spell when actually known; use null for an environmental, melee or unavailable cause.
- [ ] When the leader releases at the entrance and the surviving party is in a different navigation component, use the registered transport route to reunite them before resuming the dungeon.
- [ ] Choose an input route consisting of the authored boss route plus enough weighted trash packs to meet the challenge's force criterion. Count only source spawns hostile to the actual owner in the production copy. Greedily select reachable packs by additional native-graph travel distance per force point; include the chosen route and planned forces in the run record.
- [ ] Record each pull's start and duration from actual party combat transitions, and per-actor damage and healing by specialization. Attribute controlled-unit output to its owner and state whether amounts are effective or raw.
- [ ] Derive death and pull timestamps from the simulation clock, starting when the configured countdown ends, not from Font activation or wall-clock combat-log timestamps. Exclude countdown events from run statistics. Keep numeric challenge policy in the existing configuration.
- [ ] Prove a complete low-key Azure run using real APLs and the landed key-level scaling. A timeout, wipe, blocked route or partial record is not a clear.

## Run-record schema, version 1

The JSON object has `schema_version`, `seed`, `requested_specs`, `map_id`, `challenge_id`, `key_level`, `affixes`, `party`, `route`, `scaling`, `outcome`, `deaths`, `pulls` and `actors`. Unavailable values are null when initialization fails. Entity IDs are decimal strings. Each party member carries `spec_id`, per-slot item IDs and native `level`, plus their arithmetic mean; that mean is not a Retail UI item-level calculation. `route` contains ordered `steps`, `added_packs` and `planned_forces`. `scaling` records the number of actually scaled copy creatures and their observed damage multipliers.

`diagnostics` retains terminal native unit state, observed cleared source IDs, pull decisions and ordered `native_combat_events`. Each event has `elapsed_ms` from the simulated challenge clock; the embedded native event's own timestamp remains wall-clock time. `outcome` contains `status`, `reason`, `elapsed_ms`, `timer_ms`, `in_time`, `upgrade_levels`, `resulting_key_level`, observed `forces`, `forces_required` and native `bosses`; `in_time` and `upgrade_levels` are null for non-completions; `resulting_key_level` is present only when the persisted key was observed. A death contains simulated `elapsed_ms`, victim ID and spec, and nullable `killer_npc_entry` and `killer_spell_id`. Pulls contain simulated `start_ms`, `duration_ms` and `ended`; an open terminal pull has `ended: false`. Actor totals contain raw and effective damage and healing, effective damage taken, time with the actor's combat status active, and direct successful casts by spell. Effective amounts subtract overkill or overhealing. Controlled-unit damage and healing are attributed to the owner; cast counts are the actor's own.

## How it works

- [Dungeon simulator](../wiki/systems/dungeon-simulator.md).
- [Mythic+ lifecycle](mythic-plus.md).

## Implementation inventory

- `crates/server-combat/src/followers/simulator.rs` — explicit class-party selection and key-band production spawning.
- `crates/server-combat/src/mythic_plus_headless.rs` — isolated bag, authored Font, real challenge lifecycle and simulated ServerClock bridge.
- `crates/server-combat/src/spell_cast/test_harness.rs` — opt-in fatal-spell capture for observed death events, without changing wire combat logs.
- `crates/server-simulation/src/dungeon_simulator/keystone_route.rs` — deterministic weighted force-pack route planner.
- `crates/server-simulation/src/dungeon_simulator/keystone_record.rs` — schema and event-based simulation-time aggregation.
- `crates/server-simulation/src/dungeon_simulator/keystone.rs` — authored-route planning, Font approach and challenge session.
- `scripts/run-keystone-simulator.py` — explicit inputs and invocation of the headless test executable.

## Tests asserting this spec

- `mythic_plus::tests::headless_keystone_uses_authored_font_and_real_clock` checks actual Font activation, key depletion and clock advancement.
- `followers::profile_behavior_tests::native_follower_lethal_spell_is_captured_without_changing_wire_death` checks fatal-spell provenance and unchanged wire output.
- `dungeon_simulator::runtime::keystone_simulator_spawns_selected_class_party_with_real_apls` checks the selected specs, equipped items and actual APL registration.
- `dungeon_simulator::keystone_route::tests` checks greedy route choice and the Azure source-force threshold.
- `dungeon_simulator::keystone_record::tests` checks the record shape, event timing, raw/effective amounts and controlled-unit attribution.
- `dungeon_simulator::keystone::tests` checks explicit input parsing and failure-record behavior; its ignored full-run test executes the native keystone.
- `scripts/tests/test_run_keystone_simulator.py` checks CLI input forwarding and validation.

## Known gaps (current cycle)

- [ ] The existing Azure simulator selects the Normal follower population and Journal gear. Its historical Azure cohorts did not establish a clear.
- [ ] Native combat-event timestamps are wall-clock values and cannot serve as accelerated simulation time.

## Out of scope

- Fabricated scaling, timer, affix, loot, kill or scoring rules; the runner uses sourced catalog data and existing configuration.
