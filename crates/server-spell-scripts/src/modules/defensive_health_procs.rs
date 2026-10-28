//! Health-triggered racial and Death Knight runeforge defenses.
use bevy::prelude::*;
use shared::components::Health;

use super::combat;
use combat::auras::{AuraInstance, procs::AuraEvent, scripts::AuraScripts};
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    for spell in [312916, 326801] {
        scripts.0.entry(spell).or_default().check_proc = Some(below_health_without_cooldown);
    }
}

// SimC76f8118 spelltext_data.inc 13277/13911: below the native effect's
// threshold and not while the triggered spell's cooldown aura is active.
fn below_health_without_cooldown(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    let Some(target) = event.target.filter(|target| *target == aura.caster) else {
        return false;
    };
    let Some(health) = world.get::<Health>(target) else {
        return false;
    };
    let threshold = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("health-triggered defense threshold")
        .amount;
    let cooldown = match aura.spell_id {
        312916 => 313015,
        326801 => 326809,
        _ => unreachable!(),
    };
    health.current < health.max * threshold / 100.0
        && !combat::auras::has_aura(world, target, cooldown)
}
