//! Pinned69933 Toy RequiredAreasID; TC a352b1fa SpellInfo::CheckLocation and
//! Player::UpdateAreaDependentAuras. Terrain area and AreaTable ancestry, never map inference.
use std::{collections::HashMap, sync::OnceLock};

use bevy::prelude::*;
use serde::Deserialize;
use shared::components::{Player, Position};

use super::CastFailure;

#[derive(Deserialize)]
struct Requirements {
    spells: HashMap<u32, u32>,
    groups: HashMap<u32, Vec<u32>>,
}

fn requirements() -> &'static Requirements {
    static DATA: OnceLock<Requirements> = OnceLock::new();
    DATA.get_or_init(|| {
        let data: Requirements = serde_json::from_str(include_str!("toy_required_areas.json"))
            .expect("valid generated Toy RequiredAreasID manifest");
        assert!(
            data.spells.values().all(|group| data
                .groups
                .get(group)
                .is_some_and(|areas| !areas.is_empty())),
            "Toy area group has no members"
        );
        data
    })
}

fn load_current_area(world: &mut World, unit: Entity) -> Option<u32> {
    let position = *world.get::<Position>(unit)?;
    let map = world.get::<crate::maps::InMap>(unit)?.map_id;
    let area = world
        .get_resource_mut::<crate::ground::GroundMap>()?
        .area_at(map, Vec3::new(position.x, position.y, position.z))?;
    Some(area)
}

fn permits(world: &World, group: u32, location: Option<u32>) -> bool {
    let Some(area) = location else {
        return false;
    };
    let Some(table) = world.get_resource::<crate::area_table::AreaTable>() else {
        return false;
    };
    requirements().groups[&group]
        .iter()
        .any(|&member| table.is_in_area(area, member))
}

pub(crate) fn check_cast(world: &mut World, caster: Entity, spell: u32) -> Result<(), CastFailure> {
    let Some(&group) = requirements().spells.get(&spell) else {
        return Ok(());
    };
    let location = load_current_area(world, caster);
    if permits(world, group, location) {
        Ok(())
    } else {
        Err(CastFailure::IncorrectArea(group))
    }
}

/// Auras owned by the player are checked at their owner's position, regardless of caster.
pub(crate) fn remove_outside_auras(world: &mut World, owner: Entity) {
    if world.get::<Player>(owner).is_none() {
        return;
    }
    let restricted: Vec<_> = world
        .get::<crate::auras::ActiveAuras>(owner)
        .map(|auras| {
            auras
                .iter()
                .filter_map(|aura| {
                    requirements()
                        .spells
                        .get(&aura.spell_id)
                        .map(|&group| (aura.instance_id, group))
                })
                .collect()
        })
        .unwrap_or_default();
    if restricted.is_empty() {
        return;
    }
    let location = load_current_area(world, owner);
    let removed: Vec<_> = restricted
        .into_iter()
        .filter_map(|(instance, group)| (!permits(world, group, location)).then_some(instance))
        .collect();
    if !removed.is_empty() {
        crate::auras::remove_auras(
            world,
            owner,
            crate::auras::RemoveReason::Interrupted,
            |aura| removed.contains(&aura.instance_id),
        );
    }
}
