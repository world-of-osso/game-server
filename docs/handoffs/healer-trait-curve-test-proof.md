# Native healer tests after trait-effect-point publication

Status: PARKED by lead priority pause. No builds, tests or extractions until lead RESUME. Owned agents-healer_ai.slice has populated0; no queued/running native job remained to cancel.

Scope: two reported master/current-canonical failures after the trait-point publish. Decide expectations versus production from TC/DB2; no changed amounts, tolerance widening, cancelled passives or canonical writes. Separate from five native-kit publication and frozen-dataset replay diagnosis.

## Proof status

| Case | Decision / exact scope | Proof |
|---|---|---|
| Wild Growth43 normalized falloff | Fixture read unranked SpellStore40 instead of applied talent amount. a3bd225dfe3fdd4b796251364c14c10905d40c93 uses owned aura effect0. Production unchanged. | Native GREEN: exact two-case run gives WG PASS/Holy FAIL,10.13s at32790/P2abc/privateDD1. Native factor0.7 confirms learned30. |
| Prismatic61 qualification | Real production rescaling precision loss BEFORE Echo: fully healthy Garrick18601/18601→new max29194 computes29193.998 then truncates29193. No damage receipts. | Same native run reproduces exact full-health guard. Diagnostic2061fac8f observes real inputs; d3d12ba20 adds an unrun direct regression. Production precision fix NOT implemented. |

Native proof artifact: `game-engine/data/diagnostics/healer-ai-2026-10-21/native-kits/trait-regressions/native-two.log`; immutable SQLite backup SHA256 dd1de231e32c26291182dff050a8de5a5132908723ed21454eb128e45e4b05ae. First attempt used incompatible protocol186134 and failed compilation before tests; archived separately. Corrected protocol2abc carries required group portrait metadata. No ready/all-pass claim. Lead can batch WG's actual test-only correction with Frenzy e19c33027; Holy requires a generic production fix.

## Wild Growth evidence

Current native DB2-derived rows:

- Spell382559 unranked effect0 base40.
- TraitDefinition108143 / TraitDefinitionEffectPoints19696: effect0, OperationType0(Set), Curve58935.
- Curve58935 Type0: rank1→30; rank2→60. Neither is bare40.

Pinned TC a352b1fa `SpellInfo.cpp:511–529`, `SpellEffectInfo::CalcValue`, reads the definition's effect-index override, evaluates CurveID at learned rank, replaces `value` for Set and multiplies it for Multiply. The local generic implementation is `spell_info/trait_effect_points.rs::TraitEffectPoints::apply`.

Production `auras/periodic.rs::unstoppable_growth_reduction` already reads the applied aura amount. The fixture must use that same actual source input while independently checking the existing SimC falloff policy. It retains the bare40 metadata assertion, native0.481 coefficient assertion and original ±0.5 receipt tolerance. No hardcoded30/60 expectation or new healing multiplier is introduced.

## Holy fixture boundary: not an Echo amount failure

The completed gate footer identifies the actual panic in `holy_policy_native_tests.rs::native_group_at`, before qualification: full-health assertion current29193/max29194. The owned native run confirms Garrick209057, zero damage receipts, and `rescaled_current` dropping a point while projecting fully healthy18601 to29194. Thus do not change Echo's formula or attribute handling. The earlier diagnostic55de82b9e was unreachable and is removed by2061fac8f.

TraitDefinition108749 points19824/22997 use Set curves59083/79270: effect0 rank1→8/rank2→15; effect1 rank1→20/rank2→40. Fixture and producer already read applied effect0. Actual projection inputs are now observed; damage is excluded. `stats.rs::rescaled_current` multiplies whole health values in f32, then truncates: the native product exceeds f32's exact-integer range. The pending generic correction is a wider intermediate, retaining existing ratio preservation, clamping and whole-unit truncation—not a special-case heal. It was NOT implemented before the priority pause. Direct regression d3d12ba20 uses the exact native18601→29194 inputs plus identity re-projection; it has NOT run. Preserve the unchanged native full-health assertion and existing partial-health tests. No health priming or tolerance changes.

Native77489's ignored-healing-modifier flags were a separate source discrepancy, not this failure. No such production change is included or authorized by this diagnosis. Wild Growth's source fix remains separate.
