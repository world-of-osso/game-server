# Model geometry

Production reach and toy-facing geometry use authored creature-model metadata and one explicit project default policy, `MISSING_MODEL_INFO_TC_PLAYER_DEFAULTS_SCALED`. [Implementation](../wiki/systems/model-geometry.md). The default is user-approved project behavior, not a claim that TC spawns creatures with missing model-info.

## What it must do

- [x] Exact authored model-info always wins, including player-race/follower displays whose source reach differs from1.5. No follower-only sizing rule remains.
- [x] An actual display without a native model-info row uses base bounding radius0.389 and combat reach1.5. Both multiply by the same effective `UnitScale` already applied to sourced geometry. Do not multiply display scale twice.
- [ ] World geometry/distance and normal proximity-aggro SystemParam lookups share the same decision. Missing display identity or an uninitialized catalog is not a fabricated model row.
- [x] An ordinary native Shadowguard Champion122403/display124440 with missing source geometry acquires and attacks a nearby player through real proximity aggro.
- [ ] Missing-display scaling and authored-row precedence remain observable; Bo still pursues and attacks through ordinary follower gameplay.

## Ordinary cone close boundary

- [x] Shared caster-facing, destination-facing and entry-conditioned cones accept close targets regardless of arc, while retaining area radius, map, life, faction, conditions and target caps. Line selectors do not gain this exception.
- [x] TC a352b1fa `Spell.cpp:9462–9483`, `Unit.cpp:712–720`: strict 3D center distance must be less than `max(target bounding radius × effective UnitScale, 2yd)`. This uses the target's radius, not the caster's radius or either combat reach.
- [x] Creature radii come from the existing `encounter_creature_model_info` source alongside unchanged combat reaches. Unknown rows, missing display identity or an uninitialized catalog do not invent a cone radius or use the reach/toy default policy. Players use TC's native player radius.
- [ ] Native FoF117418 and Mage120 close-off-arc RED→GREEN, unchanged cone/frontal controls and the two-crate check qualify this change; lead full integration gate remains separate.

## How it works

- [Model geometry](../wiki/systems/model-geometry.md)

## Implementation inventory

- `crates/server/src/scripted_ai/model_reach.rs` — source geometry, shared default decision, scaling and reach-adjusted distance.
- `crates/server/src/creature_aggro_distance.rs` — production source/default geometry in proximity-acquisition calculations.
- `crates/server/src/spell_cast/toy_facing.rs` — authored bounding radius and shared scaled missing-model default.

## Tests asserting this spec

- `crates/server/src/zones_npc_combat_tests.rs`: `missing_model_info_native_champion_proximity_aggroes_a_player`, `missing_model_info_scales_default_reach_but_preserves_authored_reach`.
- `crates/server/src/spell_cast/cone_boundary_tests.rs`: actual117418 1.803yd/33.69° and Mage120 rear-target delivery, with exact2yd, distant and vertical exclusions.
- `crates/server/src/scripted_ai/model_reach_tests.rs`: native model-source radius/scale, zero combat reach and unavailable-row controls.
- `crates/server/src/spell_cast/toy_facing.rs`: `missing_model_info_radius_scales_defaults_and_preserves_authored_radius` verifies missing124440/scaled default and authored42247/boundary.
- Detached native simulator: `alternate_roster_attackers_participate_in_the_first_native_fight` with the actual Bo/Lantha roster; qualified against the same production policy before utility comparisons.

## Known gaps (current cycle)

- [ ] Merged focused retained geometry/aggro/toy checks and Bo requalification. Native3-case RED/GREEN already passes at a25c38d2e.
- [ ] Live-server deployment proof remains separate from source/native test acceptance.

## Out of scope

Inventing native model-info rows, changing world.db, non-spawn policy, guessed creature identities, new aggro-distance formulas, unrelated movement/evade changes and utility scheduling. The former1051-display source inventory remains missing source data; the project decision supplies runtime geometry without relabeling those defaults as authored rows.
