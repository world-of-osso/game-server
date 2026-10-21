# Restoration depth qualification

verified: 2026-10-21

## Source and frozen comparison

Baseline: master655258480 + passive observer181b44bb09d70ec92bc12b49f981336aea2d4214. Candidate:2d4dc7856 (before subsequent master integration). Both frozen protocol03cebdb971894aa1fa69b2e4ba7e7fae2702d515, server simopt1/deps3, Stockade32, seeds1–20/jobs2. Not legacy AI. Native RNG is not fully seeded.

Evidence root: `game-engine/data/diagnostics/healer-ai-2026-10-21/`. `paired-input-manifest.json` pins SQLite/graph/source/observer; `paired-stockade-ground-hashes.json` pins40geometry/LOS files. DB SHA2566b8b5e96b2a1f0caac05df74abce559516cacae9279d85e669d97edfd0dd7f2c and all40assets unchanged after both batches. Observation callbacks identical; only two native Tree lifecycle hooks differ in their parent aura file.

| Metric | Framework | Restoration depth |
|---|---:|---:|
| Clear / timeout / stalled |11 /8 /1 |10 /9 /1 |
| Party deaths / wipes |1 /0 |1 /0 |
| Native healer overheal |50.875% |51.673% |
| Tank Lifebloom / Rejuvenation / Regrowth |78.52 /68.23 /80.78% |95.76 /74.85 /81.47% |
| Any tank HoT |90.73% |95.82% |
| Clearcasting consumed / gained charges |2153 /2429 |2676 /2949 |
| Soul consumed / gained charges |2947 /2963 |2903 /2917 |
| Median /10th-percentile pack-end mana |87.84 /58.91% |89.35 /64.20% |
| Mean per-run minimum mana |2.15% |4.55% |
| Alive-tank combat seconds |52201.8 |53412.95 |

All40reports present; EXIT101 on both runners records uncleared scenarios, not omitted/crashed reports. Different reached routes/durations remain in denominators. No clear-rate, mortality or overheal improvement claim. The observed coverage/mana changes are descriptive; native thread_rng and whole-party route failures prevent causal attribution from20scenario seeds.

Raw outputs: `baseline-framework-655258480/`, `candidate-resto-depth/`; aggregate `paired-healer-metrics.json`;40run rows `paired-runs.csv`;283223mana samples `paired-mana-curve.csv`;3040native pack ends `paired-pull-end-mana.csv`. High-level spells are qualified through native pressure tests, not granted in this level32 benchmark.

## Native proof

[System page](../wiki/systems/follower-healer-triage.md#proof-ledger) records exact RED/GREEN scopes. The final master-integrated source is separately gated; its newer unrelated rotation/targeting changes are not silently substituted into the frozen comparison.

Remaining exclusions: Convoke absent from kit; SotF Wild Growth bonus unqualified; Rebirth decision/execution utility-owned; NPC treat-as-raid Efflo priority subcase unqualified. Production navigation/pixels are not inferred from frozen-plane simulation.

Final code gate06cd80b2b/masterc6bc60006:39targetedPASS/0FAIL/0ignored58.74s, compile5m15s; scoped sim checkEXIT0/45.12s with sole pre-existing lead-excluded13179 `has_uncleared_fight` warning; cargo fmt --all --check PASS; changed Rust manual readability audit (CLI unavailable), no suppression. Tests include actual Efflo cap3/player priority and full native field/lifecycle chain. Subsequent changes are documentation only.
