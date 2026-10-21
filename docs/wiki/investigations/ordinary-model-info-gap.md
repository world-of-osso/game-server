# Ordinary used-display model-info gap

`ORDINARY_USED_DISPLAY_MODEL_INFO_GAP` is a source-data finding. Runtime sizing now follows the explicitly approved [shared geometry policy](../systems/model-geometry.md); that decision does not turn runtime defaults into authored data. Verified2026-10-21 against canonical world.db and the complete local TDB1210.26091 dump:1051 distinct nonzero displays used by `content_creature_template_model` lack both prepared and raw-source model-info. This is a display inventory, not1051 observed live failures or1051 necessarily spawned creatures.

## Historical fail-closed consumers (before the approved policy)

- `creature_aggro_distance.rs::AggroDistance::combat_reach` returns None for an ordinary NPC without exact model-info. `creature_aggro.rs::PlayerSight::candidate_distance` propagates that None, excluding normal player-proximity acquisition.
- `creature_threat.rs::in_melee_range` returns false on missing self/victim reaches, affecting melee/ranged threat-reselection thresholds. `scripted_ai/smart_ai.rs::process_range_event` treats missing reach-adjusted distance as out of range and rechecks500ms.
- `scripted_ai/caster_chase.rs::plan_caster_step` errors for missing caster/victim reach; the source caster-movement consumer holds. Charge destinations and selected scripted movement/range consumers also require actual reaches.
- This does **not** establish blanket ordinary melee/chase/evade failure. `combat.rs::in_melee_range` uses fixed5yd plus aura range; generic `creature_ai.rs` chase uses2yd and leash/no-path evade use position/navigation state independently.

These were production-code consequences, not a live-server reproduction. The generic policy's real native Champion proximity/white-swing regression now passes; source/default provenance remains explicit.

## Concrete source inventory

Actual spawned examples: Sapphiron15989/display113690/map533; Shadowguard Champion122403/display124440/map1753; Flavor Scientist214673/display119436/map2661. Current Stockade34 and Azure2515 spawn choices have **zero** missing model-info displays, including all id1/id2/id3 alternatives; they are not examples of this ordinary-NPC gap.

`native-model-geometry.json` and `ordinary-geometry-gap-samples.json` under `game-engine/data/diagnostics/follower-utility-2026-10-21/` retain the producer receipt and bounded sample query. `scripts/follower_model_geometry.py` emits only missing used displays backed by actual TDB rows; this snapshot yields zero additions because all available native rows are already imported. No canonical data was changed.

## Boundary

The user-approved [shared model-geometry contract](../../specs/model-geometry.md) replaces follower-only sizing. Actual displays missing model-info now receive the explicit scaled default; authored rows retain precedence. Non-spawn admission remains outside this decision.

## Sources

- [Native model lookup](../../../crates/server/src/scripted_ai/model_reach.rs)
- [Native proximity radius](../../../crates/server/src/creature_aggro_distance.rs)
- [Missing-used-display producer](../../../scripts/follower_model_geometry.py)

## See also

- [[follower-data]] — original follower source/behavior investigations.
- [[dungeon-simulator]] — bounded in-process acceptance, not live proof.
