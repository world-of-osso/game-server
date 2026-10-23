# Dungeon healing metrics

Standing healer benchmark: fixed recorded dungeon pressure, not a SimC HPS comparison. The adapter observes a real Stockade run, then replays one shared raw-hit timeline through native healer kits and recipient mitigation. Source inspection verified2026-10-22; native execution proof pending admission.

## Capture and replay

`healer_metrics::Recorder` installs an opt-in raw-hit capture resource in the transport-free App. Native melee records source-side damage before target taken multipliers/armor/absorbs and retains native hand/outcome/crit multiplier. Native spell damage records the source amount before target taken multipliers, armor, area avoidance and absorbs; effect index, school, periodic/crit and blocked flags remain explicit. Source GUID/formation identity, party slot,50ms tick, positions, source deaths and pull-end observations are retained. Combat-log amounts are never inverted to manufacture raw damage. Unknown raw source identity or already-mitigated direct damage becomes a blocking capture gap.

Replay disables autonomous enemy brains and retains native enemy identity/scaling/spells, party initialization and healer/companion decisions. Incoming hits run inside FixedUpdate before follower decisions. Spell replay enters the shared native resolver after source amount calculation; melee replay uses native recipient modifiers, armor, block, immunity, absorbs, transfers, health/death commit and swing publication/procs. Source health becomes zero on its recorded death tick without manufacturing new boss credit or expiring its source identity before queued hits finish. Recorded party positions are restored before each tick. This is a fixed-input healing benchmark, not a second route-clear qualification.

Native future scripted-damage alerts are not reconstructed. Native partial spell resistance is not implemented in the existing damage resolver; magical-input reports retain that named gap. A recorded blocked spell cannot silently use an unblocked replay path. No post-mitigation input fallback exists.

## Metrics

| Metric | Definition |
|---|---|
| Party deaths | Native Death receipts targeting an admitted member, including leader |
| Overheal % |100 × healer-attributed party Heal overflow / raw healing; null for no healing |
| Mana at end | Observed final replay-frame mana/current and maximum |
| Idle GCD equivalents | Alive combat-frame time with no cast/channel or active native GCD, divided by profile heal's native hasted GCD; frame-sampled readiness, not hypothetical cast opportunities |
| Effective HPS | Sum(raw heal − overheal) / observed party combat seconds; null for zero combat |

Absorbs affect survival and mana through native mitigation but are not mislabeled as effective healing. Native pull-end mana remains separately available in the ordinary simulator observations.

## Commands and artifacts

Run via capacity admission and `agent-run`, using targeted server tests only:

- `healer_benchmark_capture_stockade_and_report -- --ignored --nocapture`: run actual Stockade seed1, capture raw intake, then replay supported healers.
- `healer_benchmark_report_recorded_stockade -- --ignored --nocapture`: replay the committed intake without another autonomous dungeon run.
- `HEALER_DUNGEON_REPORT_DIR` selects output; default is simulator output directory plus `healer-benchmark/`. `HEALER_DUNGEON_INTAKE` overrides the persisted input path.
- Outputs: `stockade-seed1.intake.json`, `report.json`, `report.md`, and per-spec JSON. BLAKE3 binds every row to the exact same intake bytes.

Only105 Crenna209072 and257 Henry215011 have native follower profiles in the verified source DB.65,256,264,270,1468 remain `NATIVE_HEALER_PROFILE_ABSENT`, with null metrics. No new profile or player-healer policy is invented. Isolated native coefficient probes still cover all seven specs independently of dungeon actor availability.

## Sources

- [Feature contract](../../specs/healer-dungeon-metrics.md)
- [Coefficient ledger](../../reports/healer-coefficient-ledger.md) and [exact rows](../../reports/healer-coefficient-ledger.json)
- `crates/server/src/dungeon_simulator/{healer_intake,healer_metrics,healer_replay,healer_report}.rs`
- `crates/server/src/{combat.rs,spell_cast/effects.rs,followers/simulator.rs}`: narrow native capture/replay/actor-selection seams

## See also

- [Dungeon simulator](dungeon-simulator.md)
