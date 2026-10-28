//! Preservation empower releases from native spell descriptions and child data.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, CastFailure, script_registry::{EmpowerCompleteHook, SpellScriptHook}};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::Health;

const DREAM_BREATH: u32 = 355936;
const DREAM_HEAL: u32 = 355941;
const SPIRITBLOOM: u32 = 367226;
const SPIRIT_HEAL: u32 = 367230;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EmpowerCompletion(completion)];

fn completion(spell: u32) -> Option<EmpowerCompleteHook> {
    match spell {
        DREAM_BREATH => Some(release_dream),
        SPIRITBLOOM => Some(release_spirit),
        _ => None,
    }
}

fn release_rank(spell: &SpellInfo) -> Result<u8, CastFailure> {
    spell.cast_empower_level.filter(|rank| (1..=4).contains(rank))
        .ok_or_else(|| CastFailure::Unsupported(format!("preservation empower{} lacks a valid rank", spell.id)))
}

fn release_dream(world: &mut World, caster: Entity, spell: &SpellInfo, _target: Option<Entity>) -> Result<(), CastFailure> {
    let rank = release_rank(spell)?;
    let mut heal = world.resource::<SpellStore>().get(DREAM_HEAL).expect("native Dream Breath child").clone();
    let hot = heal.effects.iter().find(|effect| effect.index == 0).expect("Dream Breath periodic effect").clone();
    let converted_ticks = 2 * u32::from(rank - 1);
    let direct = heal.effects.iter_mut().find(|effect| effect.index == 2).expect("Dream Breath direct effect");
    // Native description-variable719: ranks convert two 2-second HoT ticks into direct healing.
    direct.base_points += hot.base_points * converted_ticks as f32;
    direct.bonus_coefficient += hot.bonus_coefficient * converted_ticks as f32;
    direct.bonus_coefficient_from_ap += hot.bonus_coefficient_from_ap * converted_ticks as f32;
    let duration_ms = (8 - converted_ticks) * hot.aura_period_ms;
    let duration = heal.misc.as_mut().and_then(|misc| misc.duration.as_mut()).expect("Dream Breath native duration");
    duration.duration_ms = duration_ms as i32;
    duration.max_duration_ms = duration.duration_ms;
    let allies = spell_cast::scripts::script_effect_value(world, caster, DREAM_BREATH, 0).max(0.0) as u32;
    heal.target_restrictions.as_mut().expect("Dream Breath native cone").max_targets = allies + 1;
    heal.cast_empower_level = Some(rank);
    spell_cast::execute_cast(world, caster, &heal, Some(caster))
}

fn release_spirit(world: &mut World, caster: Entity, spell: &SpellInfo, target: Option<Entity>) -> Result<(), CastFailure> {
    let rank = release_rank(spell)?;
    let primary = target.ok_or_else(|| CastFailure::Unsupported("Spiritbloom lacks a friendly target".into()))?;
    let mut heal = world.resource::<SpellStore>().get(SPIRIT_HEAL).expect("native Spiritbloom child").clone();
    heal.cast_empower_level = Some(rank);
    let targets = spirit_targets(world, caster, primary, rank);
    for target in targets {
        spell_cast::execute_cast(world, caster, &heal, Some(target))?;
    }
    Ok(())
}

fn spirit_targets(world: &mut World, caster: Entity, primary: Entity, rank: u8) -> Vec<Entity> {
    // Native description-variable703: each jump searches within 30 yards.
    const JUMP_RADIUS: f32 = 30.0;
    let mut selected = vec![primary];
    while selected.len() < usize::from(rank) {
        let source = *selected.last().expect("Spiritbloom primary exists");
        let Some(position) = spell_cast::targets::position(world, source) else { break; };
        let candidates = spell_cast::targets::area_units(world, caster, position, JUMP_RADIUS, spell_cast::targets::Relation::Ally, 0);
        let next = candidates.into_iter().filter(|unit| !selected.contains(unit))
            .filter_map(|unit| { let health = world.get::<Health>(unit)?; (health.current < health.max).then_some((unit, health.max - health.current)) })
            .max_by(|a, b| a.1.total_cmp(&b.1)).map(|(unit, _)| unit);
        let Some(next) = next else { break; };
        selected.push(next);
    }
    selected
}
