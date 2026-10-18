//! Implicit target selection (`SpellEffect.implicit_targets`, TrinityCore
//! `Targets` enum IDs).
//!
//! Relations come from `UnitFactionTemplate` reactions (`crate::faction`):
//! an enemy is a valid attack target, an ally a unit the caster regards as
//! friendly. The caster is always its own ally; its duel opponent is an enemy.
//! An invisible unit is neither (`CanSeeOrDetect`), and a `NON_ATTACKABLE_2` one
//! is no enemy (`WorldObject::IsValidAttackTarget`).
//!
//! Handled IDs: 0 none, 1 UNIT_CASTER, 5 UNIT_PET (the caster's summoned hunter pet), 6
//! UNIT_TARGET_ENEMY, 15
//! UNIT_SRC_AREA_ENEMY, 16 UNIT_DEST_AREA_ENEMY, 18 DEST_CASTER, 21
//! UNIT_TARGET_ALLY, 22 SRC_CASTER, 25 UNIT_TARGET_ANY, 30 UNIT_SRC_AREA_ALLY,
//! 31 UNIT_DEST_AREA_ALLY, 32 DEST_CASTER_SUMMON (front-left at max(1yd, combat reach),
//! ignoring radius), 53 DEST_TARGET_ENEMY, 60 UNIT_CONE_ENTRY (units ahead of the
//! caster that meet the effect's `conditions`), 63 DEST_TARGET_ANY, 132
//! DEST_TARGET_ALLY, 65 DEST_TARGET_BACK (the explicit target's position moved the
//! effect radius behind it; collision is not modelled), 76 DEST_CHANNEL_TARGET
//! (the live channel destination), 77 UNIT_CHANNEL_TARGET
//! (the caster's channel target), 92 UNIT_SUMMONER (the native summoned creature's
//! actual summoner), and 28 DEST_DYNOBJ_ENEMY (the units a persistent
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
/// TC a352b1fa SpellInfo::CheckTarget, SharedDefines.h:492.
const SPELL_ATTR1_EXCLUDE_CASTER: u32 = 0x0008_0000;
/// TC a352b1fa SharedDefines.h:666; SpellInfo.cpp:2458–2460.
const SPELL_ATTR6_DO_NOT_CHAIN_TO_CROWD_CONTROLLED_TARGETS: u32 = 0x0000_0100;

fn implicit_target_allowed(world: &World, spell: &SpellInfo, unit: Entity) -> bool {
    let excludes_cc =
        super::attributes(spell, 6) & SPELL_ATTR6_DO_NOT_CHAIN_TO_CROWD_CONTROLLED_TARGETS != 0;
    !excludes_cc || !crate::auras::has_breakable_by_damage_cc(world, unit)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    Enemy,
    Ally,
    Any,
}

/// What one implicit target ID selects.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Selector {
    None,
    Caster,
    /// The explicit (requested) unit target.
    Explicit(Relation),
    ///119: dead party/raid members selected at their stored corpse positions.
    RaidCorpses,
    ///56: living members of the caster's party/raid (and their controlled units).
    RaidArea,
    /// Area center = caster (SRC/DEST_CASTER).
    CenterCaster,
    /// Already selected destination;138 resolves terrain height in the mutable cast-plan step.
    ExistingDest,
    ///72: random point in this slot's radius span around the caster.
    CasterRandom,
    ///86/91: random direction;86 samples the radius span,91 keeps the radius.
    DestRandom,
    /// Destination moved forward (78) or backward (79) by this slot's DB2 radius.
    DestOffset(f32),
    /// 55 DEST_CASTER_FRONT_LEAP: radius of this target slot, collision checked on execution.
    ForwardLeap,
    /// Area center = explicit target (DEST_TARGET_*).
    CenterExplicit(Relation),
    AreaAroundSrc(Relation),
    ///52 GAMEOBJECT_SRC_AREA: physical source objects, not units.
    GameObjectsAroundSource,
    EntryAreaSource,
    EntryAreaDestination,
    AreaAroundDest(Relation),
    ///38 UNIT_NEARBY_ENTRY: nearest qualifying unit in the spell's range.
    NearbyEntry,
    /// Area center = the explicit target's position, the radius behind it.
    DestTargetBack,
    /// The live channel's destination, else its target's current position.
    ChannelDestination,
    /// The caster's channel target.
    ChannelTarget,
    /// The native spell-created creature's actual summoner, not an explicit unit.
    Summoner,
    /// The caster's summoned pet.
    Pet,
    /// Area center = the caster's position moved `angle` (radians from its facing) by
    /// the effect radius, else `DEFAULT_TOTEM_DISTANCE`.
    CasterOffset(f32),
    ///32: fixed pet-follow distance clamped to the caster's source combat reach.
    CasterSummon,
    /// Units a persistent area aura affects: no unit at cast time.
    DynObj(Relation),
    /// Units in the cone ahead of the caster meeting the effect's conditions
    /// (`TARGET_CHECK_ENTRY`).
    ConeEntry,
    ///24: hostile cone along the caster's facing, using native ConeDegrees.
    ConeEnemy,
    ///129: hostile rectangle along caster facing, using native Width/CalcRadius.
    CasterLine,
    ///104: hostile cone aimed from the caster toward the selected destination.
    ConeToDest,
    ///134: hostile line aimed toward the destination, to the effect radius.
    LineToDest,
}

