# Trait Effect Points

Native talent effect amounts follow installed DB2 TraitDefinitionEffectPoints and TCa352b1fa learned-trait rank semantics. [Runtime/data model](../wiki/systems/talents.md#trait-effect-points).

## What it must do

- [ ] Import native TraitDefinitionEffectPoints and every required Curve/CurvePoint through the normal DB2 importer, preserving signed operation values and source build receipts.
- [ ] Applied trait spells retain their definition ID and purchased-plus-granted rank; inactive hero selections and unrelated triggered spells do not acquire that identity.
- [ ] CalcValue evaluates the matching effect's curve at the learned rank: Set replaces the value, Multiply multiplies it, None preserves it. Apply before variance/point modifiers; variance retains the original base contribution.
- [ ] Native rank2 and rank1 learned passives expose source-correct Set and Multiply amounts. Rank-only changes update existing passive amounts even when learned spell IDs do not change.
- [ ] Current70448 referenced curves use exact TC linear/Type2 interpolation. Missing or unsupported referenced data fails explicitly; no invented curve or rank-times-amount substitute.

## How it works

- [Trait runtime and source coverage](../wiki/systems/talents.md#trait-effect-points).
- [Source rows and native proof ledger](../handoffs/trait-effect-points-proof-ledger.md).

## Implementation inventory

- `scripts/db2.py`, `scripts/db2_schema.sql`: normal native import/schema.
- `crates/server/src/trait_config.rs`, `class_progression.rs`: active learned definition/rank.
- `crates/server/src/spell_info/trait_effect_points.rs`, `spell_info/load.rs`: modifier/curve loading and evaluation.
- `crates/server/src/spell_cast/effects.rs`: CalcValue integration; `spell_cast/test_harness.rs` retains catalog metadata in scoped native fixtures.

## Tests asserting this spec

- `crates/server/src/spell_cast/trait_effect_points_tests.rs`: native Set/Multiply rank2, rank1 controls and same-book rank changes.
- `crates/server/src/spell_info/trait_effect_points.rs`: actual82523 Bezier values.
- `scripts/tests/test_db2_trait_effect_points.py`: normal importer output and curve relationships.

## Known gaps (current cycle)

- [ ] Fresh amount implementation committed after two genuine native rank2 REDs; GREEN and rank1 control pending.
- [ ] Existing-passive rank relearning remains a separate implementation/proof step.
- [ ] Parent-owned canonical data publication and cross-spec integration gate.

## Out of scope

New trait allocation rules, client tooltip presentation, unrelated labels, reference-derived coefficients and unsupported curve types absent from the current referenced source corpus.
