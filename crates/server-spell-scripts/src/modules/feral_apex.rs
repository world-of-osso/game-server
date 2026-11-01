//! Feral Apex Predator's Craving is earned only by Rip ticks.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraInstance, procs::AuraEvent, scripts::AuraScripts};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const TALENT: u32 = 391881;
const BUFF: u32 = 391882;
const RIP: u32 = 1079;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    let talent = scripts.0.entry(TALENT).or_default();
    assert!(talent.check_proc.is_none(), "Apex Predator's Craving proc already scripted");
    talent.check_proc = Some(block_generic_periodic_proc);
    let rip = scripts.0.entry(RIP).or_default();
    assert!(rip.after_periodic_damage.is_none(), "Rip periodic damage already scripted");
    rip.after_periodic_damage = Some(roll_apex_on_rip_tick);
}

// The native ProcChance is 100 on every harmful periodic event; the Rip tick owns the roll.
fn block_generic_periodic_proc(_world: &World, _aura: &AuraInstance, _event: &AuraEvent) -> bool {
    false
}

fn read_talent_value(world: &World, caster: Entity, index: u8) -> Option<f32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == TALENT && aura.caster == caster)?
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .map(|effect| effect.amount)
}

fn count_owned_rips(world: &mut World, caster: Entity) -> usize {
    let mut units = world.query::<&ActiveAuras>();
    units
        .iter(world)
        .filter(|auras| auras.iter().any(|aura| aura.spell_id == RIP && aura.caster == caster))
        .count()
}

/// SimC76f8118 sc_druid.cpp4655–4656/4727–4733: effect1 tenths of a percent,
/// divided by active Rips raised to effect2's percent exponent.
fn apex_chance(base: f32, exponent: f32, rips: usize) -> f64 {
    let base = f64::from(base) / 10.0 / 100.0;
    base / (rips.max(1) as f64).powf(f64::from(exponent) / 100.0)
}

fn roll_apex_on_rip_tick(world: &mut World, _target: Entity, rip: &AuraInstance, _index: u8) {
    let caster = rip.caster;
    let (Some(base), Some(exponent)) = (
        read_talent_value(world, caster, 0),
        read_talent_value(world, caster, 1),
    ) else {
        return;
    };
    let rips = count_owned_rips(world, caster);
    if combat::random::random::<f64>() < apex_chance(base, exponent, rips) {
        spell_cast::cast_triggered(world, caster, BUFF, Some(caster), 1);
    }
}

#[cfg(test)]
mod tests {
    use super::apex_chance;

    #[test]
    fn apex_chance_uses_native_tenths_and_rip_exponent() {
        assert!((apex_chance(40.0, 15.0, 1) - 0.04).abs() < 1e-12);
        assert!((apex_chance(40.0, 15.0, 4) - 0.04 / 4f64.powf(0.15)).abs() < 1e-12);
    }
}
