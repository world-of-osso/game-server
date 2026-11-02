//! Merektha's Thunder Spit target chain, WCL M4cdBNJkTjavYgQV/11 pull4:
//! marker 1289109 expires at 28.497s, 1289588 begins at that instant and
//! 1298329 hits at application and once per second through 32.507s.
use super::combat;
use bevy::prelude::*;
use combat::auras::{AuraInstance, scripts::AuraScripts};
use combat::spell_cast::script_registry::SpellScriptHook;

const MARK: u32 = 1289109;
const STRIKES: u32 = 1289588;
const DAMAGE: u32 = 1298329;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(MARK).or_default().on_natural_end = Some(begin_strikes);
    let strikes = scripts.0.entry(STRIKES).or_default();
    // Native 1289588 requests its first periodic tick on application.
    strikes.on_periodic = Some(periodic_strike);
}

fn begin_strikes(world: &mut World, target: Entity, aura: &AuraInstance) {
    combat::spell_cast::cast_triggered(world, aura.caster, STRIKES, Some(target), 1);
}

fn initial_strike(world: &mut World, target: Entity, aura: &AuraInstance) {
    combat::spell_cast::cast_triggered(world, aura.caster, DAMAGE, Some(target), 1);
}

fn periodic_strike(world: &mut World, target: Entity, aura: &AuraInstance, effect: u8) {
    if effect == 0 { initial_strike(world, target, aura); }
}
