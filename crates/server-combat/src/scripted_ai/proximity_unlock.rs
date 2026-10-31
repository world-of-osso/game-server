//! Log-correlated encounter actors that become selectable near a player.
use bevy::prelude::*;
use shared::components::{Health, Npc, Player, Position, UnitFlags};

use super::instance_script;
use crate::combat::DeathState;
use crate::maps::InMap;

struct Unlock {
    map: u32,
    entry: u32,
    radius: f64,
    state_key: u32,
}

// EIRICH_PROXIMITY_LOG_CORRELATED: the largest nearest-player horizontal
// distance at encounter start in WCL 8maLdvGchJ1MK97g/1, v79xdVMArhKQt8jz/3,
// and qLNnp4x613mv2Ctz/10. These samples do not establish the Retail trigger.
const UNLOCKS: &[Unlock] = &[Unlock {
    map: 2652,
    entry: 213119,
    radius: 30.931856444393173,
    state_key: 2_131_190,
}];

#[derive(Component)]
pub(super) struct ProximityUnlock(usize);

pub(super) fn register_actors(
    mut commands: Commands,
    actors: Query<(Entity, &Npc, &InMap), Added<Npc>>,
) {
    for (entity, npc, map) in &actors {
        if let Some(index) = UNLOCKS
            .iter()
            .position(|unlock| unlock.map == map.map_id && unlock.entry == npc.template_id)
        {
            commands.entity(entity).insert(ProximityUnlock(index));
        }
    }
}

pub(super) fn update(world: &mut World) {
    let actors: Vec<_> = world
        .query_filtered::<(Entity, &InMap, &Position, &ProximityUnlock), Without<DeathState>>()
        .iter(world)
        .map(|(entity, map, position, unlock)| (entity, *map, *position, unlock.0))
        .collect();
    for (entity, map, position, index) in actors {
        let unlock = &UNLOCKS[index];
        let already_unlocked = instance_script::get_data(world, map, unlock.state_key) != 0;
        if !already_unlocked && !player_near(world, map, position, unlock.radius) {
            continue;
        }
        if !already_unlocked {
            instance_script::set_data(world, map, unlock.state_key, 1);
        }
        if let Some(mut flags) = world.get_mut::<UnitFlags>(entity)
            && flags.0 & UnitFlags::NOT_SELECTABLE != 0
        {
            flags.0 &= !UnitFlags::NOT_SELECTABLE;
        }
    }
}

fn player_near(world: &mut World, map: InMap, position: Position, radius: f64) -> bool {
    world
        .query::<(&Player, &InMap, &Position, &Health)>()
        .iter(world)
        .any(|(_, player_map, player, health)| {
            let dx = f64::from(player.x) - f64::from(position.x);
            let dz = f64::from(player.z) - f64::from(position.z);
            *player_map == map && health.current > 0.0 && dx * dx + dz * dz <= radius * radius
        })
}
