//! SimC76f8118 sc_druid.cpp11384–11396 and 13187–13191.
use bevy::prelude::*;

use super::combat;
use combat::auras::{self, ActiveAuras, AuraInstance, scripts::AuraScripts};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const TALENT: u32 = 203962;
const ENERGIZE: u32 = 203961;
const THRASH_BLEED: u32 = 192090;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EarlyAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(TALENT).or_default();
    script.on_apply = Some(randomize_phase);
    script.on_periodic = Some(earn_rage);
}

fn randomize_phase(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster == unit {
        auras::randomize_initial_periodic_phase(world, unit, aura.instance_id, 0);
    }
}

fn earn_rage(world: &mut World, unit: Entity, aura: &AuraInstance, effect: u8) {
    if aura.caster != unit || effect != 0 {
        return;
    }
    let cap = aura.effects.iter().find(|effect| effect.index == 0)
        .expect("Blood Frenzy target cap").amount.max(0.0) as usize;
    let mut query = world.query::<&ActiveAuras>();
    let targets = query.iter(world).filter(|auras| {
        auras.iter().any(|dot| dot.spell_id == THRASH_BLEED && dot.caster == unit)
    }).count().min(cap);
    for _ in 0..targets {
        spell_cast::cast_triggered(world, unit, ENERGIZE, Some(unit), 1);
    }
}
