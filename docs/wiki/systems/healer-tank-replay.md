# Healer and tank replay

The replay applies recorded incoming hits to native combat actors. The SimC dummy trace supplies tank pressure; an optional second trace supplies party-wide pulse damage. The healer policy, native spell effects, mitigation and resource state run in the server. This is not a same-input SimC healer oracle when SimC lacks the relevant healing policy.

## Reference and measurements

`scripts/healer_tank_replay.py` runs the replay and reports healer and tank metrics. The [replay specification](../../specs/healer-tank-replay.md) defines the traces, metrics and profiles. [Holy Paladin](../../specs/healer-tank-replay/65.md) and [Preservation Evoker](../../specs/healer-tank-replay/1468.md) contain their WCL references, native results and named gaps. WCL full-dungeon spell rates and overheal describe different damage intake and cannot establish numerical parity with the synthetic pulse.

The Preservation path resolves Lifebind transfers, Consume Flame healing and Life-Giver's Flame from native effects. The specialization specification records their current proof boundaries and the WCL/tooltip target-cap discrepancy.

## Sources

- [Healer and tank replay specification](../../specs/healer-tank-replay.md)
- `crates/server-simulation/src/dungeon_simulator/healer_simc_replay.rs`
- `crates/server-combat/src/auras/{lifebind,consume_flame,life_givers_flame}.rs`

## See also

- [Dungeon healing metrics](healer-dungeon-metrics.md) — recorded Stockade intake and native healer coefficient probes.
