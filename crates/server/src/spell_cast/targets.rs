//! Implicit target selection (`SpellEffect.implicit_targets`, TrinityCore
//! `Targets` enum IDs).
//!
//! Relations: the server has no faction model, so players are allies of
//! players and NPCs are hostile to players (and the reverse).
//!
//! Handled IDs: 0 none, 1 UNIT_CASTER, 6 UNIT_TARGET_ENEMY, 15
//! UNIT_SRC_AREA_ENEMY, 16 UNIT_DEST_AREA_ENEMY, 18 DEST_CASTER, 21
//! UNIT_TARGET_ALLY, 22 SRC_CASTER, 25 UNIT_TARGET_ANY, 30 UNIT_SRC_AREA_ALLY,
//! 31 UNIT_DEST_AREA_ALLY, 53 DEST_TARGET_ENEMY, 63 DEST_TARGET_ANY, 132
//! DEST_TARGET_ALLY. With no group or pet backend, 56 UNIT_CASTER_AREA_RAID
//! and 120 UNIT_CASTER_AND_SUMMONS resolve to the caster, and 57
//! UNIT_TARGET_RAID and 118 UNIT_TARGET_ALLY_OR_RAID to the ally target.
//! Every other ID is unsupported and fails the cast.

use bevy::prelude::*;
use shared::components::{Health, Npc, Position};

use crate::spell_info::{SpellEffect, SpellInfo};

const MELEE_RANGE: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    Enemy,
    Ally,
    Any,
}

/// What one implicit target ID selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Selector {
    None,
    Caster,
    /// The explicit (requested) unit target.
    Explicit(Relation),
    /// Area center = caster (SRC/DEST_CASTER).
    CenterCaster,
    /// Area center = explicit target (DEST_TARGET_*).
    CenterExplicit(Relation),
    AreaAroundSrc(Relation),
    AreaAroundDest(Relation),
}

fn selector(id: u32) -> Option<Selector> {
    use Relation::*;
    Some(match id {
        0 => Selector::None,
        1 | 56 | 120 => Selector::Caster,
        6 => Selector::Explicit(Enemy),
        21 | 57 | 118 => Selector::Explicit(Ally),
        25 => Selector::Explicit(Any),
        18 | 22 => Selector::CenterCaster,
        53 => Selector::CenterExplicit(Enemy),
        132 => Selector::CenterExplicit(Ally),
        63 => Selector::CenterExplicit(Any),
        15 => Selector::AreaAroundSrc(Enemy),
        30 => Selector::AreaAroundSrc(Ally),
        16 => Selector::AreaAroundDest(Enemy),
        31 => Selector::AreaAroundDest(Ally),
        _ => return None,
    })
}

/// First implicit target ID of `effect` that has no selector.
pub fn unsupported_target(effect: &SpellEffect) -> Option<u32> {
    effect
        .implicit_targets
        .iter()
        .copied()
        .find(|&id| selector(id).is_none())
}

/// Relation the spell requires of its explicit target, from the first
/// effect (of `effects`) selecting one. `None` = no explicit target needed.
pub fn explicit_requirement<'a>(
    effects: impl Iterator<Item = &'a SpellEffect>,
) -> Option<Relation> {
    effects
        .flat_map(|effect| effect.implicit_targets)
        .find_map(|id| match selector(id)? {
            Selector::Explicit(relation) | Selector::CenterExplicit(relation) => Some(relation),
            _ => None,
        })
}

pub fn is_hostile(world: &World, a: Entity, b: Entity) -> bool {
    a != b && world.get::<Npc>(a).is_some() != world.get::<Npc>(b).is_some()
}

pub fn relation_matches(world: &World, caster: Entity, target: Entity, relation: Relation) -> bool {
    match relation {
        Relation::Enemy => is_hostile(world, caster, target),
        Relation::Ally => !is_hostile(world, caster, target),
        Relation::Any => true,
    }
}

pub fn is_alive(world: &World, entity: Entity) -> bool {
    world
        .get::<Health>(entity)
        .is_some_and(|health| health.current > 0.0)
}

