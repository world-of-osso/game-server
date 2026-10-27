//! SimC76f8118 sc_demon_hunter.cpp3339–3348,8567–8581: native Fury buff producers.
use super::{combat, havoc::has_talent};
use bevy::prelude::*;
use combat::auras::AuraInstance;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterResourceValues(after_cast),
    SpellScriptHook::AfterCounterLaunch(after_launch),
    SpellScriptHook::OwnsPolicyProc(|spell| spell == 197125),
    SpellScriptHook::ActualCriticalChanceBonus(chaos_theory_critical_bonus),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    match spell {
        198013 | 452497 => Some(grant_student),
        188499 | 210152 => Some(grant_chaos_theory),
        _ => None,
    }
}

pub(super) fn apply_blind_fury(world: &mut World, caster: Entity, aura: &AuraInstance) {
    if aura.caster != caster || !has_talent(world, caster, 203550) {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp9986–9993 uses talent effect3/50
    // every100ms: exactly its native raw amount per5s. Eye/Gaze already
    // carries the real aura85 Fury-regeneration effect; populate that amount.
    let amount = combat::spell_cast::scripts::script_effect_value(world, caster, 203550, 2);
    let regen = aura
        .effects
        .iter()
        .find(|effect| {
            effect.aura_type == combat::auras::handlers::MOD_POWER_REGEN
                && effect.misc_values[0] == 17
        })
        .expect("native beam Fury regeneration effect");
    assert!(
        combat::auras::set_effect_amount(world, caster, aura.instance_id, regen.index, amount),
        "applied native beam instance"
    );
}

fn grant_student(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if has_talent(world, caster, 452412) {
        combat::spell_cast::cast_triggered(world, caster, 453239, Some(caster), 1);
    }
}

// SimC76f8118 sc_demon_hunter.cpp7234–7237 grants the native buff on dance.
fn grant_chaos_theory(world: &mut World, caster: Entity, _: &SpellInfo) {
    if has_talent(world, caster, 389687)
        && !combat::spell_cast::try_cast_triggered(world, caster, 390195, Some(caster), 1)
    {
        error!("Native Chaos Theory buff cast refused");
    }
}

fn after_launch(world: &mut World, caster: Entity, spell: u32) {
    // Native198813 is a queued triggered child, so it never runs the ordinary
    // foreground AfterCast stage. This callback follows its actual launch.
    if spell == 198813 && has_talent(world, caster, 389688) {
        combat::spell_cast::cast_triggered(world, caster, 389890, Some(caster), 1);
    }
    if matches!(spell, 199547 | 201428) {
        roll_chaos_strike_refund(world, caster);
    }
}

// SimC76f8118 sc_demon_hunter.cpp2098–2103,2413–2420 samples the talent's
// native effect1–effect2 range separately for each affected action.
fn chaos_theory_critical_bonus(
    world: &World,
    caster: Entity,
    _target: Entity,
    spell: &SpellInfo,
) -> f32 {
    const TALENT: u32 = 389687;
    const BUFF: u32 = 390195;
    if !has_talent(world, caster, TALENT) {
        return 0.0;
    }
    let affected = world
        .get::<combat::auras::ActiveAuras>(caster)
        .is_some_and(|auras| {
            auras.iter().any(|aura| {
                aura.spell_id == BUFF
                    && aura.caster == caster
                    && aura.effects.iter().any(|effect| {
                        effect.index == 0
                            && combat::auras::spell_mods::class_mask_affects(aura, effect, spell)
                    })
            })
        });
    if !affected {
        return 0.0;
    }
    let lower = combat::spell_cast::scripts::script_effect_value(world, caster, TALENT, 0);
    let upper = combat::spell_cast::scripts::script_effect_value(world, caster, TALENT, 1);
    lower + combat::random::random::<f32>() * (upper - lower)
}

fn roll_chaos_strike_refund(world: &mut World, caster: Entity) {
    // SimC76f8118 sc_demon_hunter.cpp7472–7510,7673–7685: the real
    // Chaos Strike/Annihilation off-hand execution rolls the native refund.
    // Own197125's event so broad metadata cannot spend its ICD without a payload.
    // The native helper takes the live aura instance, not its spell ID.
    let instance = super::havoc::owned_passive_instance(world, caster, 197125);
    if let Some(instance) = instance
        && combat::auras::procs::try_native_policy_proc(world, caster, instance)
    {
        combat::spell_cast::cast_triggered(world, caster, 193840, Some(caster), 1);
    }
    // SimC7513–7515 consumes the buff on the off-hand execution, even if
    // the native refund roll fails or its ICD is still active.
    if has_talent(world, caster, 389687) {
        combat::auras::remove_auras(
            world,
            caster,
            combat::auras::RemoveReason::Consumed,
            |aura| aura.caster == caster && aura.spell_id == 390195,
        );
    }
}
