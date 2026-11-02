//! INFERRED_PETALWING_TAXI: retire when authentic spellclick/vehicle data is available.
use bevy::prelude::*;
use shared::components::{Mounted, MovementControl, Npc, Player, Position, Rotation};

use crate::maps::InMap;
use crate::scripted_ai::instance_script::{self, EncounterState};

pub(crate) const MAP: u32 = 2859;
pub(crate) const ENTRY: u32 = 247402;
pub(crate) const PATH: u32 = 10799;
const REQUIRED_ENCOUNTERS: [u32; 2] = [3199, 3200];

fn unlocked(world: &mut World, map: InMap) -> bool {
    REQUIRED_ENCOUNTERS.iter().all(|&encounter| {
        instance_script::boss_id_for_encounter(world, map, encounter)
            .and_then(|id| instance_script::get_boss_state(world, map, id))
            == Some(EncounterState::Done)
    })
}

pub(crate) fn update_visibility(world: &mut World) {
    let mut birds: Vec<_> =
        crate::system_states::with_query::<(Entity, &Npc, &InMap), (), _>(world, |world, units| {
            units
                .iter(world)
                .filter_map(|(entity, npc, map)| {
                    (map.map_id == MAP && npc.template_id == ENTRY).then_some((entity, *map))
                })
                .collect()
        });
    birds.sort_unstable_by_key(|(bird, _)| *bird);
    for (bird, map) in birds {
        let hidden = !unlocked(world, map);
        crate::interest::set_hidden(world, bird, hidden);
    }
}

/// Start only the activator's flight; followers approach and use the NPC themselves.
pub(crate) fn traverse(world: &mut World, rider: Entity, bird: Entity) -> Result<(), String> {
    let map = *world
        .get::<InMap>(rider)
        .ok_or("Petalwing rider lacks map")?;
    if map.map_id != MAP || world.get::<InMap>(bird) != Some(&map) {
        return Err("Petalwing rider and source must share the Blinding Vale copy".into());
    }
    if world
        .get::<Npc>(bird)
        .is_none_or(|npc| npc.template_id != ENTRY)
    {
        return Err("Petalwing source has the wrong NPC entry".into());
    }
    if !unlocked(world, map) {
        return Err("Petalwing requires both opening encounters".into());
    }
    let position = world
        .get::<Position>(rider)
        .ok_or("Petalwing rider lacks position")?;
    let source = world
        .get::<Position>(bird)
        .ok_or("Petalwing source lacks position")?;
    if Vec3::new(position.x, position.y, position.z)
        .distance(Vec3::new(source.x, source.y, source.z))
        > crate::npc_interaction::INTERACTION_DISTANCE
    {
        return Err("Petalwing rider is outside interaction range".into());
    }
    if !crate::spell_cast::targets::is_alive(world, rider)
        || world.get::<crate::taxi::TaxiFlight>(rider).is_some()
        || world.get::<Mounted>(rider).is_some()
        || world.get::<Rotation>(rider).is_none()
        || world
            .get::<MovementControl>(rider)
            .is_some_and(|control| control.controlled)
    {
        return Err("Petalwing rider cannot start flight".into());
    }
    let points = world
        .get_resource::<crate::taxi::TaxiNetwork>()
        .and_then(|network| network.scripted_path(PATH))
        .ok_or("Petalwing TaxiPath 10799 unavailable")?;
    let destination = *points.last().ok_or("Petalwing TaxiPath 10799 is empty")?;
    world
        .entity_mut(rider)
        .insert(crate::taxi::TaxiFlight { points, next: 0 });
    if let Some(mut control) = world.get_mut::<MovementControl>(rider) {
        control.controlled = true;
    }
    if world.get::<Player>(rider).is_some() {
        crate::followers::transport_follow::record_use_to(
            world,
            rider,
            bird,
            crate::followers::transport_follow::TransportKind::Petalwing,
            destination,
        );
    }
    Ok(())
}
