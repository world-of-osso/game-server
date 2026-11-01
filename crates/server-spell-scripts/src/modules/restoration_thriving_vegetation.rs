//! 70448 Thriving Vegetation 447131: Rejuvenation instantly heals its target for
//! s1% of its total periodic effect with 447132 (20 at rank 1, 30 at rank 2 via
//! TraitDefinitionEffectPoints). SimC 76f8118 marks it NYI.
//! WCL kD8MqXLV6CFYHN2p fight 3 actor 108: 717 noncritical 447132 heals from 742
//! Rejuvenation/Germination casts.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    self, AuraInstance,
    spell_mods::{self, SpellModOp},
};
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
};
use combat::spell_info::SpellStore;

const TALENT: u32 = 447131;
const HEAL: u32 = 447132;
const REJUVENATION: u32 = 774;
const GERMINATION: u32 = 155777;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterAuraApplication(heal_on_rejuvenation)];

fn heal_on_rejuvenation(world: &mut World, target: Entity, aura: &AuraInstance) {
    if !matches!(aura.spell_id, REJUVENATION | GERMINATION)
        || !auras::has_aura(world, aura.caster, TALENT)
    {
        return;
    }
    let Some(total) = total_periodic_effect(world, aura) else {
        return;
    };
    let caster = aura.caster;
    let percent = combat::spell_cast::scripts::script_effect_value(world, caster, TALENT, 0);
    let spell = world
        .resource::<SpellStore>()
        .get(HEAL)
        .expect("Thriving Vegetation heal missing")
        .clone();
    let ctx = EffectCtx {
        caster,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    // The HoT's stored amount already carries the caster's healing-done bonuses.
    effects::apply_fixed_heal(world, &ctx, target, total * percent / 100.0);
}

/// The applied HoT's remaining ticks, including an owed initial tick, times its
/// tick amount after the periodic healing modifiers its own ticks receive.
fn total_periodic_effect(world: &World, aura: &AuraInstance) -> Option<f32> {
    let effect = aura
        .effects
        .iter()
        .find(|effect| effect.aura_type == 8 && effect.period_ms > 0)?;
    let remaining = aura.expires_at? - auras::aura_now(world);
    let initial = f64::from(u8::from(effect.initial_tick_pending()));
    let ticks = (initial + remaining * 1000.0 / f64::from(effect.period_ms)) as f32;
    let hot = world.resource::<SpellStore>().get(aura.spell_id)?;
    let tick = spell_mods::modify(
        world,
        aura.caster,
        hot,
        SpellModOp::PeriodicHealingAndDamage,
        effect.amount,
    );
    Some(tick * ticks)
}
