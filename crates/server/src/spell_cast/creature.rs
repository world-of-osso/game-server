//! Casts by creature scripts (`UnitAI::DoCast*` → `Unit::CastSpell`).
//!
//! A creature's cast with no explicit target takes its victim when the spell needs
//! a unit target (`Spell::InitExplicitTargets`). A triggered cast
//! (`TRIGGERED_FULL_MASK`) resolves at once without cast time, costs, cooldowns or
//! caster aura checks. Creatures have no global cooldown (`start_gcd`). A
//! creature's melee spells (`DefenseType` MELEE) miss hostile units by its melee
//! miss chance (`Unit::MeleeSpellHitResult` miss row; dodge, parry and block are
//! not modelled), which MOD_HIT_CHANCE lowers.

use bevy::prelude::*;
use shared::components::Npc;
use shared::protocol::{CombatLogKind, MissKind};

use super::{
    CastFailure, TargetPlan, cast_event, handled_effects, log_event, plan_targets, resolve_effects,
    start_cast, targets, validate_target,
};
use crate::creature_ai::CreatureState;
use crate::spell_info::{SpellInfo, SpellStore};

/// `SPELL_DAMAGE_CLASS_MELEE` (`SpellCategories.DefenseType`).
const SPELL_DAMAGE_CLASS_MELEE: u32 = 2;
/// `SPELL_ATTR7_NO_ATTACK_MISS`.
const SPELL_ATTR7_NO_ATTACK_MISS: u32 = 0x0200_0000;
/// Triggered casts resolve without proc events (`TRIGGERED_DISALLOW_PROC_EVENTS`).
const TRIGGERED_DEPTH: u8 = 1;

fn victim(world: &World, creature: Entity) -> Option<Entity> {
    match world.get::<CreatureState>(creature) {
        Some(CreatureState::Pursuing { target }) => Entity::try_from_bits(*target),
        _ => None,
    }
}

/// `Unit::CastSpell(target, spellId, triggered)` for a creature.
pub(crate) fn creature_cast(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    target: Option<Entity>,
    triggered: bool,
) -> Result<(), CastFailure> {
    let spell = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell_id))
        .cloned()
        .ok_or(CastFailure::UnknownSpell)?;
    let needs_unit = targets::explicit_requirement(handled_effects(&spell)).is_some();
    let explicit = target.or_else(|| needs_unit.then(|| victim(world, caster)).flatten());
    if !triggered {
        return start_cast(world, caster, &spell, explicit);
    }
    if !targets::is_alive(world, caster) {
        return Err(CastFailure::CasterDead);
    }
    let explicit = validate_target(world, caster, &spell, explicit)?;
    let plan = plan_targets(world, caster, &spell, explicit)?;
    log_event(
        world,
        cast_event(caster, explicit, &spell, CombatLogKind::CastSuccess),
    );
    resolve_effects(world, caster, &spell, &plan, TRIGGERED_DEPTH);
    Ok(())
}

/// `AISpellInfoType::target` (`UnitAI::FillAISpellInfo`), which `DoCast(spellId)`
/// casts at. `AITARGET_BUFF` (a positive aura on a ranged spell, cast at the caster
/// like `SelfTarget`) is not derived: every spell the ported scripts cast this way
/// is self-targeted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AiTarget {
    SelfTarget,
    Victim,
    Enemy,
    Debuff,
}

const TARGET_UNIT_TARGET_ENEMY: u32 = 6;
const TARGET_UNIT_DEST_AREA_ENEMY: u32 = 16;
const TARGET_DEST_TARGET_ENEMY: u32 = 53;
const SPELL_EFFECT_APPLY_AURA: u32 = 6;

pub(crate) fn ai_spell_target(world: &World, spell_id: u32) -> AiTarget {
    let Some(spell) = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell_id))
    else {
        return AiTarget::SelfTarget;
    };
    let has_range = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.range)
        .is_some_and(|range| range.max_hostile > 0.0);
    let mut target = AiTarget::SelfTarget;
    if !has_range {
        return target;
    }
    for effect in &spell.effects {
        let target_a = effect.implicit_targets[0];
        if target_a == TARGET_UNIT_TARGET_ENEMY || target_a == TARGET_DEST_TARGET_ENEMY {
            target = target.max(AiTarget::Victim);
        } else if target_a == TARGET_UNIT_DEST_AREA_ENEMY {
            target = target.max(AiTarget::Enemy);
        }
        if effect.effect == SPELL_EFFECT_APPLY_AURA && target_a == TARGET_UNIT_TARGET_ENEMY {
            target = target.max(AiTarget::Debuff);
        }
    }
    target
}

fn attribute(spell: &SpellInfo, index: usize) -> u32 {
    spell.misc.as_ref().map_or(0, |misc| misc.attributes[index])
}

/// Remove the hostile units a creature's melee spell misses from every planned
/// effect and log each miss.
pub(super) fn roll_spell_misses(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &mut TargetPlan,
) {
    let melee = spell
        .categories
        .as_ref()
        .is_some_and(|categories| categories.defense_type == SPELL_DAMAGE_CLASS_MELEE);
    if world.get::<Npc>(caster).is_none()
        || !melee
        || attribute(spell, 7) & SPELL_ATTR7_NO_ATTACK_MISS != 0
    {
        return;
    }
    let miss_chance =
        crate::combat::creature_miss_chance(world.get::<crate::auras::AuraStatMods>(caster));
    let mut rolled: Vec<(Entity, bool)> = Vec::new();
    for planned in plan.iter() {
        for &unit in &planned.units {
            if unit == caster
                || rolled.iter().any(|(seen, _)| *seen == unit)
                || !targets::is_hostile(world, caster, unit)
            {
                continue;
            }
            let roll = rand::Rng::gen_range(&mut rand::thread_rng(), 0.0..100.0);
            rolled.push((unit, roll < miss_chance));
        }
    }
    let missed: Vec<Entity> = rolled
        .into_iter()
        .filter_map(|(unit, missed)| missed.then_some(unit))
        .collect();
    for planned in plan.iter_mut() {
        planned.units.retain(|unit| !missed.contains(unit));
    }
    for unit in missed {
        log_event(
            world,
            cast_event(
                caster,
                Some(unit),
                spell,
                CombatLogKind::Miss(MissKind::Miss),
            ),
        );
    }
}
