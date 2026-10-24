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
| Implementation GREEN | Pending; no pass claim | Await current committed-source build |

Both genuine RED commands reused the exact compiled c88e6749e binary recorded by `lifetimes-current-binary.proof.json`; no stale-binary selection. Read-only SQLite backup of current canonical: `world.arms-current-dd1de231e32c.db`, SHA256 `dd1de231e32c26291182dff050a8de5a5132908723ed21454eb128e45e4b05ae`. Canonical unchanged before/after backup: `568200017bcb3b5ed6f17931fc0994fff502c5c801f93f96197fc718bcd493bf`. Snapshot provenance: `native-current-backup.proof.json`. This is test provenance, not permission to reuse a frozen snapshot for later DPS measurements.

## Current capability / remaining proof

| Capability | Implementation | Proof level |
|---|---|---|
| Actual expiration/spawn gaps and per-target Health/TTD/boss role | `FixtureTarget`, `FixtureAdds`, actual native clock boundary substeps; normal unsummon/spawn and combat refs | RED reproduced; GREEN pending, including main-clock control |
| Selected native target Health and action-explicit identity | Explicit action identity wins; untargeted Player reads native selection; fixture Follower reads its real player controller's selection | RED reproduced; GREEN pending |
| Owned target aura `up/down/ticking/stack/remains` | Bound native spell ID and actual selected target; filters actual caster and expiration | Foreign/own native208086+388539 RED reproduced; GREEN pending |
| Fixture `raid_event.adds.exists/remains/in/cooldown/duration/count` | Explicit event state; pinned absent-event max-timespan semantics; dungeon timing fails explicitly | Absent-event/dungeon RED reproduced; live lifecycle checks pending |
| FIRST/cycle iteration, action-relative dots, refreshable, active_dot, applied persistent multipliers and composites | Not implemented by this slice | Remain open; no inferred snapshot or invented timer |

New test receipts must name the exact committed revision, command, binary and snapshot. Later source changes invalidate only intersecting proof scopes. No native DPS qualification claimed.
