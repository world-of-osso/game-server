//! Implicit target selection (`SpellEffect.implicit_targets`, TrinityCore
//! `Targets` enum IDs).
//!
//! Relations come from `UnitFactionTemplate` reactions (`crate::faction`):
//! an enemy is a valid attack target, an ally a unit the caster regards as
//! friendly. The caster is always its own ally; its duel opponent is an enemy.
//! An invisible unit is neither (`CanSeeOrDetect`), and a `NON_ATTACKABLE_2` one
//! is no enemy (`WorldObject::IsValidAttackTarget`).
//!
//! Handled IDs: 0 none, 1 UNIT_CASTER, 6 UNIT_TARGET_ENEMY, 15
//! UNIT_SRC_AREA_ENEMY, 16 UNIT_DEST_AREA_ENEMY, 18 DEST_CASTER, 21
//! UNIT_TARGET_ALLY, 22 SRC_CASTER, 25 UNIT_TARGET_ANY, 30 UNIT_SRC_AREA_ALLY,
//! 31 UNIT_DEST_AREA_ALLY, 53 DEST_TARGET_ENEMY, 63 DEST_TARGET_ANY, 132
//! DEST_TARGET_ALLY, 65 DEST_TARGET_BACK (the explicit target's position moved the
//! effect radius behind it; collision is not modelled), 77 UNIT_CHANNEL_TARGET
//! (the caster's channel target) and 28 DEST_DYNOBJ_ENEMY (the units a persistent
//! area aura affects, `spell_cast::area_aura`). With no group or pet backend, 56 UNIT_CASTER_AREA_RAID
//! and 120 UNIT_CASTER_AND_SUMMONS resolve to the caster, and 57
//! UNIT_TARGET_RAID and 118 UNIT_TARGET_ALLY_OR_RAID to the ally target.
//! Every other ID is unsupported and fails the cast.

use bevy::prelude::*;
use shared::casting::{CastState, CastType};
use shared::components::{Health, Player, Position, Rotation, UnitFactionTemplate};

use crate::faction::{FactionTemplates, Reaction, UnitFaction};

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
    /// Area center = the explicit target's position, the radius behind it.
    DestTargetBack,
    /// The caster's channel target.
    ChannelTarget,
    /// Units a persistent area aura affects: no unit at cast time.
    DynObj(Relation),
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
        28 => Selector::DynObj(Enemy),
        65 => Selector::DestTargetBack,
        77 => Selector::ChannelTarget,
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
            Selector::DestTargetBack => Some(Relation::Any),
            _ => None,
        })
}

fn unit_faction(world: &World, entity: Entity) -> UnitFaction {
    UnitFaction {
        template: world
            .get::<UnitFactionTemplate>(entity)
            .map_or(0, |template| template.0),
        is_player: world.get::<Player>(entity).is_some(),
    }
}

/// `caster` regards `target` as hostile (selects the hostile spell range).
pub fn is_hostile(world: &World, caster: Entity, target: Entity) -> bool {
    if dueling(world, caster, target) {
        return true;
    }
    let templates = world.resource::<FactionTemplates>();
    let (caster, target) = (unit_faction(world, caster), unit_faction(world, target));
    templates.reaction(caster.template, target.template) == Reaction::Hostile
}

pub fn relation_matches(world: &World, caster: Entity, target: Entity, relation: Relation) -> bool {
    if relation == Relation::Any {
        return true;
    }
    if caster == target {
        return relation == Relation::Ally;
    }
    if world.get::<crate::auras::Invisible>(target).is_some() {
        return false;
    }
    // WorldObject::IsValidAttackTarget / IsValidAssistTarget (Object.cpp:2374, 2538): an
    // uninteractible unit is neither attacked nor assisted.
    if world
        .get::<shared::components::UnitFlags>(target)
        .is_some_and(|flags| !flags.is_selectable())
    {
        return false;
    }
    // The duel opponent is hostile: attackable, never assisted.
    let dueling = dueling(world, caster, target);
    let templates = world.resource::<FactionTemplates>();
    let target_entity = target;
    let (caster, target) = (unit_faction(world, caster), unit_faction(world, target));
    let attackable = world
        .get::<shared::components::UnitFlags>(target_entity)
        .is_none_or(|flags| flags.is_attackable());
    match relation {
        Relation::Enemy => attackable && (dueling || templates.can_attack(caster, target)),
        _ => !dueling && templates.can_assist(caster, target),
    }
}

fn dueling(world: &World, caster: Entity, target: Entity) -> bool {
    crate::duel::dueling(world.get::<crate::duel::DuelOpponent>(caster), target)
}

