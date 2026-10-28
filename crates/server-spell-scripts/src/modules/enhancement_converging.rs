//! SimC76f8118 sc_shaman.cpp5591–5604/6094–6100: Crash earns Strike stacks.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource, RemoveReason};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use shared::components::ActiveSpec;

const TALENT: u32 = 384363;
const BUFF: u32 = 198300;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResolvedHits(after_hits)];

fn after_hits(world: &mut World, ctx: &EffectCtx<'_>) {
    if !world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == 263)
    {
        return;
    }
    match ctx.spell.id {
        187874 => grant_from_crash(world, ctx),
        17364 | 115356 => consume_after_hands(world, ctx),
        _ => {}
    }
}

fn grant_from_crash(world: &mut World, ctx: &EffectCtx<'_>) {
    if !world
        .get::<KnownSpells>(ctx.caster)
        .is_some_and(|spells| spells.contains(TALENT))
    {
        return;
    }
    let hits = spell_cast::resolving_damage(world, ctx)
        .expect("Crash retains its resolved hits")
        .len();
    for _ in 0..hits {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster: ctx.caster,
                target: ctx.caster,
                spell_id: BUFF,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}

fn consume_after_hands(world: &mut World, ctx: &EffectCtx<'_>) {
    if !auras::has_aura(world, ctx.caster, BUFF) {
        return;
    }
    // Native64 hand children are queued; preserve their source buff until both
    // execute, then expire it at the same queued deadline as the last hand.
    let delay = ctx
        .spell
        .effects
        .iter()
        .filter(|effect| matches!(effect.trigger_spell, 32175 | 32176 | 115357 | 115360))
        .map(|effect| effect.misc_values[0].max(0) as u32)
        .max()
        .expect("Strike retains its native weapon children");
    spell_cast::queued_triggers::schedule_callback(world, ctx, ctx.caster, delay, remove_buff);
}

fn remove_buff(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == BUFF && aura.caster == caster
    });
}
