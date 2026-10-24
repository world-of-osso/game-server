# Native healer tests after trait-effect-point publication

Scope: two reported master/current-canonical failures after the trait-point publish. Decide expectations versus production from TC/DB2; no changed amounts, tolerance widening, cancelled passives or canonical writes. Separate from five native-kit publication and frozen-dataset replay diagnosis.

## Proof status

| Case | Decision / exact scope | Proof |
|---|---|---|
| Wild Growth43 normalized falloff | Fixture read unranked SpellStore40 instead of applied talent amount. a3bd225dfe3fdd4b796251364c14c10905d40c93 uses owned aura effect0. Production unchanged. | Reported gate RED actual66 versus expected65.41296; DB2/TC/source confirmed; formatter passed. Native GREEN pending. |
| Prismatic61 Echo ticks | Not classified. Existing fixture ALREADY reads applied390967effect0; WG's fix cannot be copied blindly. | Reported isolated RED; detailed panic not yet available in growing gate-cs1046 log when inspected. Diagnostic-only55de82b9e leaves every assertion unchanged. |

Native heavy proof is serialized behind the already-owned cast-divergence probe. No ready/all-pass claim. Lead intends to batch actual test-only corrections with Frenzy e19c33027.

## Wild Growth evidence

Current native DB2-derived rows:

- Spell382559 unranked effect0 base40.
- TraitDefinition108143 / TraitDefinitionEffectPoints19696: effect0, OperationType0(Set), Curve58935.
- Curve58935 Type0: rank1→30; rank2→60. Neither is bare40.

Pinned TC a352b1fa `SpellInfo.cpp:511–529`, `SpellEffectInfo::CalcValue`, reads the definition's effect-index override, evaluates CurveID at learned rank, replaces `value` for Set and multiplies it for Multiply. The local generic implementation is `spell_info/trait_effect_points.rs::TraitEffectPoints::apply`.

Production `auras/periodic.rs::unstoppable_growth_reduction` already reads the applied aura amount. The fixture must use that same actual source input while independently checking the existing SimC falloff policy. It retains the bare40 metadata assertion, native0.481 coefficient assertion and original ±0.5 receipt tolerance. No hardcoded30/60 expectation or new healing multiplier is introduced.

## Echo boundary: do not guess

TraitDefinition108749 points19824/22997 use Set curves59083/79270: effect0 rank1→8/rank2→15; effect1 rank1→20/rank2→40. The existing native fixture and producer both read applied effect0. Added diagnostic prints native mastery before/after direct healing, actual direct amount, owned Echo effect snapshots and caster healing-done multiplier, separating producer-pool drift from tick execution.

A separate source discrepancy exists: native77489 Attr6=0x08001000 includes IGNORE_HEALING_MODIFIERS; TC `Unit.cpp:7458–7473` returns1 from SpellHealingPctDone for ignore flags, while current apply_heal always multiplies caster done factors. This is NOT accepted as this test's cause and no production change is included. Wild Growth lacks that ignore flag; Regenesis383191's two354 masks do not match48438. Neither hypothesis substitutes for the actual native failure boundary.
