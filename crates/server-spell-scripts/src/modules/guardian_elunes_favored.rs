//! VERIFIED_WCL mHcDYRVyprndwjFC fight 2 pull 2: Elune's Favored heals once per second.
//! DB2 70448: 370586 effect 0 is 18% of Arcane damage; 370588 ticks every second.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

const TALENT: u32 = 370586;
const TIMER: u32 = 370588;
const HEAL: u32 = 370602;
const BEAR_FORM: u32 = 5487;
const ARCANE_SCHOOL: u32 = 64;

#[derive(Component, Default)]
struct AccumulatedArcane(f32);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::ResolvedDamage(accumulate_damage),
    SpellScriptHook::ConsumedEffectValues(&[(TALENT, 0)]),
];

fn register(scripts: &mut AuraScripts) {
    let talent = scripts.0.entry(TALENT).or_default();
    talent.on_apply = Some(start_timer);
    talent.on_remove = Some(stop_timer);
    scripts.0.entry(TIMER).or_default().on_periodic = Some(heal_accumulated_damage);
}

fn start_timer(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster == unit {
        spell_cast::cast_triggered(world, unit, TIMER, Some(unit), 1);
    }
}

fn stop_timer(world: &mut World, unit: Entity, aura: &AuraInstance, _: RemoveReason) {
    if aura.caster == unit {
        auras::remove_auras(world, unit, RemoveReason::Cancelled, |active| {
            active.caster == unit && active.spell_id == TIMER
        });
        world.entity_mut(unit).remove::<AccumulatedArcane>();
    }
}

fn accumulate_damage(world: &mut World, ctx: &EffectCtx<'_>, _: Entity, _: bool, damage: f32) {
    if damage <= 0.0
        || ctx.spell.school_mask() & ARCANE_SCHOOL == 0
        || !auras::has_aura(world, ctx.caster, TALENT)
        || !auras::has_aura(world, ctx.caster, BEAR_FORM)
    {
        return;
    }
    if let Some(mut pending) = world.get_mut::<AccumulatedArcane>(ctx.caster) {
        pending.0 += damage;
    } else {
        world
            .entity_mut(ctx.caster)
            .insert(AccumulatedArcane(damage));
    }
}

fn heal_accumulated_damage(world: &mut World, unit: Entity, aura: &AuraInstance, effect: u8) {
    if aura.caster != unit || effect != 0 {
        return;
    }
    let Some(mut pending) = world.get_mut::<AccumulatedArcane>(unit) else {
        return;
    };
    let damage = std::mem::take(&mut pending.0);
    if damage <= 0.0 {
        return;
    }
    let percent = combat::spell_cast::scripts::script_effect_value(world, unit, TALENT, 0);
    spell_cast::cast_triggered_with_effect0_amount(
        world,
        unit,
        HEAL,
        Some(unit),
        1,
        damage * percent / 100.0,
    );
}
