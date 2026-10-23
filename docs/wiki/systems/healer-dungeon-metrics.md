# Dungeon healing metrics

Standing healer benchmark: fixed recorded dungeon pressure, not a SimC HPS comparison. The adapter observes a real Stockade run, then replays one shared raw-hit timeline through native healer kits and recipient mitigation. Source inspection verified2026-10-22; native recording and targeted controls executed2026-10-23. [Frozen recording/provenance](../../../content/dungeons/healer-benchmark/README.md) is retained unchanged. [Native results and proof](../../reports/healer-stockade-seed1/README.md):14 targeted controls pass at rebased dce76186c, including full1132-hit replay for105/257 and15 direct-heal probes; five missing-profile gaps remain. Initial full-replay spell-block and second native swing-clock failures are fixed without approximating input.

## Capture and replay

`healer_metrics::Recorder` installs an opt-in raw-hit capture resource in the transport-free App. Native melee records source-side damage before target taken multipliers/armor/absorbs and retains native hand/outcome/crit multiplier. Native spell damage records the source amount before target taken multipliers, armor, area avoidance and absorbs; effect index, school, periodic/crit, trigger depth and blocked flags remain explicit. Trigger depth preserves native proc eligibility; damage-shield owner and bonus aura caster retain separate sourced identities (an unavailable bonus caster blocks qualification). Source GUID/formation identity, native level/tuning snapshot, party slot,50ms tick, party positions, source deaths and pull-end observations are retained. Native `zones.rs::roll_spawn_scaling` rolls template delta per spawn; Normal Stockade route entries46248/46249/46250/46251/46252/46261/46263 source delta0..1. Replay restores captured deltas after validating entry/native level/tuning range, preventing healer-dependent RNG prefixes from changing attacker armor/health-conversion inputs. Catalog drift remains a named gap. Combat-log amounts are never inverted to manufacture raw damage. Unknown raw source identity or already-mitigated direct damage becomes a blocking capture gap.

Replay disables autonomous enemy brains and retains native enemy identity/scaling/spells, party initialization and healer/companion decisions. Source `CombatTarget` is removed and never projected from recorded hits: native `creature_attack_system` starts auto-attacks from that component independently of `CreatureState`, which the full-recording regression exposed at tick930. Combat refs still admit native companion decisions; the raw-receipt guard rejects any second incoming hit clock immediately. Incoming hits run inside FixedUpdate before follower decisions. Spell replay enters the shared native resolver after source amount calculation; melee replay uses native recipient modifiers, armor, block, immunity, absorbs, transfers, health/death commit and swing publication/procs. Source health becomes zero on its recorded death tick without manufacturing new boss credit or expiring its source identity before queued hits finish. Recorded party positions are restored before each tick. This is a fixed-input healing benchmark, not a second route-clear qualification.

Native future scripted-damage alerts and moving enemy-source geometry are not reconstructed (both remain named coverage gaps); recorded party positions are restored. Enemy sources retain native spawn positions. Native partial spell resistance is not implemented in the existing damage resolver; magical-input reports retain that named gap. Recorded physical spell-block flags enter the same native `spell_block_damage` percent/critical-block handling after armor; no rerolled hit table or inferred post-mitigation amount is substituted. The first full intake exposed41 actual blocks and now supplies the regression fixture. No post-mitigation input fallback exists.

## Metrics

| Metric | Definition |
|---|---|
| Party deaths | Native Death receipts targeting an admitted member, including leader |
| Overheal % |100 × healer-attributed party Heal overflow / raw healing; null for no healing |
| Mana at end | Observed final replay-frame mana/current and maximum |
| Idle GCD equivalents | Alive combat-frame time with no cast/channel or active native GCD, divided by profile heal's native hasted GCD; frame-sampled readiness, not hypothetical cast opportunities |
| Effective HPS | Sum(raw heal − overheal) / frozen intake recorded combat seconds; null for zero combat; native observed combat seconds remain separate |
| Consumed healer absorbs | Native SCHOOL_ABSORB points actually consumed on party before level scaling, attributed to healer caster before aura removal; native damage-meter convention, not normalized prevented HP |
| Combined throughput | (Effective healing + consumed healer absorbs) / the same recorded combat window, including early-wipe comparisons |

