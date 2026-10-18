//! Named contracts authored in local Mainline12.1.0.69933 descriptions/rows.
use bevy::prelude::*;
use shared::components::{Health, PowerType};

use super::script_effect_value;
use crate::auras::{AuraApplication, AuraInstance, AuraSource, RemoveReason};
use crate::spell_cast::{self, CostPool, effects, targets};
use crate::spell_info::SpellInfo;

pub(super) const MORTAL_STRIKE: u32 = 12294;
const MORTAL_WOUNDS: u32 = 115804;
pub(super) const REND: u32 = 772;
const REND_BLEED: u32 = 388539;
const EXECUTE: u32 = 163201;
const EXECUTE_RANK: u32 = 316405;
pub(super) const SHADOW_WORD_DEATH: u32 = 32379;
const DEATHSPEAKER: u32 = 364675;
pub(crate) const RAPID_FIRE: u32 = 257044;
const RAPID_FIRE_SHOT: u32 = 257045;
const RAPID_FIRE_FOCUS: u32 = 263585;
pub(super) const BESTIAL_WRATH: u32 = 19574;
const BESTIAL_STRIKE: u32 = 344572;
pub(super) const ASPECT_WILD: u32 = 193530;
const COBRA_SHOT: u32 = 193455;
pub(crate) const HAUNT: u32 = 48181;

///12294 explicitly names115804's healing reduction and lifetime.
pub(super) fn apply_mortal_wounds(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    apply_owned_aura(world, caster, target, MORTAL_WOUNDS, 1);
}

///772 explicitly names388539, independent of talent/weapon proc guesses.
pub(super) fn apply_rend_bleed(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    apply_owned_aura(world, caster, target, REND_BLEED, 1);
}

fn apply_owned_aura(world: &mut World, caster: Entity, target: Entity, spell_id: u32, mask: u32) {
    crate::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask: mask,
            source: AuraSource::Cast,
        },
    );
}

pub(super) fn cast_effect(spell: u32, index: u8) -> Option<effects::CastEffectHandler> {
    ((spell, index) == (EXECUTE, 0)).then_some(cast_execute_damage)
}

/// TC spell_warr_execute::HandleExecuteCast prevents default effect64 and casts
/// the authored damage child directly (spell_warrior.cpp:779–801).
fn cast_execute_damage(
    world: &mut World,
    ctx: &effects::EffectCtx,
    effect: &crate::spell_info::SpellEffect,
    target: Entity,
) {
    spell_cast::cast_queued_trigger_with_item_level(
        world,
        ctx.caster,
        effect.trigger_spell,
        Some(target),
        ctx.depth + 1,
        ctx.spell.cast_item_level,
    );
}

///163201 and its260798 damage child both have native Speed0. Refund after that
///successful launch resolves, using paid raw Rage including OptionalCost.
pub(in crate::spell_cast) fn refund_execute(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    costs: &[(CostPool, f32)],
) {
    if spell.id != EXECUTE || !crate::auras::has_aura(world, caster, EXECUTE_RANK) {
        return;
    }
    if !explicit.is_some_and(|target| targets::is_alive(world, target)) {
        return;
    }
    let spent = spell_cast::resources::spent(costs, PowerType::Rage);
    let percent = script_effect_value(world, caster, EXECUTE, 1);
    let amount = spent * percent / 100.0;
    let ctx = effects::EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::energize_unit(world, &ctx, caster, PowerType::Rage, amount, false);
}

///The tooltip identifies s2 as damage, not the simultaneous legacy s1 effect.
///Keep the unused legacy field uncertified; don't add both authored variants.
pub(in crate::spell_cast) fn effect_enabled(spell: u32, index: u8) -> bool {
    spell != SHADOW_WORD_DEATH || index != 0
}

///32379s3/s4 define the strict health threshold and bonus.364675s1/s2
///extend those exact values, only while the real Deathspeaker aura is active.
pub(super) fn death_execute(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    let rank = crate::auras::has_aura(world, caster, DEATHSPEAKER);
    let mut threshold = script_effect_value(world, caster, SHADOW_WORD_DEATH, 2);
    let mut percent = script_effect_value(world, caster, SHADOW_WORD_DEATH, 3);
    if rank {
        threshold += script_effect_value(world, caster, DEATHSPEAKER, 0);
        percent += script_effect_value(world, caster, DEATHSPEAKER, 1);
    }
    let health = world
        .get::<Health>(target)
        .expect("Death hit target health");
    if health.current < health.max * threshold / 100.0 {
        amount * (1.0 + percent / 100.0)
    } else {
        amount
    }
}

///257044 explicitly names257045 and263585. Its333ms period,2000ms duration
///and EXTRA_INITIAL_PERIOD attribute author seven shots; no synthetic timer.
pub(crate) fn cast_rapid_fire_tick(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    index: u8,
) {
    if index != 1 {
        return;
    }
    spell_cast::cast_triggered(world, aura.caster, RAPID_FIRE_SHOT, Some(unit), 1);
    spell_cast::cast_triggered(world, aura.caster, RAPID_FIRE_FOCUS, Some(aura.caster), 1);
}

///19574 explicitly names344572 at the persistent pet's own target.
///Its owner a108 does not establish a pet a79 binding; buff/CC/charges remain open.
pub(super) fn strike_with_owned_pet(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    let Some(pet) = crate::hunter_pet::summoned_pet(world, caster) else {
        return;
    };
    let target = world
        .get::<crate::networking::CombatTarget>(pet)
        .and_then(|target| target.0);
    if let Some(target) = target.filter(|target| targets::is_alive(world, *target)) {
        spell_cast::cast_triggered(world, pet, BESTIAL_STRIKE, Some(target), 1);
    }
}

///193530 explicitly fires a free Cobra Shot at the primary on activation.
///Extra nearby targets require an authored radius; don't invent geometry.
pub(super) fn cast_wild_primary(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    spell_cast::cast_triggered(world, caster, COBRA_SHOT, Some(target), depth + 1);
}

///48181's reset is on the owned mark's death removal, not expiry/cancellation.
pub(crate) fn reset_haunt_on_death(
    world: &mut World,
    _unit: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    if reason != RemoveReason::Death {
        return;
    }
    let removed = world
        .get_mut::<spell_cast::SpellHistory>(aura.caster)
        .and_then(|mut history| history.spell_ready_at.remove(&HAUNT));
    if removed.is_some() {
        super::send_reduced_cooldown(world, aura.caster, HAUNT, 0);
    }
}