pub fn position(world: &World, entity: Entity) -> Option<Vec3> {
    world
        .get::<Position>(entity)
        .map(|pos| Vec3::new(pos.x, pos.y, pos.z))
}

/// `(min, max)` range in yards for a target of this relation; a 0 max is melee.
pub fn spell_range(spell: &SpellInfo, hostile: bool) -> (f32, f32) {
    let range = spell.misc.as_ref().and_then(|misc| misc.range);
    let (min, max) = range.map_or((0.0, 0.0), |range| {
        if hostile {
            (range.min_hostile, range.max_hostile)
        } else {
            (range.min_friendly, range.max_friendly)
        }
    });
    (min, if max > 0.0 { max } else { MELEE_RANGE })
}

/// Radius for target slot `slot` (TrinityCore `CalcRadius`: TargetB prefers
/// the second radius entry).
fn radius(effect: &SpellEffect, slot: usize) -> f32 {
    let entry = if slot == 1 {
        effect.radius[1].or(effect.radius[0])
    } else {
        effect.radius[0]
    };
    entry.map_or(0.0, |radius| {
        if radius.radius > 0.0 {
            radius.radius
        } else {
            radius.radius_max
        }
    })
}

/// Unit targets of one effect. Errors name an unsupported target ID. An
/// effect whose slots select no unit and no area (both 0) hits the explicit
/// target, else the caster.
pub fn select_targets(
    world: &mut World,
    caster: Entity,
    explicit: Option<Entity>,
    effect: &SpellEffect,
    max_targets: u32,
) -> Result<Vec<Entity>, u32> {
    let caster_pos = position(world, caster).unwrap_or_default();
    let mut center = None;
    let mut units = Vec::new();
    let mut selected_any = false;
    for (slot, &id) in effect.implicit_targets.iter().enumerate() {
        let selector = selector(id).ok_or(id)?;
        match selector {
            Selector::None => {}
            Selector::Caster => {
                selected_any = true;
                units.push(caster);
            }
            Selector::Explicit(relation) => {
                selected_any = true;
                units.extend(
                    explicit.filter(|&target| relation_matches(world, caster, target, relation)),
                );
            }
            Selector::CenterCaster => center = Some(caster_pos),
            Selector::CenterExplicit(_) => {
                center = explicit.and_then(|target| position(world, target));
            }
            Selector::AreaAroundSrc(relation) => {
                selected_any = true;
                let src = center.unwrap_or(caster_pos);
                units.extend(area_units(
                    world,
                    caster,
                    src,
                    radius(effect, slot),
                    relation,
                    max_targets,
                ));
            }
            Selector::AreaAroundDest(relation) => {
                selected_any = true;
                let dest = center
                    .or_else(|| explicit.and_then(|target| position(world, target)))
                    .unwrap_or(caster_pos);
                units.extend(area_units(
                    world,
                    caster,
                    dest,
                    radius(effect, slot),
                    relation,
                    max_targets,
                ));
            }
        }
    }
    if !selected_any && effect.implicit_targets == [0, 0] {
        units.push(explicit.unwrap_or(caster));
    }
    units.dedup();
    Ok(units)
}

/// Living units within `radius` of `center` matching `relation`, nearest
/// first, capped by `max_targets` (0 = no cap).
fn area_units(
    world: &mut World,
    caster: Entity,
    center: Vec3,
    radius: f32,
    relation: Relation,
    max_targets: u32,
) -> Vec<Entity> {
    let mut query = world.query::<(Entity, &Position, &Health)>();
    let mut found: Vec<(f32, Entity)> = query
        .iter(world)
        .filter(|(_, _, health)| health.current > 0.0)
        .map(|(entity, pos, _)| (Vec3::new(pos.x, pos.y, pos.z).distance(center), entity))
        .filter(|&(distance, _)| distance <= radius)
        .collect();
    found.retain(|&(_, entity)| relation_matches(world, caster, entity, relation));
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    if max_targets > 0 {
        found.truncate(max_targets as usize);
    }
    found.into_iter().map(|(_, entity)| entity).collect()
}
