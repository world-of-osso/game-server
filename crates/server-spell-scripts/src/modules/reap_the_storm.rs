//! Slayer Reap executions retain native damage, chance and internal cooldown.
use bevy::prelude::*;

use super::combat;
use combat::auras::{ActiveAuras, procs};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::targets;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::BeforeNativeOnCast(snapshot_cleave_targets),
    SpellScriptHook::AfterSpenderProc(after_cast),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
];

const REAP_TALENT: u32 = 444775;
const REAP_DAMAGE: u32 = 446005;
const IMMINENT_DEMISE: u32 = 444769;
const SUDDEN_DEATH: u32 = 52437;

fn owns_proc(spell: u32) -> bool {
    spell == REAP_TALENT
}

#[derive(Component)]
struct CleaveRecipients(usize);

fn snapshot_cleave_targets(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if spell.id != 845 || depth != 0 || read_reap_instance(world, caster).is_none() {
        return;
    }
    // sc_warrior.cpp3714–3720 checks execution n_targets, not post-hit survivors.
    // Reuse native Cleave selection before its damage can kill any recipient.
    let effect = spell.effects.iter().find(|effect| effect.index == 0).expect("native Cleave damage effect");
    let cap = spell.target_restrictions.as_ref().map_or(0, |limits| limits.max_targets);
    let recipients = targets::select_targets(world, caster, spell.cast_primary_target, spell, effect, cap, false)
        .expect("admitted native Cleave has supported target selection");
    world.entity_mut(caster).insert(CleaveRecipients(recipients.len()));
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo, target: Option<Entity>) {
    let chance = if spell.id == 845 {
        take_cleave_chance(world, caster)
    } else if matches!(spell.id, 163201 | 281000 | 280735) {
        read_execute_chance(world, caster)
    } else {
        None
    };
    let Some(chance) = chance else {
        return;
    };
    let Some(instance) = read_reap_instance(world, caster) else {
        return;
    };
    if procs::try_policy_proc(world, caster, instance, chance)
        && !combat::spell_cast::try_cast_triggered(world, caster, REAP_DAMAGE, target, 1)
    {
        warn!(?caster, "Native Reap the Storm payload refused");
    }
}

fn take_cleave_chance(world: &mut World, caster: Entity) -> Option<f32> {
    let recipients = world.entity_mut(caster).take::<CleaveRecipients>()?.0;
    let minimum = script_effect_value(world, caster, REAP_TALENT, 1) as usize;
    // sc_warrior.cpp3716: one roll per Cleave, native effect3 chance.
    (recipients >= minimum).then(|| script_effect_value(world, caster, REAP_TALENT, 2))
}

fn read_execute_chance(world: &World, caster: Entity) -> Option<f32> {
    // SimC76f8118 sc_warrior.cpp4347–4359/4547–4559: only an SD Execute
    // may roll selected Imminent Demise's Reap. This stage precedes the existing
    // SD consumer; the shared native Reap aura owns its500ms ICD (cpp7980–7981).
    if !combat::auras::has_aura(world, caster, SUDDEN_DEATH)
        || !combat::auras::has_aura(world, caster, IMMINENT_DEMISE)
    {
        return None;
    }
    Some(script_effect_value(world, caster, IMMINENT_DEMISE, 1))
}

fn read_reap_instance(world: &World, caster: Entity) -> Option<u32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == REAP_TALENT && aura.caster == caster)
        .map(|aura| aura.instance_id)
}
