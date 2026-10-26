//! SimC76f8118 rogue6103–6125/7419–7444: Vanish stealth and weapon suspension.
use super::combat;
use bevy::prelude::*;
use combat::auras::{AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::combat::{self as attacks, AutoAttack};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use std::time::Duration;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::BeforeNativeOnCast(before_cast),
    SpellScriptHook::AfterNativeOnCast(after_cast),
    SpellScriptHook::LateAuraRegistration(register),
];
const VANISH: u32 = 1856;
const VANISH_BUFF: u32 = 11327;
const STEALTH: u32 = 1784;
const MELEE_COMBAT_START: u32 = 0x200;

#[derive(Component)]
struct SuspendedWeapons {
    attack: AutoAttack,
    paused_at: f64,
}

fn before_cast(world: &mut World, caster: Entity, spell: &SpellInfo, _depth: u8) {
    if spell.id == VANISH {
        save_weapons(world, caster);
    } else if spell
        .misc
        .as_ref()
        .is_some_and(|misc| misc.attributes[1] & MELEE_COMBAT_START != 0)
    {
        restore_weapons(world, caster, spell.cast_primary_target);
    }
}

fn save_weapons(world: &mut World, caster: Entity) {
    if let Some(attack) = world.get::<AutoAttack>(caster).cloned() {
        let paused_at = spell_cast::now(world);
        world
            .entity_mut(caster)
            .insert(SuspendedWeapons { attack, paused_at });
    }
    // Native1856 already has CANCELS_AUTO_ATTACK_COMBAT: ordinary Spell::finish
    // removes AutoAttack and publishes AttackStopped after the cast completes.
}

fn restore_weapons(world: &mut World, caster: Entity, target: Option<Entity>) {
    let Some(target) = target else { return };
    if !spell_cast::targets::relation_matches(
        world,
        caster,
        target,
        spell_cast::targets::Relation::Enemy,
    ) {
        return;
    }
    let Some(mut saved) = world.entity_mut(caster).take::<SuspendedWeapons>() else {
        return;
    };
    // An explicit native attack request already owns its new victim/timers.
    if world.get::<AutoAttack>(caster).is_some() {
        return;
    }
    let elapsed = Duration::from_secs_f64((spell_cast::now(world) - saved.paused_at).max(0.0));
    saved.attack.victim = target;
    saved.attack.timer = saved.attack.timer.saturating_sub(elapsed);
    saved.attack.off_hand_timer = saved.attack.off_hand_timer.saturating_sub(elapsed);
    attacks::start_attack_now(world, caster, saved.attack);
}

fn after_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if spell == VANISH {
        spell_cast::cast_triggered(world, caster, VANISH_BUFF, Some(caster), depth + 1);
    }
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(VANISH_BUFF).or_default().on_remove = Some(expire_to_stealth);
}

fn expire_to_stealth(world: &mut World, unit: Entity, aura: &AuraInstance, reason: RemoveReason) {
    if unit == aura.caster && reason == RemoveReason::Expired {
        // Apply before the generic last-stealth check: natural expiry does not
        // start the post-stealth window while actual Stealth remains active.
        spell_cast::cast_triggered(world, unit, STEALTH, Some(unit), 1);
    }
}
