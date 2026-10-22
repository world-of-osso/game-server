# Model geometry

`MISSING_MODEL_INFO_TC_PLAYER_DEFAULTS_SCALED` is an explicit project policy for actual displays lacking model-info. It is not a claim that TrinityCore admits missing-model creatures. [Requirements](../../specs/model-geometry.md).

## Source precedence and scale

Authored `encounter_creature_model_info.CombatReach` wins. Missing entries use TC's player combat reach1.5; bounding-radius consumers use its player radius0.389 only when their authored entry is absent. Both use the existing effective `UnitScale`, which already includes creature and display scale. Player and sourced pet semantics remain unchanged. Missing display identity and uninitialized catalogs remain unavailable; defaults do not invent a source row.

World reach-adjusted distance and ordinary proximity aggro share `model_reach::creature_combat_reach`. Reach and toy-facing bounding radius share the pure `scaled_model_dimension` decision; each consumer retains its authored catalog. The former follower-only sizing rule is replaced, not layered underneath this policy. No canonical data changes are required.

## Proof

Native Shadowguard Champion122403/display124440 has no model-info row. At f626188b8, real nearby-player acquisition and scaled default/source-precedence tests both fail (5.15s). Reach then passes2/2 at c08b8a2ea; the new scaled radius/boundary case genuinely fails on the missing native radius (4.03s composite:2PASS/1FAIL). All three GREEN at a25c38d2e (5.56s). Merged bbc4c9e1e (master655258480) passes20/20 retained cases in54.26s: the three new cases, six attack-distance controls, four Gold-follow movement/cast/root controls, four actual toy-facing controls and three external-data controls. Format check passes; scoped check and Bo simulator requalification remain pending.

## Sources

- TC a352b1fa `ObjectDefines.h:39–40` — player radius0.388999998569489f/reach1.5f constants.
- TC a352b1fa `Creature.cpp:3591–3610` — model geometry uses objectScale×displayScale.
- `crates/server/src/zones/entry.rs` and `crates/server/src/auras/unit_state.rs` — existing effective `UnitScale`.
- `crates/server/src/scripted_ai/model_reach.rs`, `crates/server/src/creature_aggro_distance.rs` — production reach consumers.
- `crates/server/src/spell_cast/toy_facing.rs` — bounding-radius cast boundary.

## See Also

- [Follower dungeons](follower-dungeons.md) — follower pursuit uses the same model reach.