Every raw hit records `level_scaling_factor = native damage multiplier / victim health multiplier`, before integer rounding. JSON/MD report total/unit/non-unit counts and each native replay's share of healer-consumed points on non-unit hits. `ABSORB_POINTS_VS_PREVENTED_HP_LEVEL_SCALING` is retained for non-unit cases; all-unit captures explicitly mark it moot with the count. No counterfactual prevented-HP observer or normalization is implemented.

The native absorb consumer already retains caster/consumed-amount contributions. An observation-only hook records those before aura removal, including follower casters, tagged by the active recorded/replayed hit's native level factor; applied/unconsumed capacity and incoming combat-log `absorbed` totals are not added. Absorbs remain distinct from Heal HPS and also contribute to the combined throughput column. Native pull-end mana remains separately available in the ordinary simulator observations.

## Commands and artifacts

Run via capacity admission and `agent-run`, using targeted server tests only:

- `healer_benchmark_capture_stockade_and_report -- --ignored --nocapture`: run actual Stockade seed1, capture raw intake, then replay supported healers.
- `healer_benchmark_report_recorded_stockade -- --ignored --nocapture`: replay the committed intake without another autonomous dungeon run.
- `HEALER_DUNGEON_REPORT_DIR` selects output; default is simulator output directory plus `healer-benchmark/`. `HEALER_DUNGEON_INTAKE` overrides the persisted input path.
- Outputs: `stockade-seed1.intake.json`, `report.json`, `report.md`, and standalone per-spec JSON plus Markdown (including blocked specs). BLAKE3 binds every row to the exact same intake bytes.

Only105 Crenna209072 and257 Henry215011 have authored healer follower profiles in the verified source DB. The approved benchmark-only `HEALER_BENCHMARK_ALTERNATE_SPEC_BRIDGE` reuses authenticated same-class actors for65/256/264/270/1468 without rewriting source identity/display or production roster; [source candidates](../../reports/healer-alternate-spec-source-candidates.md). A configured actor is not a decision policy: absent available kit rows return `NATIVE_HEALER_POLICY_ABSENT` before spawning, and a missing policy-owned primary heal reference returns `NATIVE_HEALER_PRIMARY_HEAL_ABSENT`. JSON/Markdown retain source and requested specs even on blocked rows. The bridge uses current master's unchanged book initialization.14752's separate future native-book helper is not a bridge prerequisite; no parallel evaluator or guessed kit is added. Isolated native coefficient probes still cover all seven specs independently of dungeon actor availability. The determinism fixture derives an80%-of-native-HP pre-armor injury from actual source/target scaling, runs6s and requires positive native healing; identical zero-healing traces do not qualify.

## Private policy input boundary (prepared RED)

Existing follower fixtures consume `FOLLOWER_KIT_DB`, but the benchmark's runtime currently loads the canonical catalog and ignores that explicit selection. Approved follow-up loads the private FollowerCatalog only into the benchmark App, with current native class/book initialization unchanged; no generic runtime or canonical/production mutation.

Two process-isolated ignored controls cover a nonexistent explicit catalog (no canonical native-metric substitution or empty-file creation) and a private copy of authentic catalog tables missing source209057 (no canonical actor replacement). The latter deliberately tests source refusal, not a guessed policy. Test subprocesses inherit the admitted agent slice; native report output and source errors are observed rather than mutating process-global environment inside a shared test executable. Implementation/RED execution remain pending; [contract](../../specs/healer-dungeon-metrics.md#explicit-private-policy-catalog).

## Sources

- [Feature contract](../../specs/healer-dungeon-metrics.md)
- [Coefficient ledger](../../reports/healer-coefficient-ledger.md) and [exact rows](../../reports/healer-coefficient-ledger.json)
- `crates/server/src/dungeon_simulator/{healer_intake,healer_metrics,healer_replay,healer_report}.rs`
- `crates/server/src/{combat.rs,spell_cast/effects.rs,followers/simulator.rs}`: narrow native capture/replay/actor-selection seams

## See also

- [Dungeon simulator](dungeon-simulator.md)
