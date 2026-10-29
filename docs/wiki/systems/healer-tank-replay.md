# Healer and tank replay

The replay feeds recorded incoming hits into native combat to measure healer and tank behavior under fixed pressure. The optional party trace adds five-slot synthetic Shadow pulses to the tank-dummy trace. This is a synthetic benchmark, not a recorded dungeon encounter.

## Reference limits

The pinned SimC `76f8118` Priest module explicitly marks healing unsupported, but contains experimental Discipline and Holy action lists and partial healing implementations. The supplied top-character default APLs fail in SimC. Bounded custom policies can run with SimC healing dummies, but those dummies and the SimC tank attacks do not reproduce the native replay's incoming trace. WCL dungeon logs supply real spell-mix and event-sampled mana references; different incoming damage and downtime prevent direct parity claims. See the specialization specs for exact inputs, results and open gaps.

Native replay healing metrics count healer and owned-summon heal events, separately recording raw healing, effective healing and overheal. Consumed shield points come from the native per-caster, per-spell absorb consumer, not the victim's aggregate absorb counter. Shield points are not included in healing HPS.

## Sources

- [Replay contract](../../specs/healer-tank-replay.md)
- [Discipline Priest reference](../../specs/healer-tank-replay/256.md)
- [Holy Priest reference](../../specs/healer-tank-replay/257.md)
- `crates/server-simulation/src/dungeon_simulator/healer_simc_replay.rs` and `healer_simc_metrics.rs`
- `crates/server-combat/src/test_support/healer_intake.rs`

## See also

- [Dungeon healing metrics](healer-dungeon-metrics.md)
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
