# Native trait effect points

## Active goal

Import native TraitDefinitionEffectPoints through the normal DB2 importer; retain definition and purchased+granted rank for applied trait spells; apply TCa352b1fa CalcValue Set/Multiply curve operations before variance and point modifiers. Private DB only. Parent owns canonical publication and integration gate. No spell-name lists, rank-times-amount heuristic, invented curves, unrelated label changes or Ret DPS claims.

## Source and coverage

- TC `Player.cpp29409–29424`: applied trait learns definition spell with definition ID and total purchased+granted ranks. `SpellInfo.cpp532–553`: matching effect index evaluates the curve at rank, Set replaces and Multiply multiplies the value. `DB2Stores.cpp2296–2458` owns interpolation.
- Native70448 TraitDefinitionEffectPoints:1408 rows, Set909/Multiply492/None7.1358 referenced curves/4340 points. Current referenced types:1371 rows linear Type0;37 rows Type2 (point-count-dependent cosine/Bezier). Other types are not present in this source census.
- Normal `scripts/db2.py` already imports all Curve/CurvePoint rows. All1358/4340 referenced frozen DB rows were compared against exact native CSV bytes/numeric fields; no missing curves. New registry/schema entry still pending.
- Private DB `game-engine/data/diagnostics/trait-points-2026-10-23/world.trait-points.db` adds all1408 point rows to the frozen552458c2 source, canonical untouched. `private-source.proof.json` records database/CSV hashes and equality.

## Implementation slices

1. Core implementationdb9e2b3d6: fresh rank2 Multiply, fresh rank2 Set, rank1 controls and actual82523 Bezier GREEN; unchanged same-book lifecycle genuinely RED. `trait-points-core-green-lifetime-red-v1.{log,proof.json}`:4PASS/1expectedFAIL32.71s,13m04compile, Rust warning-free, stable source scope. Scoped spell fixtures retain native point/curve data; variance retains its original-base contribution.
2. Existing-aura rank replacement now has genuine RED: same learned IDs, amount5/-3/10 remains after1→2 instead of10/-6/20. Aura lifecycle implementation next.
3. Normal importer1PASS1.482s atdb9 (`trait-points-normal-import-v1`). Its pre-existing `check_csvs` in-memory schema connection emitted ResourceWarning; close it explicitly and make the same integration test assert no leaked SQLite resources. Rerun required only for that changed importer scope.

Publication artifact `trait-points.delta.sql`/`.proof.json` replaces1408 point rows plus1358 referenced curves/4340 points and their build receipts, preserving all other curve IDs/tables. Generated from native CSVs, not applied canonically.

`trait-points-red-v3` at8da8a9c17 was SHED-stopped during compilation (RAM5.7GiB), exit-15; not behavior evidence. No tests were changed to bypass native eligibility.

## Native behavioral tests

Ignored tests were pushed before production changes. Atbc5ce36ca, `trait-points-red-v2` observed both genuine rank2 amount failures below. Rank1/lifecycle instead hit native condition20153: lowering both ranks leaves21 earlier class points, below23. Corrected controls omit native final class-group8482; the lifecycle keeps that same early-class spell book at both ranks. Runtime remains unchanged; control/lifecycle rerun pending.

| Case | Native source | Expected observable | Proof |
|---|---|---|---|
| Rank2 Multiply | Ret selected node93009/entry115034, definition120041, spell402964; rows22216–22218; curves64686/64685/64684; points180714–180719 | Actual learned passive amounts10/-6/20 from raw5/-3/10 × curveY2 | GREEN atdb9 |
| Rank2 Set | Ret selected node81630/entry102622, definition107627, spell385427; rows24203–24205; curves94680/94679/94678; points284380–284385 | Actual learned passive amounts4/-4/4, replacing raw2/-2/2 | GREEN atdb9 |
| Rank1 controls | Native validated reference-derived purchases, two rank1 allocations and no final class-group8482 | Actual passive amounts5/-3/10 and2/-2/2 | GREEN atdb9 |
| Existing passive rank change | Same player and learned spell IDs, native validated1→2→1 trait configurations | Actual aura amounts follow each rank, not stale first application | genuine RED atdb9 |

Reference purchases are the pinned76f8118 spec70 profile fixture. Rank2 witnesses retain the exact selected reference allocation; rank1/lifecycle controls omit final class-group8482, whose native condition20153 requires23 earlier class points. All configurations use ordinary native validation; no injected aura, effect amount, spell coefficient or resource.
