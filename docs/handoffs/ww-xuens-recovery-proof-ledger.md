# Windwalker Xuen's Battlegear FoF recovery

Approved2026-10-24: `SIMC_WW_XUENS_BATTLEGEAR_FOF_RECOVERY`, third after Flurry and SCK. Separate `ww-xuens-recovery` starts at origin/master4996e5967. SCKc743e921e/sk1106 and Flurry889cfbf6c/fl1103 remain frozen; each has its own lead gate.

## Named policy, not a native recovered relationship

SimC76f8118cf3f7f41dad20708e8328e1f4ab2deebc `engine/class_modules/monk/sc_monk.cpp954–958` requires primary (`!state->chain_target`) and `RESULT_CRIT` before reducing existing FoF recovery by talent effectN(2). Native392993 zero-based effect1 supplies4000ms. RWK damage inherits that same `rising_sun_kick_t::base_damage_t` through its alias at1143–1150; native468179 provides a real cone secondary boundary, whereas normal185099 is single-target. Do not substitute mutable CombatTarget for actual impact TargetInfo order.

SimC1541–1546 records unique impacted actor IDs without a damage-result predicate. Its1596–1609 expiration callback caps count at effectN(5)/effectN(4), adjusts existing recovery and clears the set. Native zero-based effect3=500ms and effect4=2500ms give at most5 actors/2500ms. Native parent101546/child107270 own actual channel timing, geometry and damage; no manual SimC pulse schedule is imported.

**TC/DB2 absence:** pinned TC a352b1fa `spell_monk.cpp` has no392993/Battlegear binding under the source packet's recorded search terms. Native392993 effects1/3/4 are aura4 dummy values with EffectTriggerSpell0; they do not encode the conditional critical-primary/unique-expiration relationship. Current `tdb_spell_proc` has no392993 row. Native ProcChance101/ProcTypeMask4112 remain unchanged; their presence is not an execution link. The conditional consumer is explicitly the approved SimC named policy. Native effects0/2 are not reinterpreted here.

Exact excerpts, raw rows, source hashes and absence scope: `game-engine/data/diagnostics/ww-parity-2026-10-23/xuen-recovery-source-packet.json`.

## Native proof matrix

| Boundary | Current proof |
|---|---|
| Actual primary normal RSK and inherited RWK critical; exactly one native4000ms reduction | f8af512e0 genuine2RED: actual0/1ms projected rounding versus4000ms |
| Actual noncrit, secondary-only crit and unselected normal RSK | Noncrit/unselected PASS; secondary-only joint-outcome sampling still pending |
| Real SCK repeated impacts on1/3/6 distinct actors; native500ms per actor/max2500 | f8af512e0 genuine3RED: actual0 versus500/1500/2500ms |
| No-target SCK, unselected SCK and absent FoF timer | f8af512e0 three controls PASS; absent-timer case strengthened to assert repeated real impacts |
| Later no-hit parent must not reuse an earlier unique set | f8af512e0 genuineRED at first500ms recovery precondition; later no-reuse assertion not reached yet |

The tests earn Chi through actual Tiger Palm, earn RWK through native Combo Breaker/Blackout Kick, cast real FoF to create its timer, and observe native outcomes without changing crit/stats/powers/coefficients. Unselected125017 is removed from the declared profile before native progression. A read-only test-support observer records actual impact-resolution TargetInfo order, including misses; it changes no selection, defense or combat outcome. It is not a gameplay policy implementation. Millisecond cooldown projection permits at most2ms comparison tolerance.

At f8af512e0 the native suite produced six genuine recovery REDs/five controls PASS plus one sampling-premise failure in52.95s; compile615.53s/zero warnings. Source data and actual outcomes are under `native-ww-xuen-red-f8af512e0/`. d9a66e6b8 records each of eight real earned RWK trajectories and their native critical factors: focus chance50.7174%, primary noncrit occurred twice but never with a secondary critical. This is not a gameplay RED. The bound now permits32 continuing native cycles without resetting the clock/RNG/resources or altering any stats, and that control is retested before implementation. Its diagnostic receipt is `native-ww-xuen-secondary-d9a66e6b8/`.

Initial ccb16df6b compiler attempt stopped at a test-only private PowerType import (E0603); no native case executed and this is not RED evidence. After that job exited, origin/master3f897bd38 was merged, including the production cooldown_query reader fix. The test imports shared::components::PowerType; production code must never depend on test-gated native_apl. The lead's full gate now includes plain `cargo check -p server --bins` in addition to test-support evidence.

No production recovery implementation precedes genuine native RED. Full lead gate and broader WW numerical parity remain separate/open. No worker push or spawned agents.
