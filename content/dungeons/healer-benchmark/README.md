# Recorded Stockade healer intake

`stockade-seed1.intake.json` is an actual transport-free native dungeon recording, not synthesized damage or inverted combat logs. Capture source: implementation `07b2a25096cceacf7569cf1b116ace539070487c`, native test executable preserved from its targeted build, executed2026-10-23 under capacity admission and `agent-run healer-metrics`.

| Property | Observed value |
|---|---|
| Dungeon | Stockade map34, Normal difficulty1, level32 |
| Seed / result |1 / `cleared` |
| Timing |43,797 ×50ms ticks;1,594.3 recorded party-combat seconds |
| Receipts |1,132 raw incoming attempts;60 native pull-end receipts |
| Native damage-to-HP scale |1,132/1,132 unit;0 non-unit |
| Recorded spell blocks |41:86727×34,86604×4,86729×3 |
| BLAKE3 of exact file bytes |`541a4dedf0e1114307fd9972cd21642ac5ad62f6646b7a62f61c677839229eb8` |

Capture command filter: `healer_benchmark_capture_stockade_and_report -- --ignored --nocapture --test-threads=1`, through the native build helper/`agent-run` after `CAPACITY_ID=healer-metrics` admission. The capture runs `campaign::run(1)` with the unchanged native Garrick/Crenna/Shuja/Meredy/Austin party and route. Observation hooks record source-side amounts before recipient mitigation, stable source GUID/party slot/tick, hit/crit/block/depth flags, source native-level/tuning snapshots, party positions and source deaths. Source modifiers/variance are already represented in raw amounts; target armor, damage reduction and absorbed points are not baked in.

All recorded level factors are unit, so `ABSORB_POINTS_VS_PREVENTED_HP_LEVEL_SCALING` is moot for this intake. Native replay still verifies its own scale and consumed-point attribution. No normalized prevented-HP observer exists.

Replay this exact file with `healer_benchmark_report_recorded_stockade -- --ignored --nocapture`; do not rerun the autonomous route when fixing the replay adapter. Source moving geometry, future damage alerts and native partial spell resistance remain named coverage gaps; see [benchmark implementation](../../../docs/wiki/systems/healer-dungeon-metrics.md). Initial replay exposed a real unsupported recorded-block flag; the data is retained unchanged while that seam is corrected.
