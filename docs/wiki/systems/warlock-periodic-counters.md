# Warlock periodic counters

Hellcaller Wither uses stacks as a counter, not as an ordinary periodic-damage multiplier. Both Affliction265 and Destruction267 share the [counter contract](../../specs/dps-parity.md#shared-hellcaller-wither-counters).

## Representation

The downstream `hellcaller_wither` module registers native445474 through `CounterOnlyDotStacks`. Native aura stacks, maximum, duration and tick phase remain intact. The generic periodic scheduler uses one payload regardless of that counter. Ordinary refresh contributes no new counter stack; an explicit native `SPELLVALUE_AURA_STACK` application still uses normal stack merging.

This is separate from [Destruction shard funding](spell-casting.md#warlock-periodic-shard-funding--2026-10-25), which is specialization-qualified, and from Affliction's Agony threshold RNG. Neither resource policy is a Wither damage multiplier.

## Cause and proof

Pinned SimC76f8118 `sc_warlock_actions.cpp1066` sets `dot_ignore_stack=true`;1174–1186 cancels the automatic refreshed-dot stack increment while preserving earned stacks. Native generic stacking previously did both: a real recast increased1→2, and ordinary noncritical tick damage increased6398→63979 when the native counter input changed1→10.

`wither_stack_tests.rs` reproduces both failures and passes after the source policy on both265 and267. It observes actual ordinary casts and periodic damage, including preservation of an existing eight-stack counter. Destruction funding, Affliction native mechanics/rotation and owned DoT-duration controls also pass. This proves the counter boundary, not Blackened Soul producers, collapse behavior or numerical DPS parity.

## Sources

- [Downstream source policy](../../../crates/server-spell-scripts/src/modules/hellcaller_wither.rs)
- [Native cast and tick tests](../../../crates/server-simulation/src/dungeon_simulator/dps_parity/wither_stack_tests.rs)
- [Generic periodic scheduler](../../../crates/server-combat/src/auras/periodic.rs)
- [Native refresh handling](../../../crates/server-combat/src/auras/refresh.rs)

## See Also

- [[spell-script-crate]] — downstream registry and assembly.
- [[spell-casting]] — separate native Warlock funding boundary.
- [[auras]] — ordinary stack, refresh and periodic scheduling rules.
