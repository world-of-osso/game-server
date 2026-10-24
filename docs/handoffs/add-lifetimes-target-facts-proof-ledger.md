# Exact fixture add lifetimes and target facts

## Scope

Main target retains the fixed300s clock/boss role. Ten add waves use independent30s lifetimes at0,30.01,…270.09s, with nine real10ms gaps. Native50ms frames split only at expiration/spawn boundaries; timestamps are not rewritten. Creature31146 templates, references and production dummy damage remain unchanged. Broad integration and all-owner3T remeasurement belong to lead.

Sources: pinned SimC76f8118 `sc_enemy.cpp1053–1101`, `raid_event.cpp2700–2701,3068–3141,3180–3206`. [Contracts](../specs/dps-parity.md#measurement), [native facts](../specs/simc-apl.md#shared-native-facts).

## Behavioral proof

Diagnostics root: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/arms-parity-2026-10-23/`.

| Revision / command filter | Result | Evidence |
|---|---|---|
| c88e6749e / `dps_parity_fixture_add_ --ignored --nocapture --test-threads=1` | Genuine2FAIL,19.63s runtime; all20 phase observations exercised. First replacement30.000 versus30.010; adds inherit global Health/TTD instead of each summon lifetime; missing target/role facts | `lifetimes-current-red.log`, `.proof.json` |
| c88e6749e / `arms_native_apl_ --nocapture --test-threads=1` | Genuine4FAIL,8.90s: unknown selected Health, caster-owned debuff remains, absent-add facts and named dungeon timing refusal | `target-facts-current-red.log`, `.proof.json` |
| Earlier `lifetimes-red` | Invalid setup-only failure: stale private snapshot lacked `trait_definition_effect_points`. NOT behavioral RED | Historical log retained |
| d07db700c / `dps_parity_fixture_add_ --ignored --nocapture --test-threads=1` |2GREEN27.20s; actual10starts/20Health+TTD+role phases/nine10ms gaps. Compile24m14,0warnings | `lifetimes-green.log`, `.proof.json` |
| d07db700c / `simc_apl_native:: --nocapture --test-threads=1` |10GREEN16.45s:4new target cases +6 existing native controls | `target-facts-native-controls-green.log`, `.proof.json` |
| d07db700c / `dps_parity_fixture_fixed_time_main_health_ --ignored --nocapture --test-threads=1` |1GREEN11.98s; main clock retained through fight end | `lifetimes-main-clock-green.log`, `.proof.json` |
| d07db700c / `dps_parity_engagement_all_opponents_ --ignored --nocapture --test-threads=1` |1GREEN18.23s; native mutual combat refs retained across actual unsummon/spawn | `lifetimes-engagement-control-green.log`, `.proof.json` |

Both genuine RED commands reused the exact compiled c88e6749e binary recorded by `lifetimes-current-binary.proof.json`; no stale-binary selection. Read-only SQLite backup of current canonical: `world.arms-current-dd1de231e32c.db`, SHA256 `dd1de231e32c26291182dff050a8de5a5132908723ed21454eb128e45e4b05ae`. Canonical unchanged before/after backup: `568200017bcb3b5ed6f17931fc0994fff502c5c801f93f96197fc718bcd493bf`. Snapshot provenance: `native-current-backup.proof.json`. This is test provenance, not permission to reuse a frozen snapshot for later DPS measurements.

GREEN phases reused the single d07db700c executable SHA256 `3beb862d530a4467033d3ce40d37272ea61bb7ece53a30718e2d3ca135fb6278`; `lifetimes-green-binary.proof.json` records its exact path.14total passes/0failures; same recordeddd1de231e32c snapshot, no intervening code changes. These targeted proofs do not replace lead's integration gate.

## Current capability / remaining proof

| Capability | Implementation | Proof level |
|---|---|---|
| Actual expiration/spawn gaps and per-target Health/TTD/boss role | `FixtureTarget`, `FixtureAdds`, actual native clock boundary substeps; normal unsummon/spawn and combat refs | 2lifecycleGREEN plus1main-clockGREEN atd07db700c |
| Selected native target Health and action-explicit identity | Explicit action identity wins; untargeted Player reads native selection; fixture Follower reads its real player controller's selection | Selected Health GREEN; explicit per-target Health/TTD/role GREEN in20actual phase observations |
| Owned target aura `up/down/ticking/stack/remains` | Bound native spell ID and actual selected target; filters actual caster and expiration | Foreign/own native208086+388539 remains GREEN; bf56e1b4b also proves up/down/stack/ticking and actual388539 expiry with both units alive |
| Fixture `raid_event.adds.exists/remains/in/cooldown/duration/count` | Explicit event state; pinned absent-event max-timespan semantics; dungeon timing fails explicitly | Absent-event/named dungeon refusal GREEN; bf56e1b4b proves all six live expressions at every actual frame/boundary, including20phase samples and nine gaps |
| FIRST/cycle iteration, action-relative dots, refreshable, active_dot, applied persistent multipliers and composites | Not implemented by this slice | Remain open; no inferred snapshot or invented timer |

## Follow-up acceptance (separate branch)

`arms-target-facts-followup` strengthens only tests: reads all six live add-event expressions at every actual native frame/boundary (including20phase samples/nine gaps), checks own/foreign aura up/down/stack/ticking, and advances the actual native Rend expiration while both caster/recipient stay alive. No new timer or production behavior. At bf56e1b4b, `arms_native_apl_`5GREEN19.06s and `dps_parity_fixture_add_health_and_ttd_ --ignored`1GREEN20.32s;13m59compile/0warnings. Receipts: `target-acceptance.proof.json`, `live-add-event-acceptance.proof.json`; exact executable SHA256 `049c94fc7668dee7276951be96b7821ce4e4d5289ed04159f76f8480a6b268ad` in `target-acceptance-binary.proof.json`, which also resolves the harness DB to the same dd1de231e32c snapshot. Prior disjoint14-case production proof remains unchanged;15unique cases covered across original/follow-up runs, not20 distinct tests. No redundant full/unchanged cohort rerun. Explicit target-list/current-target API, active_dot, refreshable, casting.react and movement-event extensions remain next scope.

New test receipts must name the exact committed revision, command, binary and snapshot. Later source changes invalidate only intersecting proof scopes. No native DPS qualification claimed.
