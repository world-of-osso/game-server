//! Stay in the TC Candle King world-static candlelight during the encounter.
use bevy::prelude::*;
use shared::components::{CombatStatus, Npc, Position, Rotation};

use crate::maps::InMap;
use crate::navigation::{Goal, Navigation};
use crate::world_area_triggers::{WorldAreaTrigger, candlelight_contains};

fn light(world: &mut World, unit: Entity) -> Option<(WorldAreaTrigger, Position, Rotation)> {
    let map = *world.get::<InMap>(unit)?;
    if map.map_id != 2651 {
        return None;
    }
    let fighting = world
        .query::<(&Npc, &InMap, &CombatStatus)>()
        .iter(world)
        .any(|(npc, copy, combat)| npc.template_id == 208745 && *copy == map && combat.0);
    if !fighting {
        return None;
    }
    world
        .query::<(&WorldAreaTrigger, &Position, &Rotation, &InMap)>()
        .iter(world)
        .find(|(area, _, _, copy)| {
            **copy == map && area.script_name == "at_the_candle_king_candlelight"
        })
        .map(|(area, center, rotation, _)| (area.clone(), *center, *rotation))
}

/// Move through native navigation to the authored light center if outside it.
pub(super) fn seek(world: &mut World, unit: Entity) -> bool {
    let Some((area, center, rotation)) = light(world, unit) else {
        return false;
    };
    let Some(from) = world.get::<Position>(unit).copied() else {
        return false;
    };
    if candlelight_contains(&area, center, rotation, from) {
        return false;
    }
    crate::spell_cast::cancel_cast(world, unit);
    world
        .entity_mut(unit)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert(crate::creature_ai::CreatureState::Idle);
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return true;
    };
    if world.get_resource::<Navigation>().is_none() {
        error!(?unit, "Candlelight seeking requires navigation");
        return true;
    }
    super::hazards::advance_escape(world, unit, map.map_id, from, center, Goal::Point);
    true
}

/// Do not chase out of the source light while the encounter is active.
pub(super) fn hold_pursuit(world: &mut World, unit: Entity, target: Entity) -> bool {
    let Some((area, center, rotation)) = light(world, unit) else {
        return false;
    };
    let Some(target_position) = world.get::<Position>(target).copied() else {
        return false;
    };
    if candlelight_contains(&area, center, rotation, target_position) {
        return false;
    }
    world
        .entity_mut(unit)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert(crate::creature_ai::CreatureState::Idle);
    true
}
