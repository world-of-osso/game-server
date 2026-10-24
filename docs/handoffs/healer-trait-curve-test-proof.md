# Native healer tests after trait-effect-point publication

Scope: two reported master/current-canonical failures after the trait-point publish. Decide expectations versus production from TC/DB2; no changed amounts, tolerance widening, cancelled passives or canonical writes. Separate from five native-kit publication and frozen-dataset replay diagnosis.

## Proof status

| Case | Decision / exact scope | Proof |
|---|---|---|
| Wild Growth43 normalized falloff | Fixture read unranked SpellStore40 instead of applied talent amount. a3bd225dfe3fdd4b796251364c14c10905d40c93 uses owned aura effect0. Production unchanged. | Reported gate RED actual66 versus expected65.41296; DB2/TC/source confirmed; formatter passed. Native GREEN pending. |
| Prismatic61 qualification | Fixture fails BEFORE Echo execution at native_group_at:83: current29193 versus max29194. No Echo amount failure is established. | Exact cs1046 footer read at222128–222135. Diagnostic2061fac8f identifies native member/damage receipts and unexpected full-health rescaling; assertions remain unchanged. |

Native heavy proof is serialized behind the already-owned cast-divergence probe. No ready/all-pass claim. Lead intends to batch actual test-only corrections with Frenzy e19c33027.

## Wild Growth evidence

Current native DB2-derived rows:

- Spell382559 unranked effect0 base40.
- TraitDefinition108143 / TraitDefinitionEffectPoints19696: effect0, OperationType0(Set), Curve58935.
- Curve58935 Type0: rank1→30; rank2→60. Neither is bare40.

Pinned TC a352b1fa `SpellInfo.cpp:511–529`, `SpellEffectInfo::CalcValue`, reads the definition's effect-index override, evaluates CurveID at learned rank, replaces `value` for Set and multiplies it for Multiply. The local generic implementation is `spell_info/trait_effect_points.rs::TraitEffectPoints::apply`.

Production `auras/periodic.rs::unstoppable_growth_reduction` already reads the applied aura amount. The fixture must use that same actual source input while independently checking the existing SimC falloff policy. It retains the bare40 metadata assertion, native0.481 coefficient assertion and original ±0.5 receipt tolerance. No hardcoded30/60 expectation or new healing multiplier is introduced.

## Holy fixture boundary: not an Echo amount failure

The completed gate footer identifies the actual panic in `holy_policy_native_tests.rs::native_group_at`, before qualification: full-health assertion current29193/max29194. Thus do not change Echo's formula or attribute handling to fix this failure. The earlier diagnostic55de82b9e was unreachable and is removed by2061fac8f.

TraitDefinition108749 points19824/22997 use Set curves59083/79270: effect0 rank1→8/rank2→15; effect1 rank1→20/rank2→40. Fixture and producer already read applied effect0. Native cause investigation now separates genuine preparation damage from health projection: unchanged full-health assertion reports member/template and real damage receipts; test-only rescaler logging records full→nonfull truncation inputs if it occurs. No health priming or tolerance changes. A simulated same-max29194→29194 rescale does NOT lose a point, so that assumed input sequence is rejected; actual input ordering must be observed.

Native77489's ignored-healing-modifier flags were a separate source discrepancy, not this failure. No such production change is included or authorized by this diagnosis. Wild Growth's source fix remains separate.
