# Model geometry

`MISSING_MODEL_INFO_TC_PLAYER_DEFAULTS_SCALED` is an explicit project policy for actual displays lacking model-info. It is not a claim that TrinityCore admits missing-model creatures. [Requirements](../../specs/model-geometry.md).

## Source precedence and scale

Authored `encounter_creature_model_info.CombatReach` wins. Missing entries use TC's player combat reach1.5; the existing toy-facing consumer uses its player radius0.389 only when its authored entry is absent. Ordinary cone boundary admission is source-bound and does not use this missing-model default. Both use the existing effective `UnitScale`, which already includes creature and display scale. Player and sourced pet semantics remain unchanged. Missing display identity and uninitialized catalogs remain unavailable; defaults do not invent a source row.

World reach-adjusted distance and ordinary proximity aggro share `model_reach::creature_combat_reach`. Reach and toy-facing bounding radius share the pure `scaled_model_dimension` decision; each consumer retains its authored catalog. The former follower-only sizing rule is replaced, not layered underneath this policy. No canonical data changes are required.

## Ordinary cone close boundary

`ModelReaches` retains `BoundingRadius` beside the unchanged `CombatReach` from all98609 prepared model-info rows. `model_reach::bounding_radius` reads the target's actual display and effective scale; players use TC's native radius. Missing creature rows, display identity or the catalog remain unavailable, not guessed geometry.

Shared ordinary cone selection bypasses only the arc test when exact3D center distance is strictly less than `max(target radius, 2yd)`. TC `Unit::IsWithinBoundaryRadius` uses the **target** radius, not the caster's radius and not reach-adjusted distance. Area radius, map/life/faction/conditions/caps remain unchanged; line selectors do not bypass their width/front checks.

Native RED at607781b26: actual FoF117418 misses the1.803yd/33.69° enemy and Mage120 misses the close rear enemy; strict2yd/far/vertical negatives already pass (2FAIL/1PASS,8.31s runtime). The harness loads real model-info and the Kobold6's actual display10913/scale1 (radius0.550618, combat reach0). GREEN and retained-cone qualification pending; no new WW policy or numerical parity claim. Evidence: `game-engine/data/diagnostics/ww-parity-2026-10-23/cone-boundary-*`.

## Proof

Native Shadowguard Champion122403/display124440 has no model-info row. At f626188b8, real nearby-player acquisition and scaled default/source-precedence tests both fail (5.15s). Reach then passes2/2 at c08b8a2ea; the new scaled radius/boundary case genuinely fails on the missing native radius (4.03s composite:2PASS/1FAIL). All three GREEN at a25c38d2e (5.56s). Merged bbc4c9e1e (master655258480) passes20/20 retained cases in54.26s: the three new cases, six attack-distance controls, four Gold-follow movement/cast/root controls, four actual toy-facing controls and three external-data controls. Format check passes; scoped check and Bo simulator requalification remain pending.

## Sources

- TC a352b1fa `ObjectDefines.h:39–40` — player radius0.388999998569489f/reach1.5f constants.
- TC a352b1fa `Creature.cpp:3591–3610` — model geometry uses objectScale×displayScale.
- `crates/server/src/zones/entry.rs` and `crates/server/src/auras/unit_state.rs` — existing effective `UnitScale`.
- `crates/server/src/scripted_ai/model_reach.rs`, `crates/server/src/creature_aggro_distance.rs` — production reach consumers.
- `crates/server/src/spell_cast/toy_facing.rs` — existing bounding-radius cast boundary.
- TC a352b1fa `Spell.cpp:9462–9483`, `Unit.cpp:712–720`, `Position.h:155–157`, `ObjectDefines.h:43` — ordinary cone exception, target radius/minimum2 and strict3D distance.
- `crates/server/src/spell_cast/targets.rs` — shared cone boundary admission.

## See Also

- [Follower utility](follower-utility.md) — follower pursuit uses the same model reach.
