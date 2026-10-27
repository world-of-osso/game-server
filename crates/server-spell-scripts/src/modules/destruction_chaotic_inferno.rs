//! Actual Chaos Bolt execution earns native Chaotic Inferno.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::TraitLoadout;
use combat::random as rand;
use combat::spell_cast::{
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, script_effect_value},
};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

const CHAOS_BOLT: u32 = 116858;
const TALENT: u32 = 1244788;
const BUFF: u32 = 1244860;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::NativeAfterCast(after_cast)];

#[derive(Component, Default)]
struct InfernoAttempts(u32);

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == CHAOS_BOLT).then_some(roll_inferno)
}

fn roll_inferno(world: &mut World, caster: Entity, _: &SpellInfo) {
    if world
        .get::<ActiveSpec>(caster)
        .is_none_or(|spec| spec.0 != 267)
        || !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == TALENT)
        })
        || !advance_prd(world, caster)
    {
        return;
    }
    // SimC76f8118 actions4391–4398 delays the buff 10ms so an Incinerate
    // queued in that window misses it; native casts start after the GCD.
    if !combat::spell_cast::try_cast_triggered(world, caster, BUFF, Some(caster), 1) {
        warn!(?caster, "earned Chaotic Inferno buff refused");
    }
}

fn advance_prd(world: &mut World, caster: Entity) -> bool {
    // SimC76f8118 init1331–1334: accumulated PRD from native effect1 chance.
    let chance = f64::from(script_effect_value(world, caster, TALENT, 1)) / 100.0;
    let constant = rand::accumulated_proc_constant(chance);
    if world.get::<InfernoAttempts>(caster).is_none() {
        world.entity_mut(caster).insert(InfernoAttempts::default());
    }
    let mut attempts = world.get_mut::<InfernoAttempts>(caster).unwrap();
    attempts.0 = attempts.0.saturating_add(1);
    let success = rand::random::<f64>() < constant * f64::from(attempts.0);
    if success {
        attempts.0 = 0;
    }
    success
}
