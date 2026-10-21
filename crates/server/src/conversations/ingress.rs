//! TC a352b1fa boss_orator_krix_vizk.cpp87–110; AreaTrigger.cpp752–821.
//! Static triggers search players only; account privileges are not GM mode.
use crate::maps::{InMap, MapStore};
use crate::world_area_triggers::WorldAreaTrigger;
use bevy::prelude::*;
use shared::components::{Health, Npc, Player, Position, Rotation, UnitFlags};

pub(super) fn detect(world: &mut World) {
    let areas: Vec<_> = world
        .query::<(Entity, &InMap, &Position, &Rotation, &WorldAreaTrigger)>()
        .iter(world)
        .filter(|(_, _, _, _, area)| conversation_entry(&area.script_name).is_some())
        .map(|(entity, map, position, rotation, area)| {
            (entity, *map, *position, *rotation, area.clone())
        })
        .collect();
    for (entity, map, position, rotation, area) in areas {
        let player = world
            .query_filtered::<(
                Entity,
                &InMap,
                &Position,
                Option<&Health>,
                Option<&UnitFlags>,
                Option<&crate::death::PlayerDeathState>,
            ), With<Player>>()
            .iter(world)
            .filter(|(_, copy, _, health, flags, death)| {
                **copy == map
                    && health.is_none_or(|health| health.current > 0.0)
                    && flags.is_none_or(|flags| flags.is_selectable())
                    && death.is_none_or(|death| death.state == shared::death::DeathState::Alive)
            })
            .find(|(player, _, point, _, _, _)| {
                contains(world, *player, &area, position, rotation, **point)
            })
            .map(|(player, _, point, _, _, _)| (player, *point));
        let Some((player, point)) = player else {
            continue;
        };
        if !world
            .resource::<MapStore>()
            .instance_scripts
            .contains_key(&map.map_id)
        {
            continue;
        }
        let creator = world
            .query::<(Entity, &InMap, &Npc)>()
            .iter(world)
            .find(|(_, copy, npc)| **copy == map && npc.template_id == 216619)
            .map(|(entity, _, _)| entity);
        if let Some(creator) = creator {
            let entry = conversation_entry(&area.script_name).expect("selected source script");
            if let Err(reason) = super::create_private(world, entry, creator, point, player) {
                warn!(
                    entry,
                    ?map,
                    "Source conversation creation refused: {reason}"
                );
            }
        }
        // Source removes the trigger even when GetCreature(data) has no creator.
        world.despawn(entity);
    }
}

fn conversation_entry(script: &str) -> Option<u32> {
    match script {
        "at_orator_conversation_intro_1" => Some(24642),
        "at_orator_conversation_intro_2" => Some(24643),
        "at_orator_conversation_intro_3" => Some(24644),
        _ => None,
    }
}

fn contains(
    world: &World,
    player: Entity,
    area: &WorldAreaTrigger,
    center: Position,
    rotation: Rotation,
    point: Position,
) -> bool {
    let delta = Vec3::new(point.x - center.x, point.y - center.y, point.z - center.z);
    let data = area.shape_data;
    match area.shape {
        0 => {
            let reach = crate::scripted_ai::model_reach::combat_reach(world, player)
                .expect("player source reach");
            delta.length_squared() < (data[0] + reach).powi(2)
        }
        1 => {
            // Bevy Z=-WoW Y; TC box height is [-extentZ/2,+extentZ/2].
            let angle = rotation.y - std::f32::consts::FRAC_PI_2;
            let (sin, cos) = angle.sin_cos();
            let local_x = delta.x * cos - delta.z * sin;
            let local_y = -delta.x * sin - delta.z * cos;
            local_x.abs() <= data[0] && local_y.abs() <= data[1] && delta.y.abs() <= data[2] * 0.5
        }
        _ => false,
    }
}
