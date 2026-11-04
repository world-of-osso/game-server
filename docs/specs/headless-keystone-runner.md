# Headless keystone runner

The runner executes a selected keystone dungeon and a five-spec class party in the transport-free dungeon simulator. It uses the real follower APLs, Mythic population, Font activation, challenge lifecycle and configured scaling. It writes one versioned JSON run record, including an unsuccessful run, without administrative damage or synthetic completion.

## What it must do

- [ ] Accept a dungeon, keystone level and explicit tank, healer and three damage specialization IDs. Resolve the map's unique challenge and named challenge scenario from the imported catalog; reject unsupported or ambiguous maps, profiles, roles and levels explicitly.
- [ ] Use complete native Detour walk paths for simulated ground movement and live cohesion, reunion and chain-approach distance decisions. Use the frozen graph for route planning and registered transport selection, not as a fallback for an unreachable walk. Pursuers enter native cannot-reach handling; following units and pets record and report a blocked walk while holding and retrying, and an unreachable leader intent reports an error.
- [ ] Publish all headless navigation requests on the next fixed tick, regardless of transport installation order or worker timing. Reject attempts to change an installed publication mode.
- [ ] A leader approaching a pull continues along the native walk path while its hostile actions are blocked by native target range or line-of-sight checks. Each continued step uses the native unadmitted-hostile sight guard and holds before entering unsafe sight. When a blocking hostile belongs to a later planned step and its own approach has no unresolved blockers, promote that step first. If the harness stall bound expires while the route remains blocked, record a FactError identifying the blocker; the bound is a harness diagnostic, not gameplay.
- [ ] While any movement driver moves a follower (simulator travel or flight, charge, jump, point or waypoint movement, fear, Havoc retreat setup, melee return, flight follow), its APL `movement.distance` reads that driver's current destination; a native navigation route supplies it otherwise. An idle follower reads zero; a moving follower with no destination is a fact error, never zero.
- [ ] Install the same world.db/DB2 definition set server startup installs (`world_data::catalogs::load`, used by `WorldDataPlugin`), including across seeded-world catalog reuse; do not hand-pick a subset.
- [ ] Install the selected map's native LOS bake before spawning the keystone party or population. Fail with the map ID when its bake is missing instead of silently disabling creature assistance.
- [ ] Form the class party at the Mythic copy's ContentTuning level cap with the selected key-band equipment and talents, including recorded permanent and temporary enchants and gems; enter the Mythic copy, activate the owned key at its Font and run the native APL and dungeon systems. Do not use the follower-dungeon level cap. The route controller must wait for the native countdown to finish before admitting pulls.
- [ ] Accept a Unix schedule epoch for the headless affix week, defaulting to the fixed simulator epoch rather than wall clock. Record the epoch, dungeon map and challenge IDs, key level and affixes, party members' specialization IDs and native equipped item levels, and terminal status and reason.
- [ ] Record the timer limit, elapsed time including configured death penalties, in-time result and awarded upgrade when a run completes. Use null when the result is not established; do not report a failed run as a completion.
- [ ] Record each party death with simulated elapsed milliseconds, victim, killer NPC entry and killer spell when actually known; use null for an environmental, melee or unavailable cause.
- [ ] When the simulator Travel driver moves a following unit, `FollowOwner.moving` reflects actual movement; it is false when that unit stops or is held.
- [ ] A following or pursuing unit’s route-progress baseline belongs to its current planned destination as well as its target identity. When that destination moves, later progress on the new route prevents a false stall without counting a destination change itself as progress.
- [ ] Print and flush progress at the start and every 60 simulated seconds after the countdown: elapsed simulation time, forces, bosses defeated, deaths, current pack, route step, and the tank's position, state and follow-movement flag.
- [ ] When the leader releases at the entrance, discard the pre-death movement target. If the surviving party is in a different navigation component, the leader uses the registered transport route to reunite with them before resuming the dungeon; followers hold rather than driving reunion.
- [ ] Choose an input route consisting of the authored boss route plus enough weighted trash packs to meet the challenge's force criterion. When extraction provides no authored route, derive the ordered boss path from the graph's unique sourced encounter bindings and reject missing, ambiguous or unreachable bosses. Bosses listed for the map in `content/dungeons/wcl-boss-first-damage.json` take their slots in WCL first-damage order (INFERRED_WCL); every listed report must agree, and Ruby opens its final encounter on Erkhart before Kyrakka. Build the graph from the catalog-validated copy difficulty and count only living, currently attackable hostile source spawns for the actual owner in the production copy. Greedily select reachable packs by additional native-graph travel distance per force point, inserting none after the final boss so forces finish before the end encounter. Route ordering must respect sourced transport gates: a destination unreachable without a gated transport follows its required boss; include the chosen route and planned forces in the run record.
- [ ] Before approaching a remaining boss, pull live hostile packs whose sight intersects a party member's complete approach path, except packs in the boss's own [sight pull unit](follower-dungeons.md#boss-sight-pull-units). Derive these from native exposure geometry; do not skip unrelated blockers when enemy forces are complete.
- [ ] In Blinding Vale, make the inferred Petalwing activator available only after Lightblossom Trinity and Ikuzz are defeated. Each rider uses DB2 TaxiPath 10799 independently; followers reach and activate it through native transport following. The activator template, placement and interaction are inferred because the authentic spellclick and vehicle data are unavailable; retire the inference when those data are sourced.
- [ ] Chain consecutive trash packs according to the WCL evidence registered for the map. Den uses boss-relative median target counts in `content/dungeons/den-wcl-pulls.json`; Ruby and Altar of Fangs use inferred first-damage source membership in `content/dungeons/ruby-wcl-pulls.json` and `content/dungeons/altar-wcl-pulls.json`. With source membership, the planned route visits packs in WCL pull order; an unobserved pack keeps the pull of the pack planned before it, trash precedes a boss of the same pull, and a pull order that changes the boss route is rejected. Unmatched packs stay separate. The owner tags each new pack while earlier packs remain alive; followers do not initiate pulls. Before a chain tag, the tank must be within its native opening range and be the active victim of most live engaged enemies. The owner returns to the tank after the tag; a chain approach must remain within opening range. Bosses remain separate route steps.
- [ ] For a data-backed healing encounter objective, approach its reachable route anchor and select its authored start gossip using native interaction range and validation. After the encounter starts, hold the objective route step while native healer APLs heal it and combat handles the adds. Never start it from proximity alone.
- [ ] Between fights, the leader may travel toward the next pack while the tank catches up. The leader waits for tank cohesion before tagging the pack.
- [ ] Record each pull's start and duration from actual party combat transitions, and per-actor damage and healing by specialization. Attribute controlled-unit output to its owner and state whether amounts are effective or raw.
- [ ] Derive death and pull timestamps from the simulation clock, starting when the configured countdown ends, not from Font activation or wall-clock combat-log timestamps. Exclude countdown events from run statistics. Keep numeric challenge policy in the existing configuration.
- [ ] Prove a complete low-key Midnight Season 2 run using level-90 key-band gear, real APLs and the landed key-level scaling. Compare boss durations with same-dungeon, same-key WCL kills. The level-70 Azure run with level-90 gear is mechanics-only, not balance proof. A timeout, wipe, blocked route or partial record is not a clear.

## Run-record schema, version 1

The JSON object has `schema_version`, `seed`, `requested_specs`, `map_id`, `challenge_id`, `key_level`, `affixes`, `party`, `route`, `scaling`, `outcome`, `deaths`, `pulls` and `actors`. Unavailable values are null when initialization fails. Entity IDs are decimal strings. Each party member carries `spec_id`, per-slot item IDs and native `level`, plus their arithmetic mean; that mean is not a Retail UI item-level calculation. `route` contains ordered `steps`, `added_packs` and `planned_forces`; retain it if later initialization fails. `scaling` records the number of actually scaled copy creatures and their observed damage multipliers.

`diagnostics` retains terminal native unit state, including character armor and native creature swing range and keystone damage multiplier, observed cleared source IDs, pull decisions and ordered `native_combat_events`. Each event has `elapsed_ms` from the simulated challenge clock; the embedded native event's own timestamp remains wall-clock time. `outcome` contains `status`, `reason`, `elapsed_ms`, `timer_ms`, `in_time`, `upgrade_levels`, `resulting_key_level`, observed `forces`, `forces_required` and native `bosses`; `in_time` and `upgrade_levels` are null for non-completions; `resulting_key_level` is present only when the persisted key was observed. A death contains simulated `elapsed_ms`, victim ID and spec, and nullable `killer_npc_entry` and `killer_spell_id`. Pulls contain simulated `start_ms`, `duration_ms` and `ended`; an open terminal pull has `ended: false`. Actor totals contain raw and effective damage and healing, effective damage taken, time with the actor's combat status active, and direct successful casts by spell. Effective amounts subtract overkill or overhealing. Controlled-unit damage and healing are attributed to the owner; cast counts are the actor's own.

## Den pull evidence

`INFERRED_WCL_PULL_REGIONS` records trash-pull target counts from three Den +2 reports, identified by report, fight and pull. Counts use distinct damaged NPC instances whose entries occur in the native graph; bosses and spawned totems and tombs are excluded. The three boss-relative segments have pooled medians of 18, 7 and 16 targets. Individual spawn groupings are not asserted: first-damage coordinates can differ from spawn positions, and one-to-one nearest-spawn matching had 90th-percentile distances of 37–133 yards across the reports. Replace this inference when exact retail spawn-group membership and pull routes are available.

## Ruby pull evidence

`INFERRED_WCL_FIRST_DAMAGE_MEMBERSHIP` maps report `wKpcfaj4MzDH9v8X`, fight 1, pull windows to native source GUIDs using one-to-one minimum-distance matches by entry within 50 horizontal yards. First damage may occur after movement, so membership is inferred. Sources without a match stay separate; replace this evidence when authored pull membership is available.

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

- [ ] Kings' Rest route planning aborts at 291/608 forces. The 30-point game-event criterion 64192 has no identified producer or proven route prerequisite.
- [ ] Temple of Sethraliss route planning aborts at 681/687 forces. The 5-point game-event criterion 63063 has no identified producer, multiplicity or proven route prerequisite.
- [ ] The existing Azure simulator selects the Normal follower population and Journal gear. Its historical Azure cohorts did not establish a clear.
- [ ] Native combat-event timestamps are wall-clock values and cannot serve as accelerated simulation time.

## Out of scope

- Fabricated scaling, timer, affix, loot, kill or scoring rules; the runner uses sourced catalog data and existing configuration.
