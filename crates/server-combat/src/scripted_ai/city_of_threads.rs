//! FANGS_WARMUP_LOG_CORRELATED: LittleWigs' 25.7s bar; proximity is inferred.
use bevy::prelude::*;
use shared::components::{Health, Npc, Player, Position, UnitFlags};

use super::instance_script::{self, EncounterState};
use crate::maps::InMap;

const MAP: u32 = 2669;
const ORATOR: u32 = 2907;
const NX: u32 = 216648;
const VX: u32 = 216649;
const WARMUP_STATE: u32 = 2669001;
const WARMUP_RADIUS: f32 = 20.0;
const WARMUP_SECS: f64 = 25.7;

#[derive(Component)]
struct WarmupDeadline(f64);

pub(super) fn update_fangs_warmup(world: &mut World) {
    let anchors: Vec<_> = world
        .query::<(Entity, &Npc, &InMap, &Position)>()
        .iter(world)
        .filter(|(_, npc, map, _)| npc.template_id == NX && map.map_id == MAP)
        .map(|(entity, _, map, position)| (entity, *map, *position))
        .collect();
    for (nx, map, position) in anchors {
        update_copy(world, nx, map, position);
    }
}

fn update_copy(world: &mut World, nx: Entity, map: InMap, position: Position) {
    if instance_script::get_data(world, map, WARMUP_STATE) != 0 {
        return;
    }
    let done = instance_script::boss_id_for_encounter(world, map, ORATOR)
        .and_then(|id| instance_script::get_boss_state(world, map, id));
    if done != Some(EncounterState::Done) {
        return;
    }
    let now = world.resource::<Time>().elapsed_secs_f64();
    if let Some(deadline) = world.get::<WarmupDeadline>(nx) {
        if now >= deadline.0 {
            activate_fangs(world, nx, map);
        }
        return;
    }
    let nearby = world
        .query::<(&Player, &InMap, &Position, &Health)>()
        .iter(world)
        .any(|(_, copy, player, health)| {
            *copy == map
                && health.current > 0.0
                && (player.x - position.x).powi(2)
                    + (player.y - position.y).powi(2)
                    + (player.z - position.z).powi(2)
                    <= WARMUP_RADIUS.powi(2)
        });
    if nearby {
        world
            .entity_mut(nx)
            .insert(WarmupDeadline(now + WARMUP_SECS));
    }
}

fn activate_fangs(world: &mut World, nx: Entity, map: InMap) {
    let vx = world
        .query::<(Entity, &Npc, &InMap)>()
        .iter(world)
        .find(|(_, npc, copy)| npc.template_id == VX && **copy == map)
        .map(|(entity, _, _)| entity);
    let Some(vx) = vx else { return };
    for unit in [nx, vx] {
        if let Some(mut flags) = world.get_mut::<UnitFlags>(unit) {
            flags.0 &= !(UnitFlags::IMMUNE_TO_PC | UnitFlags::NOT_SELECTABLE);
        }
    }
    world.entity_mut(nx).remove::<WarmupDeadline>();
    instance_script::set_data(world, map, WARMUP_STATE, 1);
}