fn selector(id: u32) -> Option<Selector> {
    use Relation::*;
    Some(match id {
        0 | 9 | 17 => Selector::None, // Homebind / spell database destinations handled by teleport.
        1 | 120 => Selector::Caster,
        56 => Selector::RaidArea,
        5 => Selector::Pet,
        119 => Selector::RaidCorpses,
        32 => Selector::CasterSummon,
        44 => Selector::CasterOffset(std::f32::consts::FRAC_PI_4),
        42 => Selector::CasterOffset(-3.0 * std::f32::consts::FRAC_PI_4),
        47 => Selector::CasterOffset(0.0),
        49 => Selector::CasterOffset(-std::f32::consts::FRAC_PI_2),
        50 => Selector::CasterOffset(std::f32::consts::FRAC_PI_2),
        6 => Selector::Explicit(Enemy),
        21 | 57 | 118 => Selector::Explicit(Ally),
        25 => Selector::Explicit(Any),
        18 | 22 | 62 => Selector::CenterCaster,
        87 | 138 | 148 => Selector::ExistingDest,
        78 => Selector::DestOffset(0.0),
        79 => Selector::DestOffset(std::f32::consts::PI),
        72 => Selector::CasterRandom,
        86 | 91 => Selector::DestRandom,
        55 => Selector::ForwardLeap,
        53 => Selector::CenterExplicit(Enemy),
        132 => Selector::CenterExplicit(Ally),
        63 => Selector::CenterExplicit(Any),
        7 => Selector::EntryAreaSource,
        8 => Selector::EntryAreaDestination,
        52 => Selector::GameObjectsAroundSource,
        15 | 151 => Selector::AreaAroundSrc(Enemy),
        30 => Selector::AreaAroundSrc(Ally),
        16 => Selector::AreaAroundDest(Enemy),
        31 => Selector::AreaAroundDest(Ally),
        38 => Selector::NearbyEntry,
        28 => Selector::DynObj(Enemy),
        29 => Selector::DynObj(Ally),
        65 => Selector::DestTargetBack,
        76 => Selector::ChannelDestination,
        77 => Selector::ChannelTarget,
        92 => Selector::Summoner,
        24 => Selector::ConeEnemy,
        // TC SpellInfo.cpp maps both to caster-reference CONE/ENTRY/FRONT.
        60 | 110 => Selector::ConeEntry,
        104 => Selector::ConeToDest,
        129 => Selector::CasterLine,
        134 => Selector::LineToDest,
        _ => return None,
    })
}

