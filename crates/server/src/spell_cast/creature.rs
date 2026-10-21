//! Casts by creature scripts (`UnitAI::DoCast*` → `Unit::CastSpell`).
//!
//! A creature's cast with no explicit target takes its victim when the spell needs
//! a unit target (`Spell::InitExplicitTargets`). A triggered cast
//! (`TRIGGERED_FULL_MASK`) skips cast time, costs, cooldowns and generic crowd
//! control, but still checks implemented caster/target aura metadata. Creatures
//! have no global cooldown (`start_gcd`). A
//! creature's melee spells (`DefenseType` MELEE) miss hostile units by its melee
//! miss chance (`Unit::MeleeSpellHitResult` miss row; dodge, parry and block are
//! not modelled), which MOD_HIT_CHANCE lowers.

use bevy::prelude::*;
use shared::protocol::CombatLogKind;

use super::{
    CastFailure, Sight, cast_event, handled_effects, log_event, plan_targets, start_cast, targets,
    validate_target,
};
use crate::creature_ai::CreatureState;
use crate::spell_info::{SpellInfo, SpellStore};

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
    let (spell, explicit) = load_creature_cast(world, caster, spell_id, target)?;
    if !triggered {
        return start_cast(world, caster, &spell, explicit);
    }
    // Bool FULL_MASK has no triggering aura: TC CheckCast permits a dead caster.
    super::aura_spell_restrictions::check_caster(world, caster, &spell)?;
    let explicit = validate_target(world, caster, &spell, explicit, Sight::Start)?;
    super::aura_spell_restrictions::check_target(world, caster, explicit, &spell)?;
    let plan = plan_targets(world, caster, &spell, explicit, false)?;
    log_event(
        world,
        cast_event(caster, explicit, &spell, CombatLogKind::CastSuccess),
    );
    super::missiles::launch(world, caster, &spell, plan, TRIGGERED_DEPTH);
    Ok(())
}

/// TC source flag0x2: ordinary cast except SpellHistory::IsReady/HandleCooldowns.
/// GCD, cast time, caster-aura and power checks remain independent.
pub(crate) fn creature_cast_ignoring_cooldowns(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    target: Option<Entity>,
) -> Result<(), CastFailure> {
    let (mut spell, explicit) = load_creature_cast(world, caster, spell_id, target)?;
    spell.cast_ignore_cooldowns = true;
    start_cast(world, caster, &spell, explicit)
}

/// TC TRIGGERED_IGNORE_CAST_IN_PROGRESS: retain cast time, costs, cooldowns,
/// caster-aura and school checks; only permit a concurrent scripted cast.
pub(crate) fn creature_cast_ignoring_in_progress(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    target: Option<Entity>,
) -> Result<(), CastFailure> {
    let (spell, explicit) = load_creature_cast(world, caster, spell_id, target)?;
    let target = super::validate_start(world, caster, &spell, explicit, true)?;
    super::begin_validated_cast(world, caster, &spell, target)
}

/// Stop the current cast only after the replacement passes ordinary range,
/// sight, resource, aura, school and cooldown validation. No readiness bypass.
pub(crate) fn creature_cast_interrupting_self(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    target: Option<Entity>,
) -> Result<(), CastFailure> {
    let (spell, explicit) = load_creature_cast(world, caster, spell_id, target)?;
    let target = super::validate_start(world, caster, &spell, explicit, true)?;
    super::cancel_cast(world, caster);
    super::begin_validated_cast(world, caster, &spell, target)
}

fn load_creature_cast(
    world: &World,
    caster: Entity,
    spell_id: u32,
    target: Option<Entity>,
) -> Result<(SpellInfo, Option<Entity>), CastFailure> {
    let effective = crate::auras::overrides::resolve_spell(world, caster, spell_id);
    let spell = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(effective))
        .cloned()
        .ok_or(CastFailure::UnknownSpell)?;
    let requirement = targets::explicit_requirement(handled_effects(&spell));
    let explicit = target.or_else(|| default_scripted_target(world, caster, requirement));
    Ok((spell, explicit))
}

/// TC Spell.cpp621–664 InitExplicitTargets: player selection / creature victim,
/// then caster for a non-enemy unit requirement. Explicit requests are unchanged.
fn default_scripted_target(
    world: &World,
    caster: Entity,
    requirement: Option<targets::Relation>,
) -> Option<Entity> {
    let relation = requirement?;
    let selected = if world.get::<shared::components::Player>(caster).is_some() {
        world
            .get::<crate::networking::CombatTarget>(caster)
            .and_then(|target| target.0)
            .filter(|&target| {
                world.get::<shared::components::Health>(target).is_some()
                    && targets::relation_matches(world, caster, target, relation)
            })
    } else if relation == targets::Relation::Ally {
        None
    } else {
        victim(world, caster)
    };
    selected.or_else(|| (relation != targets::Relation::Enemy).then_some(caster))
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
