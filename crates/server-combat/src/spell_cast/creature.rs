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

mod explicit_targets;

/// Triggered casts resolve without proc events (`TRIGGERED_DISALLOW_PROC_EVENTS`).
const TRIGGERED_DEPTH: u8 = 1;

fn victim(world: &World, creature: Entity) -> Option<Entity> {
    match world.get::<CreatureState>(creature) {
        Some(CreatureState::Pursuing { target }) => Entity::try_from_bits(*target),
        _ => None,
    }
}

/// `Unit::CastSpell(target, spellId, triggered)` for a creature.
pub fn creature_cast(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    target: Option<Entity>,
    triggered: bool,
) -> Result<(), CastFailure> {
    let (mut spell, explicit) = load_creature_cast(world, caster, spell_id, target)?;
    if !triggered {
        return start_cast(world, caster, &spell, explicit);
    }
    spell.cast_dont_reset_periodic_timer = true;
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

/// Ordinary NPC cast with a separate selected action context.
pub fn creature_cast_with_targets(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    targets: super::CastTargets,
) -> Result<(), CastFailure> {
    let (mut spell, explicit) = load_creature_cast(world, caster, spell_id, Some(targets.unit))?;
    spell.cast_destination = targets.destination;
    super::start_cast_with_primary(world, caster, &spell, explicit, targets.primary)
}

/// Ordinary NPC empower admission with an explicitly selected release stage.
pub(crate) fn creature_cast_empowered(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    target: Option<Entity>,
    stage: u8,
) -> Result<(), CastFailure> {
    let (mut spell, explicit) = load_creature_cast(world, caster, spell_id, target)?;
    if stage == 0 || spell.empower_stages.is_empty() {
        return Err(CastFailure::Unsupported(format!(
            "spell{spell_id} cannot empower to stage{stage}"
        )));
    }
    spell.cast_empower_to = Some(stage);
    start_cast(world, caster, &spell, explicit)
}

/// Ordinary empower cast retaining its independent selected hostile primary.
pub fn creature_cast_empowered_with_targets(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    targets: super::CastTargets,
    stage: u8,
) -> Result<(), CastFailure> {
    let (mut spell, explicit) = load_creature_cast(world, caster, spell_id, Some(targets.unit))?;
    if stage == 0 || spell.empower_stages.is_empty() {
        return Err(CastFailure::Unsupported(format!(
            "spell{spell_id} cannot empower to stage{stage}"
        )));
    }
    spell.cast_empower_to = Some(stage);
    spell.cast_destination = targets.destination;
    super::start_cast_with_primary(world, caster, &spell, explicit, targets.primary)
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

/// TC partial-trigger casts may override BP0 without bypassing native costs,
/// cooldowns, aura checks or child proc behavior (Execution Sentence expiry).
pub(crate) fn creature_cast_with_effect0_ignoring_in_progress(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    target: Option<Entity>,
    amount: f32,
) -> Result<(), CastFailure> {
    let (mut spell, explicit) = load_creature_cast(world, caster, spell_id, target)?;
    let effect = spell
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .ok_or(CastFailure::Unsupported(
            "partial-trigger BP0 effect missing".into(),
        ))?;
    effect.base_points = amount;
    super::scripts::prepare_stored_damage(&mut spell);
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
    let mut spell = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell_id))
        .cloned()
        .ok_or(CastFailure::UnknownSpell)?;
    let requirement = targets::explicit_requirement(handled_effects(&spell));
    let supplied = target.or_else(|| default_scripted_target(world, caster, requirement));
    // Normalize only the redundant self object rejected by EXCLUDE_CASTER.
    // Other supplied objects still own this runtime's cast/packet/log recipient;
    // dropping them globally loses NPC child CastSuccess delivery to players.
    let redundant_self = supplied == Some(caster) && targets::excludes_caster(&spell);
    let explicit = if redundant_self {
        explicit_targets::normalize_unit_target(world, &mut spell, supplied)
    } else {
        supplied
    };
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
    } else if matches!(
        relation,
        targets::Relation::Ally | targets::Relation::Raid | targets::Relation::Minipet
    ) {
        None
    } else {
        victim(world, caster)
    };
    let defaults_to_self = !matches!(
        relation,
        targets::Relation::Enemy | targets::Relation::Minipet
    );
    selected.or_else(|| defaults_to_self.then_some(caster))
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