pub fn is_alive(world: &World, entity: Entity) -> bool {
    world
        .get::<Health>(entity)
        .is_some_and(|health| health.current > 0.0)
}

/// Both units are in one map copy.
pub fn same_map(world: &World, a: Entity, b: Entity) -> bool {
    let map = world.get::<crate::maps::InMap>(a);
    map.is_some() && map == world.get::<crate::maps::InMap>(b)
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
            Selector::DestTargetBack => {
                center = explicit.and_then(|target| behind(world, target, radius(effect, slot)));
            }
            Selector::ChannelTarget => {
                selected_any = true;
                units.extend(channel_target(world, caster));
            }
            Selector::DynObj(_) => selected_any = true,
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

/// Where an effect's destination targets put it (`SpellCastTargets::GetDstPos`),
/// Bevy space, with the facing (WoW radians) a unit teleported there takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dest {
    pub position: Vec3,
    pub orientation: f32,
}

/// WoW orientation of a unit (`Rotation.y` is the orientation + pi/2).
pub fn orientation(world: &World, unit: Entity) -> f32 {
    world
        .get::<Rotation>(unit)
        .map_or(0.0, |rotation| rotation.y - std::f32::consts::FRAC_PI_2)
}

/// `dist` yards behind `target` (`TARGET_DEST_TARGET_BACK`: `CalcDirectionAngle` pi
/// from the target's facing), at the target's height.
fn behind(world: &World, target: Entity, dist: f32) -> Option<Vec3> {
    let pos = position(world, target)?;
    let angle = orientation(world, target) + std::f32::consts::PI;
    // WoW (x, y) offsets are Bevy (x, -z).
    Some(Vec3::new(
        pos.x + dist * angle.cos(),
        pos.y,
        pos.z - dist * angle.sin(),
    ))
}

/// The unit the caster channels at (`Spell::m_targets` of its channel).
fn channel_target(world: &World, caster: Entity) -> Option<Entity> {
    let cast = world.get::<CastState>(caster)?;
    (cast.cast_type == CastType::Channel && cast.target != 0)
        .then(|| Entity::try_from_bits(cast.target))
        .flatten()
}

/// The destination of one effect: the last of its slots that sets one
/// (`SelectImplicitTargetDestTargets`, TargetB after TargetA); the facing is the
/// explicit target's (`EffectTeleportUnits` takes it when the dest has none).
pub fn effect_dest(
    world: &World,
    caster: Entity,
    explicit: Option<Entity>,
    effect: &SpellEffect,
) -> Option<Dest> {
    let mut dest = None;
    for (slot, &id) in effect.implicit_targets.iter().enumerate() {
        let point = match selector(id)? {
            Selector::CenterCaster => position(world, caster),
            Selector::CenterExplicit(_) => explicit.and_then(|target| position(world, target)),
            Selector::DestTargetBack => {
                explicit.and_then(|target| behind(world, target, radius(effect, slot)))
            }
            _ => continue,
        };
        let facing = explicit.map_or_else(
            || orientation(world, caster),
            |target| orientation(world, target),
        );
        dest = point.map(|position| Dest {
            position,
            orientation: facing,
        });
    }
    dest
}

/// Radius of the effect's first target slot (`SpellEffectInfo::CalcRadius`).
pub fn effect_radius(effect: &SpellEffect) -> f32 {
    radius(effect, 0)
}

/// Living units within `radius` of `center` matching `relation`, nearest
/// first, capped by `max_targets` (0 = no cap).
pub(crate) fn area_units(
    world: &mut World,
    caster: Entity,
    center: Vec3,
    radius: f32,
    relation: Relation,
    max_targets: u32,
) -> Vec<Entity> {
    let caster_map = world.get::<crate::maps::InMap>(caster).copied();
    let mut query = world.query::<(Entity, &Position, &Health, &crate::maps::InMap)>();
    let mut found: Vec<(f32, Entity)> = query
        .iter(world)
        .filter(|(_, _, health, map)| health.current > 0.0 && Some(**map) == caster_map)
        .map(|(entity, pos, _, _)| (Vec3::new(pos.x, pos.y, pos.z).distance(center), entity))
        .filter(|&(distance, _)| distance <= radius)
        .collect();
    found.retain(|&(_, entity)| relation_matches(world, caster, entity, relation));
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    if max_targets > 0 {
        found.truncate(max_targets as usize);
    }
    found.into_iter().map(|(_, entity)| entity).collect()
}
