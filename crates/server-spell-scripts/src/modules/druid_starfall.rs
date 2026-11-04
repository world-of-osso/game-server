//! SimC76f8118 sc_druid.cpp8260–8370/8467–8468: periodic Starfall impacts.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::random::Rng;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    queued_triggers::{self, ScaledTrigger},
    script_registry::{ContextHitHook, SpellScriptHook},
};
use combat::spell_info::SpellInfo;

const STARFALL: u32 = 191034;
const DRIVER: u32 = 50286;
const DAMAGE: u32 = 191037;
const ASCENDANT_STARS: u32 = 1263382;
const ASCENDANT_STARFALL: u32 = 1263401;
const STAR_TRAVEL_STEP_MS: u32 = 66;
const STAR_TRAVEL_STEPS: u32 = 14;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AsynchronousAuras(&[STARFALL, ASCENDANT_STARFALL]),
    SpellScriptHook::SuccessfulCastRelationships(after_starfall),
    SpellScriptHook::ContextHit(driver_hit),
];

fn after_starfall(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != STARFALL {
        return;
    }
    // 1263382's native damage mask covers Starsurge, not the Starfall driver.
    // SimC sc_druid.cpp8467–8468 explicitly spends a use for this separate buff.
    let source = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == ASCENDANT_STARS && aura.caster == caster)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = source {
        auras::spend_stack(world, caster, instance);
        spell_cast::cast_triggered(world, caster, ASCENDANT_STARFALL, Some(caster), 1);
    }
}

fn driver_hit(spell: u32) -> Option<ContextHitHook> {
    (spell == DRIVER).then_some(queue_star)
}

fn damage_multiplier(world: &World, caster: Entity) -> f32 {
    let auras = world
        .get::<ActiveAuras>(caster)
        .expect("Starfall periodic driver retains its caster auras");
    let stacks = auras
        .iter()
        .find(|aura| aura.spell_id == STARFALL && aura.caster == caster)
        .map_or(1.0, |aura| f32::from(aura.stacks).max(1.0));
    let ascendant = auras
        .iter()
        .find(|aura| aura.spell_id == ASCENDANT_STARFALL && aura.caster == caster)
        .map_or(0.0, |aura| {
            let effect = aura
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .expect("native Ascendant Stars damage percentage");
            effect.amount * f32::from(aura.stacks) / 100.0
        });
    stacks + ascendant
}

fn queue_star(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    // SimC snapshots the live stack budget before its discrete star travel delay.
    let multiplier = damage_multiplier(world, ctx.caster);
    let delay_ms =
        combat::random::thread_rng().gen_range(1..=STAR_TRAVEL_STEPS) * STAR_TRAVEL_STEP_MS;
    queued_triggers::schedule_scaled_full_trigger(
        world,
        ctx,
        ScaledTrigger {
            target,
            spell_id: DAMAGE,
            delay_ms,
            multiplier,
        },
    );
}
