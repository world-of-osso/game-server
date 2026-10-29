//! SimC76f8118 sc_monk.cpp6840–6853: Walk with the Ox direct-hit accumulator.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellStore;

const TALENT: u32 = 387219;
const STOMP: u32 = 1242373;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LandedDamagePolicy(on_damage),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
];

#[derive(Component, Default)]
struct WalkProgress {
    attempts: u32,
    ready_at: f64,
}

fn owns_proc(spell: u32) -> bool {
    spell == TALENT
}

fn on_damage(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: Option<u32>,
    amount: f32,
    periodic: bool,
) {
    if periodic
        || spell.is_none()
        || amount <= 0.0
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 268)
        || !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == TALENT)
        })
    {
        return;
    }
    let now = combat::spell_cast::now(world);
    if world.get::<WalkProgress>(caster).is_none() {
        world.entity_mut(caster).insert(WalkProgress::default());
    }
    if world.get::<WalkProgress>(caster).unwrap().ready_at > now {
        return;
    }
    let cooldown = f64::from(
        world
            .resource::<SpellStore>()
            .get(TALENT)
            .expect("native Walk with the Ox talent")
            .aura_options
            .as_ref()
            .expect("native Walk with the Ox proc options")
            .proc_category_recovery_ms,
    ) / 1000.0;
    let coefficient = 0.0075 * f64::from(script_effect_value(world, caster, TALENT, 0));
    let mut progress = world.get_mut::<WalkProgress>(caster).unwrap();
    progress.ready_at = now + cooldown;
    progress.attempts += 1;
    if combat::random::random::<f64>() >= coefficient * f64::from(progress.attempts) {
        return;
    }
    progress.attempts = 0;
    if !combat::spell_cast::try_cast_triggered(world, caster, STOMP, Some(target), 1) {
        warn!(?caster, ?target, "Walk with the Ox Stomp refused");
    }
}
