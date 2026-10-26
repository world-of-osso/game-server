//! Native Shard Instability producers and the admitted instant-UA consumer.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    effects::EffectCtx, script_registry::SpellScriptHook, scripts::AfterCastHook,
};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

const TALENT: u32 = 1260264;
const BUFF: u32 = 1260269;
const UNSTABLE_AFFLICTION: u32 = 1259790;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterDirectDamageBeforeItems(direct_hit),
    SpellScriptHook::PeriodicPrimaryDamage(periodic_hit),
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::CastTimeProcs(&[BUFF]),
];

#[derive(Component)]
struct Attempts {
    passive: u32,
    shadow_bolt: u32,
    drain: u32,
}

fn direct_hit(world: &mut World, ctx: &EffectCtx<'_>) {
    // SimC76f8118 actions887–890/init1244–1245: landed Shadow Bolt20% PRD.
    if ctx.spell.id == 686 {
        roll_instability(world, ctx.caster, 1, false);
    }
}

fn periodic_hit(world: &mut World, ctx: &EffectCtx<'_>, _: Entity, _: f32, _: bool) {
    // SimC76f8118 actions2192–2195/2347–2350/init1242–1243: actual Grasp
    // and Drain Soul ticks share their10% PRD stream, independent of Shadow Bolt.
    // This primary-tick boundary precedes Cunning and Grasp extra-DoT emission.
    if matches!(ctx.spell.id, 1261153 | 198590) {
        roll_instability(world, ctx.caster, 0, true);
    }
}

fn roll_instability(world: &mut World, caster: Entity, effect: u8, drain: bool) {
    if world
        .get::<ActiveSpec>(caster)
        .is_none_or(|spec| spec.0 != 265)
    {
        return;
    }
    let Some(passive) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.caster == caster && aura.spell_id == TALENT)
            .map(|aura| aura.instance_id)
    }) else {
        return;
    };
    let chance = combat::spell_cast::scripts::script_effect_value(world, caster, TALENT, effect);
    let constant = combat::random::accumulated_proc_constant(f64::from(chance) / 100.0);
    if advance_attempt(world, caster, passive, drain, constant) {
        combat::auras::apply_aura(
            world,
            &combat::auras::AuraApplication {
                caster,
                target: caster,
                spell_id: BUFF,
                effect_mask: u32::MAX,
                source: combat::auras::AuraSource::Cast,
            },
        );
    }
}

fn advance_attempt(
    world: &mut World,
    caster: Entity,
    passive: u32,
    drain: bool,
    constant: f64,
) -> bool {
    if world
        .get::<Attempts>(caster)
        .is_none_or(|state| state.passive != passive)
    {
        world.entity_mut(caster).insert(Attempts {
            passive,
            shadow_bolt: 0,
            drain: 0,
        });
    }
    let mut state = world.get_mut::<Attempts>(caster).unwrap();
    let attempts = if drain {
        &mut state.drain
    } else {
        &mut state.shadow_bolt
    };
    *attempts = attempts.saturating_add(1);
    let success = combat::random::random::<f64>() < constant * f64::from(*attempts);
    if success {
        *attempts = 0;
    }
    success
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == UNSTABLE_AFFLICTION).then_some(consume_admitted_use)
}

fn consume_admitted_use(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC76f8118 actions1527–1534: only UA admitted instant with the earned
    // effect consumes it. A proc acquired during a hardcast is not consumed.
    let admitted = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.caster == caster
                && aura.spell_id == BUFF
                && spell.cast_proc_mods.contains(&aura.instance_id)
        })
    });
    if admitted {
        super::cast_relationships::decrement(world, caster, BUFF);
    }
}