/// First implicit target ID of `effect` that has no selector.
pub fn unsupported_target(effect: &SpellEffect) -> Option<u32> {
    effect.implicit_targets.iter().copied().find(|&id| {
        let item_target = id == 26
            && (effect.effect == super::disenchant::EFFECT_DISENCHANT
                || super::open_lock::is_item_effect(effect));
        let gathering_target = id == 23 && effect.effect == 33;
        let unsupported_offset =
            matches!(id, 78 | 79) && (effect.implicit_targets[0] != 18 || radius(effect, 1) <= 0.0);
        unsupported_offset || (selector(id).is_none() && !item_target && !gathering_target)
    })
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

/// TC NeedsToBeTriggeredByCaster: a channel's child supplies a non-caster unit mask.
pub(crate) fn provides_non_caster_unit_target(effect: &SpellEffect) -> bool {
    effect.implicit_targets.into_iter().any(|id| {
        matches!(
            selector(id),
            Some(
                Selector::Explicit(_)
                    | Selector::RaidCorpses
                    | Selector::ChannelTarget
                    | Selector::Summoner
                    | Selector::Pet
                    | Selector::AreaAroundSrc(_)
                    | Selector::AreaAroundDest(_)
                    | Selector::DynObj(_)
                    | Selector::ConeEntry
                    | Selector::ConeEnemy
                    | Selector::CasterLine
                    | Selector::ConeToDest
                    | Selector::LineToDest
            )
        )
    })
}

fn unit_faction(world: &World, entity: Entity) -> UnitFaction {
    // TC GetAffectingPlayer resolves an owned bomb's reaction through its live owner.
    // Its spell origin remains the game object's own position and map.
    let entity = match crate::game_objects::seaforium_bombs::find_caster_controller(world, entity) {
        Some(owner) => owner,
        None => entity,
    };
    UnitFaction {
        template: world
            .get::<UnitFactionTemplate>(entity)
            .map_or(0, |template| template.0),
        is_player: crate::faction::player_affected(
            world.get::<Player>(entity).is_some(),
            world.get::<shared::components::UnitFlags>(entity),
        ),
    }
}

/// `SpellInfo::CheckTargetCreatureType`: a spell with `TargetCreatureType` affects only
/// units whose type bit (`1 << (type - 1)`) it has; a unit without a type passes.
/// Creatures take their template's `type`.
pub fn creature_type_allowed(world: &World, spell: &SpellInfo, target: Entity) -> bool {
    let allowed = spell
        .target_restrictions
        .as_ref()
        .map_or(0, |restrictions| restrictions.target_creature_type);
    if allowed == 0 {
        return true;
    }
    let creature_type = crate::unit_creature_type::get(world, target);
    creature_type == 0 || allowed & crate::unit_creature_type::mask(creature_type) != 0
}

/// `caster` regards `target` as hostile (selects the hostile spell range).
pub fn is_hostile(world: &World, caster: Entity, target: Entity) -> bool {
    if let Some(relation) = crate::pvp::arena_combat::relation(world, caster, target) {
        return matches!(
            relation,
            crate::pvp::arena_combat::ArenaRelation::Enemy { .. }
        );
    }
    if dueling(world, caster, target) {
        return true;
    }
    let templates = world.resource::<FactionTemplates>();
    let (caster, target) = (unit_faction(world, caster), unit_faction(world, target));
    templates.reaction(caster.template, target.template) == Reaction::Hostile
}

/// `Unit::IsFriendlyTo`: the caster's reaction to the target is friendly.
pub fn is_friendly(world: &World, caster: Entity, target: Entity) -> bool {
    if let Some(relation) = crate::pvp::arena_combat::relation(world, caster, target) {
        return relation == crate::pvp::arena_combat::ArenaRelation::Teammate;
    }
    let templates = world.resource::<FactionTemplates>();
    let (caster, target) = (unit_faction(world, caster), unit_faction(world, target));
    templates.reaction(caster.template, target.template) == Reaction::Friendly
}

pub fn relation_matches(world: &World, caster: Entity, target: Entity, relation: Relation) -> bool {
    let arena = crate::pvp::arena_combat::relation(world, caster, target);
    if caster == target {
        return relation != Relation::Enemy;
    }
    if !crate::auras::invisibility::can_detect(world, caster, target) {
        return false;
    }
    if relation == Relation::Any {
        return !matches!(
            arena,
            Some(
                crate::pvp::arena_combat::ArenaRelation::Blocked
                    | crate::pvp::arena_combat::ArenaRelation::Enemy { can_attack: false }
            )
        );
    }
    // WorldObject::IsValidAttackTarget / IsValidAssistTarget (Object.cpp:2374, 2538): an
    // uninteractible unit is neither attacked nor assisted.
    if world
        .get::<shared::components::UnitFlags>(target)
        .is_some_and(|flags| !flags.is_selectable())
    {
        return false;
    }
    if let Some(arena) = arena {
        return match relation {
            Relation::Enemy => {
                matches!(
                    arena,
                    crate::pvp::arena_combat::ArenaRelation::Enemy { can_attack: true }
                ) && world
                    .get::<shared::components::UnitFlags>(target)
                    .is_none_or(|flags| flags.is_attackable())
            }
            _ => arena == crate::pvp::arena_combat::ArenaRelation::Teammate,
        };
    }
    // The duel opponent is hostile: attackable, never assisted.
    let dueling = dueling(world, caster, target);
    let templates = world.resource::<FactionTemplates>();
    let target_entity = target;
    let (caster, target) = (unit_faction(world, caster), unit_faction(world, target));
    let attackable = world
        .get::<shared::components::UnitFlags>(target_entity)
        .is_none_or(|flags| flags.can_be_attacked_by(caster.is_player));
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
/// TC CalcRadius inherits both TargetA's entry and target identity when B has no entry.
fn radius_slot(effect: &SpellEffect, slot: usize) -> usize {
    usize::from(slot == 1 && effect.radius[1].is_some())
}

pub(super) fn radius(effect: &SpellEffect, slot: usize) -> f32 {
    effect.radius[radius_slot(effect, slot)].map_or(0.0, |radius| {
        if radius.radius > 0.0 {
            radius.radius
        } else {
            radius.radius_max
        }
    })
}

/// Unit targets of one effect. Errors name an unsupported target ID. An
/// effect whose slots select no unit and no area (both 0) hits the explicit
/// target, else the caster. `ignore_relation` takes the explicit unit whatever the
/// caster's relation to it.
pub fn select_targets(
    world: &mut World,
    caster: Entity,
    explicit: Option<Entity>,
    spell: &SpellInfo,
    effect: &SpellEffect,
    max_targets: u32,
    ignore_relation: bool,
) -> Result<Vec<Entity>, u32> {
    let caster_pos = position(world, caster).unwrap_or_default();
    let mut center = None;
    let mut units = Vec::new();
    let mut selected_any = false;
    for (slot, &id) in effect.implicit_targets.iter().enumerate() {
        if id == 79 && unsupported_target(effect) == Some(79) {
            return Err(79);
        }
        let selector = selector(id).ok_or(id)?;
        match selector {
            Selector::None | Selector::ForwardLeap => {}
            Selector::ExistingDest => {
                // TC Spell::InitExplicitTargets:666–678 initializes an absent
                // required destination from the supplied object target, else self.
                center = center
                    .or_else(|| spell.cast_destination.map(Vec3::from_array))
                    .or_else(|| position(world, explicit.unwrap_or(caster)));
            }
            Selector::Caster => {
                selected_any = true;
                units.push(caster);
            }
            Selector::Explicit(relation) => {
                selected_any = true;
                if let Some(target) = explicit.filter(|&target| {
                    ignore_relation || relation_matches(world, caster, target, relation)
                }) {
                    units.push(target);
                    units.extend(chain_targets(
                        world, caster, spell, effect, target, relation,
                    ));
                }
            }
            Selector::RaidArea => {
                selected_any = true;
                let mut candidates = area_units(
                    world,
                    caster,
                    caster_pos,
                    radius(effect, slot),
                    Relation::Ally,
                    0,
                );
                candidates.retain(|&unit| shares_raid(world, caster, unit));
                let candidates =
                    limit_implicit_targets(world, spell, caster, candidates, max_targets);
                units.extend(filter_area_sight(
                    world, caster, spell, effect, caster_pos, candidates,
                ));
            }
            Selector::RaidCorpses => {
                selected_any = true;
                units.extend(raid_corpses(
                    world,
                    caster,
                    spell,
                    effect,
                    radius(effect, slot),
                    max_targets,
                ));
            }
            Selector::CenterCaster => center = Some(caster_pos),
            Selector::CenterExplicit(_) => {
                center = explicit.and_then(|target| position(world, target));
            }
            Selector::DestTargetBack => {
                center = explicit.and_then(|target| behind(world, target, radius(effect, slot)));
            }
            Selector::CasterRandom => {
                center = Some(random_destination(caster_pos, effect, slot));
            }
            Selector::DestRandom => {
                let origin = center.unwrap_or(caster_pos);
                center = Some(random_destination(origin, effect, slot));
            }
            Selector::DestOffset(angle) => {
                center = center.map(|point| {
                    offset_destination(
                        point,
                        orientation(world, caster) + angle,
                        radius(effect, slot),
                    )
                });
            }
            Selector::ChannelDestination => {
                center = channel_destination(world, caster).map(|dest| dest.position);
            }
            Selector::ChannelTarget => {
                selected_any = true;
                units.extend(channel_target(world, caster));
            }
            Selector::Summoner => {
                selected_any = true;
                units.extend(find_summoner(world, caster));
            }
            Selector::Pet => {
                selected_any = true;
                units.extend(crate::hunter_pet::summoned_pet(world, caster));
            }
            Selector::CasterOffset(angle) => {
                center = caster_offset(world, caster, angle, radius(effect, slot));
            }
            Selector::CasterSummon => center = caster_summon_point(world, caster),
            Selector::DynObj(_) => selected_any = true,
            Selector::GameObjectsAroundSource => {
                selected_any = true;
                units.extend(area_game_objects(
                    world,
                    caster,
                    center.unwrap_or(caster_pos),
                    radius(effect, slot),
                ));
            }
            Selector::AreaAroundSrc(relation) => {
                selected_any = true;
                let src = center.unwrap_or(caster_pos);
                let candidates = area_units(world, caster, src, radius(effect, slot), relation, 0);
                let candidates =
                    limit_implicit_targets(world, spell, caster, candidates, max_targets);
                units.extend(filter_area_sight(
                    world, caster, spell, effect, src, candidates,
                ));
            }
            Selector::ConeEnemy
            | Selector::CasterLine
            | Selector::ConeToDest
            | Selector::LineToDest => {
                selected_any = true;
                let destination = if matches!(selector, Selector::ConeEnemy | Selector::CasterLine)
                {
                    None
                } else {
                    spell
                        .cast_destination
                        .map(Vec3::from_array)
                        .or_else(|| explicit.and_then(|unit| position(world, unit)))
                };
                units.extend(directed_units(
                    world,
                    caster,
                    spell,
                    effect,
                    slot,
                    destination,
                    max_targets,
                )?);
            }
            Selector::EntryAreaSource | Selector::EntryAreaDestination => {
                selected_any = true;
                let origin = if matches!(selector, Selector::EntryAreaDestination) {
                    center.ok_or(id)?
                } else {
                    center.unwrap_or(caster_pos)
                };
                let candidates =
                    entry_area_units(world, caster, spell, effect, origin, radius(effect, slot));
                let candidates =
                    limit_implicit_targets(world, spell, caster, candidates, max_targets);
                units.extend(filter_area_sight(
                    world, caster, spell, effect, origin, candidates,
                ));
            }
            Selector::ConeEntry => {
                selected_any = true;
                units.extend(cone_entry_units(
                    world,
                    caster,
                    spell,
                    effect,
                    radius(effect, slot),
                ));
            }
            Selector::NearbyEntry => {
                selected_any = true;
                units.extend(nearby_entry_unit(world, caster, spell, effect));
            }
            Selector::AreaAroundDest(relation) => {
                selected_any = true;
                let dest = center
                    .or_else(|| spell.cast_destination.map(Vec3::from_array))
                    .or_else(|| explicit.and_then(|target| position(world, target)))
                    .unwrap_or(caster_pos);
                let candidates = area_units(world, caster, dest, radius(effect, slot), relation, 0);
                let candidates =
                    limit_implicit_targets(world, spell, caster, candidates, max_targets);
                units.extend(filter_area_sight(
                    world, caster, spell, effect, dest, candidates,
                ));
            }
        }
    }
    if !selected_any && effect.implicit_targets == [0, 0] {
        units.push(explicit.unwrap_or(caster));
    }
    units = limit_spell_targets(spell, caster, units, 0);
    units.dedup();
    Ok(units)
}

/// TC a352b1fa SelectImplicitAreaTargets passes center to AddUnitTarget;
/// CheckEffectTarget tests recipient-to-center LOS after the target cap.
fn filter_area_sight(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    center: Vec3,
    units: Vec<Entity>,
) -> Vec<Entity> {
    if super::attributes(spell, 2) & super::SPELL_ATTR2_IGNORE_LINE_OF_SIGHT != 0 {
        return units;
    }
    // SharedDefines.h SPELL_ATTR5_ALWAYS_AOE_LINE_OF_SIGHT and
    // DBCEnums.h SpellEffectAttributes::AlwaysAoeLineOfSight.
    const ALWAYS_AOE_LINE_OF_SIGHT: u32 = 0x0400_0000;
    const EFFECT_ALWAYS_AOE_LINE_OF_SIGHT: u32 = 0x0000_0020;
    let needs_caster_sight = super::attributes(spell, 5) & ALWAYS_AOE_LINE_OF_SIGHT != 0
        || effect.attributes & EFFECT_ALWAYS_AOE_LINE_OF_SIGHT != 0;
    units
        .into_iter()
        .filter(|&unit| {
            let center_clear = crate::line_of_sight::sees_area_center(world, unit, center);
            let caster_clear = !needs_caster_sight
                || unit == caster
                || crate::line_of_sight::sees_in_world(world, caster, unit);
            center_clear && caster_clear
        })
        .collect()
}

fn area_game_objects(world: &mut World, caster: Entity, center: Vec3, radius: f32) -> Vec<Entity> {
    let Some(map) = world.get::<crate::maps::InMap>(caster).copied() else {
        return Vec::new();
    };
    let mut objects = world.query::<(
        Entity,
        &Position,
        &crate::maps::InMap,
        &shared::protocol::GameObjectInfo,
    )>();
    objects
        .iter(world)
        .filter(|(_, at, copy, _)| {
            **copy == map && Vec3::new(at.x, at.y, at.z).distance_squared(center) <= radius * radius
        })
        .map(|(entity, _, _, _)| entity)
        .collect()
}

fn nearby_entry_unit(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> Option<Entity> {
    let negative = crate::spell_info::positivity::negative_effects(
        world.resource::<crate::spell_info::SpellStore>(),
        spell,
    );
    let range = spell_range(spell, negative != 0).1;
    let origin = position(world, caster)?;
    let candidates = area_units(world, caster, origin, range, Relation::Any, 0);
    let qualifying = filter_entry_targets(world, candidates, spell.id, effect.index);
    limit_implicit_targets(world, spell, caster, qualifying, 1)
        .into_iter()
        .next()
}

/// The spell's implicit target conditions for `effect_index` (part of TC CheckTarget).
fn filter_entry_targets(
    world: &World,
    candidates: Vec<Entity>,
    spell_id: u32,
    effect_index: u8,
) -> Vec<Entity> {
    let conditions = world.resource::<crate::conditions::SpellTargetConditions>();
    candidates
        .into_iter()
        .filter(|&unit| conditions.target_meets(world, spell_id, effect_index, unit))
        .collect()
}

pub(super) fn excludes_caster(spell: &SpellInfo) -> bool {
    spell
        .misc
        .as_ref()
        .is_some_and(|misc| misc.attributes[1] & SPELL_ATTR1_EXCLUDE_CASTER != 0)
}

/// TC CheckTarget filters implicit candidates before area/nearby target caps.
fn limit_implicit_targets(
    world: &World,
    spell: &SpellInfo,
    caster: Entity,
    mut units: Vec<Entity>,
    cap: u32,
) -> Vec<Entity> {
    units.retain(|&unit| implicit_target_allowed(world, spell, unit));
    limit_spell_targets(spell, caster, units, cap)
}

fn limit_spell_targets(
    spell: &SpellInfo,
    caster: Entity,
    mut units: Vec<Entity>,
    cap: u32,
) -> Vec<Entity> {
    if excludes_caster(spell) {
        units.retain(|unit| *unit != caster);
    }
    if cap > 0 {
        units.truncate(cap as usize);
    }
    units
}

fn raid_character(world: &World, unit: Entity) -> Option<u64> {
    let controller = match world.get::<shared::components::UnitSummonedBy>(unit) {
        Some(owner) => Entity::try_from_bits(owner.0)?,
        None => unit,
    };
    world
        .get::<crate::persistence::ClientId>(controller)
        .map(|id| id.0)
}

/// TC TARGET_CHECK_RAID uses IsInRaidWith, including a controlled unit's owner.
fn shares_raid(world: &World, caster: Entity, target: Entity) -> bool {
    if caster == target {
        return true;
    }
    let Some(caster_id) = raid_character(world, caster) else {
        return false;
    };
    let Some(target_id) = raid_character(world, target) else {
        return false;
    };
    caster_id == target_id
        || world
            .resource::<crate::group::GroupRegistry>()
            .is_in_same_group(caster_id, target_id)
}

///TC a352b1fa SpellInfo.cpp:367/Spell.cpp:9361 selects raid corpse owners.
///The stored body, not a ghost's graveyard position, determines the radius.
///Corpse state has no separate instance key; owners must remain in the caster's copy.
fn raid_corpses(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    radius: f32,
    cap: u32,
) -> Vec<Entity> {
    let Some(id) = world.get::<crate::persistence::ClientId>(caster) else {
        return Vec::new();
    };
    let members = world
        .resource::<crate::group::GroupRegistry>()
        .group_members(id.0);
    let Some(origin) = position(world, caster) else {
        return Vec::new();
    };
    let Some(map) = world.get::<crate::maps::InMap>(caster).copied() else {
        return Vec::new();
    };
    let mut query = world.query_filtered::<(
        Entity,
        &crate::persistence::ClientId,
        &crate::death::PlayerDeathState,
        &crate::maps::InMap,
    ), With<Player>>();
    let mut bodies: Vec<_> = query
        .iter(world)
        .filter_map(|(unit, id, death, owner_map)| {
            if *owner_map != map
                || !members.contains(&id.0)
                || death.state == shared::death::DeathState::Alive
            {
                return None;
            }
            let body = death.corpse.as_ref()?;
            if u32::from(body.map_id) != map.map_id {
                return None;
            }
            let distance = origin.distance(Vec3::new(body.x, body.y, body.z));
            (distance <= radius).then_some((distance, unit))
        })
        .collect();
    let conditions = world.resource::<crate::conditions::SpellTargetConditions>();
    bodies.retain(|&(_, unit)| conditions.target_meets(world, spell.id, effect.index, unit));
    bodies.sort_by(|a, b| a.0.total_cmp(&b.0));
    if cap > 0 {
        bodies.truncate(cap as usize);
    }
    bodies.into_iter().map(|(_, unit)| unit).collect()
}

///24/129 use caster facing; 104/134 use the caster-to-destination direction.
///129: SpellInfo.cpp377; SpellMgr.cpp3244-3245 marks native Width as CONE_LINE.
///TC a352b1fa Spell.cpp:1273-1324 selects 24 with native ConeAngle/CalcRadius.
///The line uses catalog Width, not a radial explosion or a target-only hit.
///Unit-center geometry; combat-reach boundary expansion is not modelled.
fn directed_units(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    slot: usize,
    destination: Option<Vec3>,
    cap: u32,
) -> Result<Vec<Entity>, u32> {
    let origin = position(world, caster).expect("directional caster position");
    let forward = direction_to_destination(origin, destination, orientation(world, caster));
    let restrictions = spell
        .target_restrictions
        .as_ref()
        .ok_or(effect.implicit_targets[slot])?;
    let half_arc = restrictions.cone_degrees.to_radians() / 2.0;
    let width = restrictions.width;
    let mut units = area_units(
        world,
        caster,
        origin,
        radius(effect, slot),
        Relation::Enemy,
        0,
    );
    units.retain(|&unit| {
        let offset = position(world, unit).expect("area unit position") - origin;
        within_directed_shape(
            offset,
            forward,
            matches!(effect.implicit_targets[slot], 129 | 134),
            width,
            half_arc,
        )
    });
    Ok(limit_implicit_targets(world, spell, caster, units, cap))
}

fn direction_to_destination(origin: Vec3, destination: Option<Vec3>, facing: f32) -> Vec3 {
    let angle = destination
        .filter(|point| point.x != origin.x || point.z != origin.z)
        .map_or(facing, |point| {
            (origin.z - point.z).atan2(point.x - origin.x)
        });
    Vec3::new(angle.cos(), 0.0, -angle.sin())
}

fn within_directed_shape(
    offset: Vec3,
    forward: Vec3,
    line: bool,
    width: f32,
    half_arc: f32,
) -> bool {
    let along = offset.dot(forward);
    let across = (offset.x * forward.z - offset.z * forward.x).abs();
    if line {
        along >= 0.0 && across <= width
    } else {
        across.atan2(along) <= half_arc
    }
}

/// Source7/8: spherical same-copy area with authored entry conditions, not faction substitution.
fn entry_area_units(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    origin: Vec3,
    radius: f32,
) -> Vec<Entity> {
    let map = world.get::<crate::maps::InMap>(caster).copied();
    let targets: Vec<_> = world
        .query::<(Entity, &Position, &Health, &crate::maps::InMap)>()
        .iter(world)
        .filter(|(_, position, health, copy)| {
            let position = Vec3::new(position.x, position.y, position.z);
            health.current > 0.0
                && Some(**copy) == map
                && position.distance_squared(origin) <= radius * radius
        })
        .map(|(entity, ..)| entity)
        .collect();
    let conditions = world.resource::<crate::conditions::SpellTargetConditions>();
    targets
        .into_iter()
        .filter(|&target| conditions.target_meets(world, spell.id, effect.index, target))
        .collect()
}

/// `Spell::SelectImplicitConeTargets` with `TARGET_CHECK_ENTRY` (Spell.cpp:1273-1324):
/// living units of the caster's map inside the `radius` cylinder around it
/// (`WorldObjectSpellAreaTargetCheck`) and within `ConeDegrees` of its facing
/// (`WorldObjectSpellConeTargetCheck`, `HasInArc`), whatever their faction, that meet the
/// effect's implicit target conditions (`WorldObjectSpellTargetCheck`, Spell.cpp:9309-9397).
/// `ConeDegrees` 0 leaves only units straight ahead; the boundary-radius exemption and the
/// cone-back and line variants are not modelled.
fn cone_entry_units(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    radius: f32,
) -> Vec<Entity> {
    let Some(origin) = position(world, caster) else {
        return Vec::new();
    };
    let facing = orientation(world, caster);
    let half_arc = spell
        .target_restrictions
        .as_ref()
        .map_or(0.0, |restrictions| restrictions.cone_degrees)
        .to_radians()
        / 2.0;
    let caster_map = world.get::<crate::maps::InMap>(caster).copied();
    let mut query = world.query::<(Entity, &Position, &Health, &crate::maps::InMap)>();
    let in_cone: Vec<Entity> = query
        .iter(world)
        .filter(|(_, _, health, map)| health.current > 0.0 && Some(**map) == caster_map)
        .filter(|(_, pos, ..)| {
            // WoW (x, y) is Bevy (x, -z); height is Bevy y.
            let (dx, dy) = (pos.x - origin.x, origin.z - pos.z);
            let within = dx.hypot(dy) <= radius && (pos.y - origin.y).abs() <= radius;
            let angle = dy.atan2(dx) - facing;
            let relative = angle.sin().atan2(angle.cos());
            within && relative.abs() <= half_arc
        })
        .map(|(entity, ..)| entity)
        .collect();
    let conditions = world.resource::<crate::conditions::SpellTargetConditions>();
    in_cone
        .into_iter()
        .filter(|&unit| conditions.target_meets(world, spell.id, effect.index, unit))
        .filter(|&unit| implicit_target_allowed(world, spell, unit))
        .collect()
}

/// `SPELL_ATTR2_CHAIN_FROM_CASTER` (SharedDefines.h:522).
const SPELL_ATTR2_CHAIN_FROM_CASTER: u32 = 0x0000_1000;
/// `SpellEffectAttributes::ChainFromInitialTarget` (DBCEnums.h:2425).
const EFFECT_ATTR_CHAIN_FROM_INITIAL_TARGET: u32 = 0x0000_0080;

/// `Spell::SelectImplicitChainTargets` / `SearchChainTargets` (Spell.cpp:1832-1864,
/// 2216-2320): after `target`, up to `EffectChainTargets - 1` more units matching
/// `relation`, each the living one nearest the chain source within the jump radius
/// (7.5 yd ranged, 5 yd melee, 10 yd otherwise), searched within the jump radius ×
/// the jumps left. The source moves to each unit found, unless the spell chains from
/// the caster (`SPELL_ATTR2_CHAIN_FROM_CASTER`, searched over the spell range) or the
/// effect from its initial target (`ChainFromInitialTarget`). Line of sight, the
/// `SPELL_ATTR5_MELEE_CHAIN_TARGETING` front arc (facing is not modelled) and the
/// ChainTargets / ChainJumpDistance spell mods are not modelled.
pub fn chain_targets(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    target: Entity,
    relation: Relation,
) -> Vec<Entity> {
    if effect.chain_targets <= 1 {
        return Vec::new();
    }
    let jumps = (effect.chain_targets - 1) as usize;
    let defense = spell
        .categories
        .as_ref()
        .map_or(0, |categories| categories.defense_type);
    let jump_radius = match defense {
        3 => 7.5,
        2 => 5.0,
        _ => 10.0,
    };
    let from_caster = spell.misc.as_ref().map_or(0, |misc| misc.attributes[2])
        & SPELL_ATTR2_CHAIN_FROM_CASTER
        != 0;
    let from_initial = effect.attributes & EFFECT_ATTR_CHAIN_FROM_INITIAL_TARGET != 0;
    let search_radius = if from_caster {
        spell_range(spell, relation == Relation::Enemy).1
    } else if from_initial {
        jump_radius
    } else {
        jump_radius * jumps as f32
    };
    let mut source = if from_caster { caster } else { target };
    let Some(search_center) = position(world, source) else {
        return Vec::new();
    };
    let mut candidates: Vec<Entity> =
        area_units(world, caster, search_center, search_radius, relation, 0)
            .into_iter()
            .filter(|&unit| unit != target && (unit != caster || !excludes_caster(spell)))
            .filter(|&unit| implicit_target_allowed(world, spell, unit))
            .collect();
    let mut chain = Vec::new();
    while chain.len() < jumps {
        let Some(from) = position(world, source) else {
            break;
        };
        let nearest = candidates
            .iter()
            .enumerate()
            .filter_map(|(slot, &unit)| Some((slot, position(world, unit)?.distance(from))))
            .filter(|&(_, distance)| distance <= jump_radius)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let Some((slot, _)) = nearest else {
            break;
        };
        let unit = candidates.remove(slot);
        if !from_caster && !from_initial {
            source = unit;
        }
        chain.push(unit);
    }
    chain
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

/// `TARGET_DEST_CASTER_FRONT_LEFT`44 and its kin without a radius
/// (`Spell::SelectImplicitCasterDestTargets`, Spell.cpp:1621-1629).
const DEFAULT_TOTEM_DISTANCE: f32 = 3.0;
/// PetDefines.h92, then Spell.cpp1635 clamps to GetCombatReach.
const PET_FOLLOW_DISTANCE: f32 = 1.0;

fn caster_summon_point(world: &World, caster: Entity) -> Option<Vec3> {
    let reach = crate::scripted_ai::model_reach::combat_reach(world, caster)?;
    let distance = PET_FOLLOW_DISTANCE.max(reach);
    caster_offset(world, caster, std::f32::consts::FRAC_PI_4, distance)
}

/// The caster's position moved `dist` yards (`DEFAULT_TOTEM_DISTANCE` when 0) at `angle`
/// from its facing.
fn caster_offset(world: &World, caster: Entity, angle: f32, dist: f32) -> Option<Vec3> {
    let pos = position(world, caster)?;
    let dist = if dist > 0.0 {
        dist
    } else {
        DEFAULT_TOTEM_DISTANCE
    };
    let angle = orientation(world, caster) + angle;
    // WoW (x, y) offsets are Bevy (x, -z).
    Some(Vec3::new(
        pos.x + dist * angle.cos(),
        pos.y,
        pos.z - dist * angle.sin(),
    ))
}

/// TC caster-front47 clamps player offsets to authored scaled reach.
/// Creature reach selection is outside this player Toy boundary.
fn clamp_player_front_distance(world: &World, caster: Entity, target: u32, distance: f32) -> f32 {
    if target != 47 || world.get::<shared::components::Player>(caster).is_none() {
        return distance;
    }
    let reach = crate::scripted_ai::model_reach::combat_reach(world, caster)
        .expect("player combat reach is always available");
    distance.max(reach)
}

/// TC a352b1fa SpellInfo.cpp CalcRadius:72/86 sample (Max-Min)*sqrt(rand_norm());
///91 only randomizes direction. Unlike GetRandomPoint, CalcRadius does not add Min.
fn random_destination(point: Vec3, effect: &SpellEffect, slot: usize) -> Vec3 {
    let mut rng = rand::thread_rng();
    let angle = rand::Rng::gen_range(&mut rng, 0.0..std::f32::consts::TAU);
    let source_slot = radius_slot(effect, slot);
    let maximum = radius(effect, source_slot);
    let distance = if matches!(effect.implicit_targets[source_slot], 72 | 86) {
        let minimum = effect.radius[source_slot].map_or(0.0, |entry| entry.radius_min);
        let area_fraction = rand::Rng::gen_range(&mut rng, 0.0_f32..1.0);
        (maximum - minimum) * area_fraction.sqrt()
    } else {
        maximum
    };
    point + Vec3::new(distance * angle.cos(), 0.0, -distance * angle.sin())
}

/// TC SelectImplicitDestDestTargets: destination facing plus the selector's
/// direction, moved by this slot's CalcRadius. Terrain/collision is not modelled.
fn offset_destination(point: Vec3, angle: f32, distance: f32) -> Vec3 {
    point + Vec3::new(distance * angle.cos(), 0.0, -distance * angle.sin())
}

/// TC Spell.cpp:1760–1763: only a TempSummon supplies UNIT_SUMMONER.
fn find_summoner(world: &World, caster: Entity) -> Option<Entity> {
    let summoner = world.get::<super::summon::SummonedCreature>(caster)?.owner;
    same_map(world, caster, summoner).then_some(summoner)
}

/// The unit the caster channels at (`Spell::m_targets` of its channel).
fn channel_target(world: &World, caster: Entity) -> Option<Entity> {
    let cast = world.get::<CastState>(caster)?;
    (cast.cast_type == CastType::Channel && cast.target != 0)
        .then(|| Entity::try_from_bits(cast.target))
        .flatten()
}

/// TC a352b1fa Spell.cpp:1037–1082: only the active channel supplies selector76.
fn channel_destination(world: &World, caster: Entity) -> Option<Dest> {
    let cast = world.get::<CastState>(caster)?;
    if cast.cast_type != CastType::Channel {
        return None;
    }
    let requested = world
        .get::<super::PendingCastSpell>(caster)
        .and_then(|pending| pending.spell.cast_destination);
    if let Some(point) = requested {
        return Some(Dest {
            position: Vec3::from_array(point),
            orientation: orientation(world, caster),
        });
    }
    let target = channel_target(world, caster)?;
    Some(Dest {
        position: position(world, target)?,
        orientation: orientation(world, target),
    })
}

/// The destination of one effect: the last of its slots that sets one
/// (`SelectImplicitTargetDestTargets`, TargetB after TargetA); the facing is the
/// explicit target's (`EffectTeleportUnits` takes it when the dest has none).
pub fn effect_dest(
    world: &World,
    caster: Entity,
    explicit: Option<Entity>,
    effect: &SpellEffect,
    requested: Option<[f32; 3]>,
) -> Option<Dest> {
    let mut dest = requested.map(|point| Dest {
        position: Vec3::from_array(point),
        orientation: orientation(world, caster),
    });
    for (slot, &id) in effect.implicit_targets.iter().enumerate() {
        let selector = selector(id)?;
        let point = match selector {
            Selector::ChannelDestination => {
                dest = channel_destination(world, caster);
                continue;
            }
            Selector::CenterCaster => position(world, caster),
            Selector::ForwardLeap => caster_offset(world, caster, 0.0, radius(effect, slot)),
            // TC Spell::InitExplicitTargets:666–678; CheckDst must retain the
            // unit-target destination instead of replacing it with the caster.
            Selector::ExistingDest => dest
                .map(|dest: Dest| dest.position)
                .or_else(|| position(world, explicit.unwrap_or(caster))),
            Selector::CasterRandom => {
                position(world, caster).map(|point| random_destination(point, effect, slot))
            }
            Selector::DestRandom => {
                // Native CheckDst initializes an absent destination before applying the offset.
                if dest.is_none() {
                    dest = Some(Dest {
                        position: position(world, caster)?,
                        orientation: orientation(world, caster),
                    });
                }
                dest.map(|dest: Dest| random_destination(dest.position, effect, slot))
            }
            Selector::DestOffset(angle) => dest.map(|dest: Dest| {
                offset_destination(
                    dest.position,
                    dest.orientation + angle,
                    radius(effect, slot),
                )
            }),
            Selector::CenterExplicit(_) => explicit.and_then(|target| position(world, target)),
            Selector::DestTargetBack => {
                explicit.and_then(|target| behind(world, target, radius(effect, slot)))
            }
            Selector::CasterOffset(angle) => {
                let distance = radius(effect, slot);
                let distance = clamp_player_front_distance(world, caster, id, distance);
                caster_offset(world, caster, angle, distance)
            }
            Selector::CasterSummon => caster_summon_point(world, caster),
            _ => continue,
        };
        let facing = match selector {
            Selector::CenterCaster
            | Selector::ForwardLeap
            | Selector::CasterRandom
            | Selector::CasterOffset(_)
            | Selector::CasterSummon => orientation(world, caster),
            Selector::DestRandom | Selector::DestOffset(_) => dest?.orientation,
            Selector::ExistingDest => {
                dest.map_or_else(|| orientation(world, caster), |dest| dest.orientation)
            }
            _ => explicit.map_or_else(
                || orientation(world, caster),
                |target| orientation(world, target),
            ),
        };
        dest = point.map(|position| Dest {
            position,
            orientation: facing,
        });
    }
    if dest.is_none()
        && !effect.implicit_targets.contains(&32)
        && matches!(
            effect.effect,
            super::summon::EFFECT_SUMMON | super::area_trigger::EFFECT_CREATE_AREATRIGGER
        )
    {
        let unit = explicit.unwrap_or(caster);
        dest = position(world, unit).map(|position| Dest {
            position,
            orientation: orientation(world, unit),
        });
    }
    dest
}

///TC caster-ground62 and destination-ground138 resolve map height (`Map::GetHeight`,
///including enabled GO floors in the caster's copy) with mutable ground access.
///Destination selection remains independent of ground-cache mutation.
pub(super) fn ground_destination(
    world: &mut World,
    caster: Entity,
    effect: &SpellEffect,
    destination: Option<Dest>,
) -> Result<Option<Dest>, super::CastFailure> {
    let requires_ground = effect
        .implicit_targets
        .iter()
        .any(|id| matches!(id, 62 | 138));
    if !requires_ground {
        return Ok(destination);
    }
    let mut destination = destination.ok_or(super::CastFailure::InvalidTarget)?;
    let map = world
        .get::<crate::maps::InMap>(caster)
        .copied()
        .ok_or(super::CastFailure::InvalidTarget)?;
    let mut ground = world
        .get_resource_mut::<crate::ground::GroundMap>()
        .ok_or_else(|| {
            super::CastFailure::Unsupported("ground destination requires ground data".into())
        })?;
    let crate::ground::GroundProbe::Supported(height) = ground.ground_in(map, destination.position)
    else {
        return Err(super::CastFailure::InvalidTarget);
    };
    destination.position.y = height;
    Ok(Some(destination))
}

/// Nearby/cone/area/line selections whose radii TC compares before sharing targets.
pub fn shared_selection_checks_radius(effect: &SpellEffect) -> bool {
    effect.implicit_targets.iter().any(|&id| {
        matches!(
            selector(id),
            Some(
                Selector::RaidCorpses
                    | Selector::AreaAroundSrc(_)
                    | Selector::AreaAroundDest(_)
                    | Selector::EntryAreaSource
                    | Selector::EntryAreaDestination
                    | Selector::NearbyEntry
                    | Selector::ConeEntry
                    | Selector::ConeEnemy
                    | Selector::CasterLine
                    | Selector::ConeToDest
                    | Selector::LineToDest
            )
        )
    })
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
