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
