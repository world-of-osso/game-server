//! SimC76f8118 shadow2206–2214/2628–2632/2850–2858 and pets75–86/1061–1134.
//! Earned cohorts, not individual missiles, fund the native Thing From Beyond.
use super::{combat, shadow_apparitions, shadow_priest};
use bevy::prelude::*;
use combat::auras::scripts::AuraScripts;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use combat::spell_cast::summon::SummonedCreature;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use shared::components::Npc;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::DotMasteryMultiplier(spike_mastery),
    SpellScriptHook::TargetContextDamageMultiplier(spike_scale),
    SpellScriptHook::ConsumedEffectValues(&[(SPIKE, 1), (SPIKE, 2)]),
];
const TALENT: u32 = 373273;
const COUNTER: u32 = 373276;
const SUMMON: u32 = 373277;
const THING: u32 = 189988;
const SPIKE: u32 = 373279;
const APEX: u32 = 1264096;

fn register(scripts: &mut AuraScripts) {
    // SimC2628–2632 supplies the only producer, not a generic Aura42 damage proc.
    scripts.0.entry(TALENT).or_default().check_proc = Some(|_, _, _| false);
}

fn counter_is_full(world: &World, caster: Entity) -> bool {
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            let own_counter = aura.caster == caster && aura.spell_id == COUNTER;
            own_counter && aura.stacks >= aura.max_stacks.max(1)
        })
    })
}

pub(super) fn earn_cohort(world: &mut World, ctx: &EffectCtx<'_>) {
    if !auras::has_aura(world, ctx.caster, TALENT) || counter_is_full(world, ctx.caster) {
        return;
    }
    increment_counter(world, ctx.caster);
    if !counter_is_full(world, ctx.caster) {
        return;
    }
    spell_cast::cast_triggered(world, ctx.caster, SUMMON, Some(ctx.caster), ctx.depth + 1);
    if auras::has_aura(world, ctx.caster, APEX) {
        // SimC2850–2858 defaults target=nullptr for the counter's extra cohort.
        // Keep the capped counter until this nested cohort returns: it cannot
        // increment beyond max before SimC's queued cancellation (2209–2212).
        shadow_apparitions::spawn_parentless_apparitions(world, ctx);
    }
    auras::remove_auras(world, ctx.caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == COUNTER && aura.caster == ctx.caster
    });
}

fn increment_counter(world: &mut World, caster: Entity) {
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: COUNTER,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn read_thing_owner(world: &World, caster: Entity, spell: u32) -> Option<Entity> {
    if spell != SPIKE {
        return None;
    }
    let summon = world.get::<SummonedCreature>(caster)?;
    let npc = world.get::<Npc>(caster)?;
    (summon.spell_id == SUMMON && npc.template_id == THING).then_some(summon.owner)
}

fn spike_mastery(world: &World, caster: Entity, target: Entity, spell: u32) -> f32 {
    let Some(owner) = read_thing_owner(world, caster, spell) else {
        return 1.0;
    };
    // pets1103: use the actual owner's live target DoTs, never pet-owned DoTs.
    shadow_priest::weaving_multiplier(world, owner, target, spell)
}

fn spike_scale(world: &World, ctx: &EffectCtx<'_>, target: Entity, _: bool) -> f32 {
    if read_thing_owner(world, ctx.caster, ctx.spell.id).is_none() {
        return 1.0;
    }
    let native = |index| spell_cast::scripts::script_effect_value(world, ctx.caster, SPIKE, index);
    // pets1100–1103/1118–1122: native effect2 supplies off-target share,
    // effect3 the reduced-AoE cap. The literal old30% comment is not the value.
    let share = if ctx.primary_target() == Some(target) {
        1.0
    } else {
        native(1) / 100.0
    };
    let count = spell_cast::resolving_targets(world, ctx)
        .expect("native Spike recipient selection")
        .len() as f32;
    let cap = native(2);
    share * (cap / count).min(1.0).sqrt()
}
