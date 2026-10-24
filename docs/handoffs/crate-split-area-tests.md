# Per-area test pilot and rebuild measurements

Approved C pilot implemented atop repaired B `14c74e6fa`, based on landed A `8e7c7a685`; compiler boundaries qualified at `acecb97d1`; full-suite acceptance pending. Table counts describe the reviewed pre-lift files. Existing behavior and complete case/ignore inventory remain mandatory. [Exact 106-case mapping](crate-split-combat-test-mapping.json).

## C: native melee behavior first

Use `server-combat-tests`, following the test-only/dev-dependency pattern of `server-simulation`. Keep the original `combat::<module>::<case>` names. All 106 selected cases appear as passing in `/home/osso/.worktrees/logs/gate-cs1046-srv.log`; this is an actual baseline census, not a source-attribute estimate.

| Existing file under `crates/server/src/` | Cases | LOC |
|---|---:|---:|
| `combat_facing_tests.rs` | 6 | 235 |
| `combat_npc_cadence_tests.rs` | 7 | 241 |
| `combat_npc_expertise_tests.rs` | 6 | 317 |
| `combat_npc_flags_tests.rs` | 2 | 106 |
| `combat_npc_no_crit_tests.rs` | 11 | 328 |
| `combat_npc_victim_dodge_tests.rs` | 7 | 253 |
| `combat_npc_victim_parry_tests.rs` | 9 | 272 |
| `combat_npc_weapon_crit_tests.rs` | 10 | 320 |
| `combat_pet_engagement_tests.rs` | 4 | 141 |
| `combat_victim_caster_crit_tests.rs` | 16 | 422 |
| `combat_victim_general_crit_tests.rs` | 15 | 417 |
| `combat_victim_summoner_crit_tests.rs` | 13 | 404 |
| **Total** | **106** | **3,456** |

These files reference only ten `crate::` roots: auras, combat, combat_refs, maps, random, spell_cast, stats, test_data, world_data and zones. All are already public after B. Dependencies still need compilation proof; this is not a claim that every referenced member is public.

Keep `combat_enemy_dodge_tests.rs`, `combat_npc_general_crit_tests.rs`, `combat_npc_victim_block_tests.rs` and `combat_victim_health_crit_tests.rs` in core: they reach the private `creature_miss_chance`/`read_outcome_chances` implementation surface. Also retain the general `combat_tests.rs` tree. Do not publish private formulas merely to relocate cases.

### Boundary work

The first consumer check also identified real ordinary-spawn/state boundaries: `CreatureSpawnContext`, `spawn_creature`, `CreatureFlagsExtra` and `end_all_combat` need public visibility, without body changes. The consumer explicitly imports canonical `CombatTarget` and uses the same existing rand0.8.5 dependency as its original cadence fixture. These do not expose the excluded private chance formulas.

- [x] Warned path owners and used `git mv` for the twelve files in pure-move commit `c765d0279`; names/ignore attributes retained.
- [x] Lifted `collect_swings` and `collect_broadcast_swings` into existing `spell_cast::test_harness` in a separate pure-body-move commit `94e9357e5`. Wiring changes only visibility and canonical combat paths. Both consumers use those implementations; `attack_stop` stays private.
- [x] Exposed only `harness_with`, `world_db`, its existing return struct and `powers_db` path field, plus the two canonical collectors. Other cache fields and `drain` remain private. Collector-only imports removed from moved sources.
- [x] Added the empty-normal-library/dev-dependency consumer and equivalent opt2/debug1 profile. Combined core-unit/simulator/combat-target check passed42.85s; production-no-feature `game-server` check passed14.59s, both without diagnostics at `acecb97d1`.
- [ ] Obtain parent full-suite/exact name reconciliation after B lands; expected core5763passed/4known failures/202ignored, simulator30/91, combat106/0. Workspace6004/4/314 remains unchanged.
- [ ] Measure one same-source test-only edit before expanding to further areas. No compile-time percentage follows from LOC reduction alone.

Excluded scope: gameplay algorithms, timers, RNG, damage assertions, data files, new test ignores, transport substitutions and production module moves. Existing protocol, Bevy components and server resources retain one type identity.

Toys has a larger test-only surface (15,193 LOC at the original graph pin), but 37 direct `crate::` roots and terrain/persistence fixtures. It is not the lowest-boundary pilot. Spell-cast/follower/scripted-AI suites remain later candidates after this pattern is proven.

## Quiet-window B measurement

A full-suite gate is not a rebuild-speed measurement. Hold gameplay fixed and compare incoming-master `eec5b6bcf` to the accepted B code tip; do not compare unrelated gameplay revisions or attribute the old a28 measurement to B.

- [ ] Obtain a parent-owned quiet window; drain existing compilers and use one build at a time. Preserve `CAPACITY_OWNER` in both admission and actual command environments. The existing matrix runner strips it, so inspect/fix the diagnostic copy before use, not the shared runner during other measurements.
- [ ] Use opt2/debug1 and `-j2` for both variants, the identical frozen protocol revision, and explicit overrides for shared, lightyear_netcode and lightyear_udp. Preserve the min-start14GiB, stop-below6GiB/300GiB safeguards. No automatic retry after a safety abort.
- [ ] Warm each variant's unmodified source once. Verify byte equality of the chosen parity source across its old/new paths, append the same single comment, then measure no-run rebuilds of old `server --lib` versus new `server-simulation --lib`. The target change is intentional; source/workload/profile must otherwise match.
- [ ] Record semantic spell and aura body edits as separate trials, not as parity-comment timings. Apply identical bounded edits to both source variants, prime restored source between trials, and restore source in `finally`. Do not run gameplay tests or deploy edited binaries. Restore/rebuild artifacts before any later runtime proof.
- [ ] Save exact revisions, commands, source hashes, wall/CPU time, aggregate memory/swap, safety outcomes, Cargo timings and incremental reuse evidence. Reuse claims require matching CGU artifact identities; elapsed time alone does not establish query reuse.

Use the existing compile-matrix runner and its README under `/home/osso/.worktrees/logs/`; diagnostic results belong in persistent `game-engine/data/diagnostics/`. Priming duration is unknown until measured. Schedule two primers plus two measured comment rebuilds first; semantic trials are additional work, reported separately. Do not clear/copy bulk target caches to manufacture parity, and do not reuse older aborted runs as successful results.

## Proof references

- [B exact baseline mapping](crate-split-test-mapping.json): 123 old subtree names and their mapped package/target/name/ignore identities; workspace expected totals6004 passed/4 known failures/314 ignored after the four eec5b6bcf additions.
- [Server crate ownership](../wiki/systems/server-crates.md).
- `crates/server/src/{combat.rs,combat_*_tests.rs}`: module declarations, actual event-boundary assertions and retained private-helper accesses.
- `crates/server/src/spell_cast/test_harness.rs`: existing cached input/harness and collector destination.
