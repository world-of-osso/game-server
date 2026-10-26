# Warlock periodic counters

Hellcaller Wither uses stacks as a counter, not as an ordinary periodic-damage multiplier. Both Affliction265 and Destruction267 share the [counter contract](../../specs/dps-parity.md#shared-hellcaller-wither-counters).

## Representation

The downstream `hellcaller_wither` module registers native445474 through `CounterOnlyDotStacks`. Native aura stacks, maximum, duration and tick phase remain intact. The generic periodic scheduler uses one payload regardless of that counter. Ordinary refresh contributes no new counter stack; an explicit native `SPELLVALUE_AURA_STACK` application still uses normal stack merging.

This is separate from [Destruction shard funding](spell-casting.md#warlock-periodic-shard-funding--2026-10-25), which is specialization-qualified, and from Affliction's Agony threshold RNG. Neither resource policy is a Wither damage multiplier.

## Cause and proof

Pinned SimC76f8118 `sc_warlock_actions.cpp1066` sets `dot_ignore_stack=true`;1174–1186 cancels the automatic refreshed-dot stack increment while preserving earned stacks. Native generic stacking previously did both: a real recast increased1→2, and ordinary noncritical tick damage increased6398→63979 when the native counter input changed1→10.

`wither_stack_tests.rs` reproduces both failures and passes after the source policy on both265 and267. It observes actual ordinary casts and periodic damage, including preservation of an existing eight-stack counter. Destruction funding, Affliction native mechanics/rotation and owned DoT-duration controls also pass. This proves the counter boundary, not Blackened Soul producers, collapse behavior or numerical DPS parity.

## Counter lifecycle

`hellcaller_counters` owns Chaos Bolt/Shadowburn landed-hit producers and actual UA1259790 applications. The additive `AfterAuraApplication` stage runs after successful storage, including refresh/replacement. It does not replace UA's `AuraScripts` expiry registration. Counter changes preserve Wither's expiry and periodic deadline.

Native TraitDefinition122451 resolves Malevolence talent430014 to actual442726. The post-cast hook uses442726; the native cast already applies its buff before granting its native stack amount and damage to targets with caster-owned Wither. Ordinary gains include its active-buff bonus and the source's global Bleakheart PRD. Seeds gates run on both gains and real Wither ticks. A positive collapse-budget trigger refreshes its own clock; a decrement does not.

Blackened445731 uses the native1000ms periodic driver with runtime permanent/max-counter state, not the catalog's2000ms lifetime. Each tick decrements Wither before native445736 damage, applies Mark of Xavius to that remaining counter, then consumes its independent collapse budget. It stops at Wither1 or when Wither disappears. The weighted Seeds threshold stream grants native Shard Instability/Flashpoint; it does not create a second shard pool. Native effect base values supply stack counts and gates. Damage-modified dummy amounts are not stack-count inputs.

Pinned behavior is in `sc_warlock_actions.cpp1119–1149,1208–1317,1544–1565,4328–4331,4807–4810,5534–5611`, `sc_warlock_init.cpp1441–1480`, and `proc_rng.cpp221–276`. The23% collapse,15% Bleakheart PRD and0.240 threshold increment are pinned RNG-model settings, not damage adjustments.

Native lifecycle RED is recorded in train `a203eec969` and `97012b5fc8`: UA and CB leave Wither1 unchanged, and collapse gates fail. The no-owned-Wither control passes. The Malevolence test initially used a damage-modified dummy amount; it now reads the raw native base value, as the source does. Probe `c77810573a` passes five lifecycle cases. Its Malevolence case incorrectly cast raw talent430014; it now uses the same native action resolution as the real driver and casts442726. Train `687ff619cf` lands `b6937d3bf310` with all six native lifecycle cases GREEN. The separate265-led `1636dfe495` also passes all six; three independently owned Affliction Calamity/Wrath cases remain RED. Three-seed267 means are94930.81ST and123087.79three-target (48.65%/47.94% of reference), not numerical parity.

## Seed of Corruption lifecycle

This is native27243/27285, not the separate Hellcaller Seeds of Their Demise counter. [Contract](../../specs/dps-parity.md#seed-of-corruption-lifecycle). The downstream `warlock_seed` leaf stores the snapshotted threshold in the actual owned Seed dummy effect2. Existing landed-damage callbacks consume that budget; native periodic/removal callbacks detach Seed and enqueue its unmodified explosion. The generic declarative `PreservePeriodicPhase` policy in the registry/refresh boundary keeps Seed's already scheduled tick; only27243 registers it. This is necessary because normal nonstacking/non-Pandemic refresh resets the timer. Without selected Sow196226, the leaf filters raw area selectors to one existing primary rather than creating three Seeds. Smart redirection/selected-Sow behavior remains separate. Real explosion impacts chain owned Seeds and apply only the native background DoT, leaving shared Wither counter policy and UA lifecycle registrations unchanged.

Native `f932eaa2c1` proves both failures with paid casts and normal frames: actual owner damage exceeded native threshold before expiry, and a separate Seed ended naturally, but neither produced27285. Probe `ffe97fd353` passes native threshold and expiry plus two literal265 and two UA controls, but proves two additional REDs: paid recast misses the original explosion deadline, and one no-Sow cast creates three actual Seeds rather than one. The phase policy and no-Sow filter address those failures and remain native qualification-gated.267 probe `c0e7affe53` passes six Hellcaller, three funding and two Wither controls. Parent assigns MID2 seed-applied UA to16783 on top; this leaf adds no UA caller. No numerical parity claim follows from the dispatch change.

## Sources

- [Counter lifecycle module](../../../crates/server-spell-scripts/src/modules/hellcaller_counters.rs) and [native lifecycle tests](../../../crates/server-simulation/src/dungeon_simulator/dps_parity/hellcaller_counter_tests.rs)
- [Downstream source policy](../../../crates/server-spell-scripts/src/modules/hellcaller_wither.rs)
- [Native cast and tick tests](../../../crates/server-simulation/src/dungeon_simulator/dps_parity/wither_stack_tests.rs)
- [Generic periodic scheduler](../../../crates/server-combat/src/auras/periodic.rs)
- [Native refresh handling](../../../crates/server-combat/src/auras/refresh.rs)

## See Also

- [[spell-script-crate]] — downstream registry and assembly.
- [[spell-casting]] — separate native Warlock funding boundary.
- [[auras]] — ordinary stack, refresh and periodic scheduling rules.
