//! TC a352b1fa aura395 / at_kings_rest_molten_gold, source12928 sphere.
//! Gameplay lifecycle only; native area-trigger visuals remain unconverted.
use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use shared::components::{Player, Position, UnitScale};

use crate::auras::{ActiveAuras, AuraInstance, RemoveReason};
use crate::maps::InMap;

const DAMAGE: u32 = 265914;
const AURA: u32 = 265915;
const CREATE_PROPERTIES: i32 = 12928;
// Player.cpp1573–1577 and Object.cpp408–427: object-scaled Player reach.
const PLAYER_COMBAT_REACH: f32 = 1.5;

#[derive(Resource)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct MoltenGoldAreas {
    radius: f32,
    active: HashMap<Entity, MoltenGoldArea>,
}

#[cfg_attr(test, derive(Clone))]
struct MoltenGoldArea {
    aura_instance: u32,
    members: HashSet<Entity>,
}

/// Admit the actual static sphere, not a radius substituted for unknown geometry.
pub(crate) fn load_areas(connection: &rusqlite::Connection) -> rusqlite::Result<MoltenGoldAreas> {
    let radius = connection.query_row(
        "SELECT ShapeData0 FROM tdb_areatrigger_create_properties \
         WHERE Id=12928 AND IsCustom=0 AND AreaTriggerId=17570 \
         AND ScriptName='at_kings_rest_molten_gold' AND Flags=4 AND Shape=0 \
         AND MoveCurveId=0 AND ScaleCurveId=0 AND MorphCurveId=0 \
         AND ShapeData0=ShapeData1 AND ShapeData0>0",
        [],
        |row| row.get(0),
    )?;
    Ok(MoltenGoldAreas {
        radius,
        active: HashMap::new(),
    })
}

pub(crate) fn handles_effect(spell_id: u32, effect: &crate::auras::AuraEffectState) -> bool {
    let molten_creator = spell_id == AURA && effect.index == 0;
    let source_geometry = effect.aura_type == 395 && effect.misc_values[0] == CREATE_PROPERTIES;
    molten_creator && source_geometry
}

fn creates_area(aura: &AuraInstance) -> bool {
    aura.effects
        .iter()
        .any(|effect| handles_effect(aura.spell_id, effect))
}

pub(super) fn apply(world: &mut World, target: Entity, aura: &AuraInstance) {
    if !creates_area(aura) {
        return;
    }
    world.resource_mut::<MoltenGoldAreas>().active.insert(
        target,
        MoltenGoldArea {
            aura_instance: aura.instance_id,
            members: HashSet::new(),
        },
    );
}

pub(super) fn remove(world: &mut World, target: Entity, aura: &AuraInstance) {
    if !creates_area(aura) {
        return;
    }
    let area = world
        .resource_mut::<MoltenGoldAreas>()
        .active
        .remove(&target);
    if let Some(area) = area {
        for player in area.members {
            exit(world, player);
        }
    }
}

fn exit(world: &mut World, player: Entity) {
    if world.get_entity(player).is_ok() {
        crate::auras::remove_auras(world, player, RemoveReason::Cancelled, |aura| {
            aura.spell_id == DAMAGE
        });
    }
}

fn area_origin(world: &World, target: Entity, instance: u32) -> Option<(Position, InMap)> {
    let aura = world.get::<ActiveAuras>(target)?;
    if !aura
        .iter()
        .any(|a| a.spell_id == AURA && a.instance_id == instance)
    {
        return None;
    }
    Some((
        *world.get::<Position>(target)?,
        *world.get::<InMap>(target)?,
    ))
}

fn find_members(world: &mut World, center: Position, map: InMap, radius: f32) -> HashSet<Entity> {
    world
        .query_filtered::<(Entity, &Position, &InMap, Option<&UnitScale>), With<Player>>()
        .iter(world)
        .filter_map(|(player, position, player_map, scale)| {
            let reach = PLAYER_COMBAT_REACH * scale.map_or(1.0, |scale| scale.0);
            let distance_squared = (position.x - center.x).powi(2)
                + (position.y - center.y).powi(2)
                + (position.z - center.z).powi(2);
            (*player_map == map && distance_squared < (radius + reach).powi(2)).then_some(player)
        })
        .collect()
}

fn update_area(world: &mut World, area: &mut MoltenGoldArea, members: HashSet<Entity>) {
    for &player in members.difference(&area.members) {
        // Source OnUnitEnter casts on the entering Player with ordinary flags.
        if let Err(failure) = crate::spell_cast::creature_cast(world, player, DAMAGE, None, false) {
            debug!("Molten Gold OnUnitEnter cast refused: {failure:?}");
        }
    }
    for &player in area.members.difference(&members) {
        exit(world, player);
    }
    area.members = members;
}

pub(crate) fn update_areas(world: &mut World) {
    // Harnesses without WorldDataPlugin need no geometry unless this aura is used.
    if !world.contains_resource::<MoltenGoldAreas>() {
        return;
    }
    world.resource_scope(|world, mut areas: Mut<MoltenGoldAreas>| {
        let radius = areas.radius;
        areas.active.retain(|target, area| {
            let Some((center, map)) = area_origin(world, *target, area.aura_instance) else {
                update_area(world, area, HashSet::new());
                return false;
            };
            let members = find_members(world, center, map, radius);
            update_area(world, area, members);
            true
        });
    });
}
