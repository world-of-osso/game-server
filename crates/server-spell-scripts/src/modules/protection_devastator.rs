//! Devastator's earned autoattack proc can reset Shield Slam.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraInstance;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        236279,
        AuraScript {
            on_proc: Some(reset_shield_slam),
            ..Default::default()
        },
    );
}

fn reset_shield_slam(world: &mut World, caster: Entity, _: &AuraInstance, _: Option<Entity>) {
    // SimC 76f8118 sc_warrior.cpp 1909–1958: the landed autoattack
    // produces Devastator; its effect 2 rolls a Shield Slam reset.
    let chance = spell_cast::scripts::script_effect_value(world, caster, 236279, 1);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    let now = spell_cast::history_now(world);
    let spell = world
        .resource::<combat::spell_info::SpellStore>()
        .get(23922)
        .expect("Shield Slam exists");
    let remaining = world
        .get::<spell_cast::SpellHistory>(caster)
        .map_or(0.0, |history| {
            history.remaining_cooldown_ms(spell, now) as f64 / 1000.0
        });
    spell_cast::scripts::reduce_cooldown(world, caster, 23922, remaining);
}
