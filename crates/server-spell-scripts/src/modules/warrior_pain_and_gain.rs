//! Retail 382551 heals about 2% max health before healing modifiers.
//! Two Midnight Warrior logs show 12,007 / 515,794 and 14,048 / 603,478;
//! the native effect's 20 points otherwise produce ten times that amount.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

const TALENT: u32 = 382549;
const HEAL: u32 = 382551;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NamedHealingMultiplier(healing_multiplier),
    SpellScriptHook::OwnsPolicyProc(|spell| spell == TALENT),
    SpellScriptHook::AuraEvent(heal_incoming_damage),
];

fn heal_incoming_damage(world: &mut World, event: &combat::auras::procs::AuraEvent) {
    if !event.can_proc || event.damage <= 0.0 {
        return;
    }
    let Some(target) = event.target else { return };
    let instance = world
        .get::<combat::auras::ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == TALENT && aura.caster == target)
                .map(|aura| aura.instance_id)
        });
    let Some(instance) = instance else { return };
    // 70448 says any damage. The older TDB row incorrectly restricts spell-family to Warrior.
    // Reuse the native chance and 10-second ICD, but do not spend it on a zero-damage hit.
    if combat::auras::procs::try_native_policy_proc(world, target, instance) {
        assert!(
            combat::spell_cast::try_cast_triggered(world, target, HEAL, Some(target), 1),
            "Pain and Gain heal refused for {target:?}"
        );
    }
}

fn healing_multiplier(_: &World, _: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    if spell.id == 382551 { 0.1 } else { 1.0 }
}
