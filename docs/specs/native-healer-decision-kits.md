# Minimal native alternate healer decisions

Named project decision kits for the five approved alternate healer configurations live in `crates/server/src/followers/` and `scripts/follower_spec_kits.py`. Native amounts and admission remain the ordinary spell engine's responsibility. [Shared triage](../wiki/systems/follower-healer-triage.md), [benchmark bridge](../wiki/systems/healer-dungeon-metrics.md).

## What it must do

- [ ] Support65 on source Garrick209057,256 on Henry215011,264 on Shuja209053,270 on Bo227734 and1468 on Kybel215240 without changing authored source identities or the production roster.
- [ ] Publish standard eight-column kit rows for native-book routine, emergency and available HoT actions; select a native primary heal/GCD reference through the policy-owned bridge accessor. Missing native-book capabilities remain named gaps, never grants.
- [ ] Admit only native learned actions at the actual level; use ordinary costs, cooldowns, auras, targets, healing and resource generation. Cite DB2/TC for amounts; thresholds and priorities are explicitly project policy.
- [ ] Prove each spec with a native RED before implementation, then an ordinary primary cast plus effective native patient healing, native GCD normalization and representative emergency/HoT controls.
- [ ] Measure each kit through the unchanged recorded-intake benchmark against a frozen master control: deaths, overheal, idle GCD equivalents and effective HPS. Reject apparent percentage improvements that suppress useful healing. Isolate baseline/published-snapshot differences before claiming comparable improvements.

## How it works

- [Alternate actor source candidates](../reports/healer-alternate-spec-source-candidates.md)
- [Native coefficient ledger](../reports/healer-coefficient-ledger.md)
- [Shared follower triage](../wiki/systems/follower-healer-triage.md)
- [Recorded-pressure benchmark](../wiki/systems/healer-dungeon-metrics.md)

## Implementation inventory

- `crates/server/src/followers/native_healer_kit_tests.rs` — per-spec native admission, primary/GCD and real-healing contract.
- `crates/server/src/followers/simulator/healer_profiles.rs` — independently owned approved alternate actor bridge; policy consumes its contract.
- `crates/server/src/followers/kit_source_tests.rs` — existing starter-build validator source export, extended to approved alternate configurations with explicit source-spec/bridge metadata.
- `scripts/follower_spec_kits.py` — existing native-book publication format; no canonical writes from this work.

## Tests asserting this spec

- `crates/server/src/followers/native_healer_kit_tests.rs` — five explicit spec cases; not qualified until native GREEN.
- `crates/server/src/followers/healer_benchmark_profile_tests.rs` — existing source-identity and missing-policy admission controls.

## Known gaps (current cycle)

- [ ] All five decision kits and their native primary/GCD references are absent at the initial249358b31 baseline.
- [ ] Exact bridge accessor signature is awaiting owner confirmation through the lead.
- [ ] Baseline/published-snapshot divergence is under causal diagnosis; frozen class/spell/trait inputs match but other party kit rows differ.

## Out of scope

- Production roster/identity rewrites, synthetic spell/talent/resource/stat grants and canonical DB mutation.
- Benchmark/capture changes, generic-map simulator work and a second native-book evaluator.
- Complete healer APL, Retail parity or class-wide mechanic closure; this slice is deliberately minimal and records unsupported native cases.
