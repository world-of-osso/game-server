//! SimC76f8118 sc_rogue.cpp7489–7501/8554–8575/11182–11195:
//! Shadow Dance expiry earns a decaying, per-landed-hit Shadow residual.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::scripts::AuraScripts;
use combat::auras::{self, ActiveAuras, AuraInstance, AuraSource, RemoveReason};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{
    self,
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
};
use combat::spell_info::SpellStore;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::ResolvedDamage(after_damage),
];

const DANCE: u32 = 185313;
const TALENT: u32 = 382524;
const LINGERING: u32 = 385960;
const RESIDUAL: u32 = 386081;
const TIER: u32 = 1296593;
const SHADOW_SCHOOL: u32 = 32;

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(DANCE).or_default().on_remove = Some(earn_lingering);
    let script = scripts.0.entry(LINGERING).or_default();
    script.on_apply = Some(start_decay);
    script.on_periodic = Some(decay_tick);
}

fn selected(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 261)
        && has_talent(world, caster, TALENT)
}

fn earn_lingering(world: &mut World, unit: Entity, aura: &AuraInstance, _reason: RemoveReason) {
    if unit != aura.caster || !selected(world, unit) {
        return;
    }
    // The pinned expire_override cancels the old counter before granting a full one.
    auras::remove_auras(world, unit, RemoveReason::Consumed, |active| {
        active.spell_id == LINGERING && active.caster == unit
    });
    spell_cast::cast_triggered(world, unit, LINGERING, Some(unit), 1);
}

fn start_decay(world: &mut World, unit: Entity, aura: &AuraInstance) {
    // The tick-zero callback runs at current_tick=0: ceil(max)-1, not max.
    assert!(
        aura.max_stacks > 1,
        "native Lingering Shadow must have a stack cap"
    );
    auras::set_self_cast_stacks(world, unit, LINGERING, aura.max_stacks - 1);
}

fn decay_tick(world: &mut World, unit: Entity, aura: &AuraInstance, index: u8) {
    if index != 0 {
        return;
    }
    let period = f64::from(
        aura.effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("native Lingering Shadow periodic effect")
            .period_ms,
    ) / 1000.0;
    let duration = f64::from(aura.duration_ms) / 1000.0;
    let started = aura.expires_at.expect("native Lingering Shadow deadline") - duration;
    let tick = ((spell_cast::now(world) - started) / period + 1e-6).floor();
    let ticks = f64::from(script_effect_value(world, unit, TALENT, 2));
    assert!(
        period > 0.0 && ticks > 0.0,
        "native Lingering Shadow decay timing"
    );
    let remaining = (f64::from(aura.max_stacks) * (1.0 - tick / ticks)).ceil() - 1.0;
    if remaining <= 0.0 {
        auras::remove_instance(world, unit, aura.instance_id, RemoveReason::Expired);
    } else {
        auras::set_self_cast_stacks(world, unit, LINGERING, remaining as u8);
    }
}

fn read_owned_aura(world: &World, caster: Entity, spell: u32) -> Option<&AuraInstance> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == caster)
}

fn source_fraction(world: &World, caster: Entity, spell: u32) -> f32 {
    // sc_rogue.cpp3701/4639/5826/5851/10568–10580: only these builders/clones.
    if matches!(spell, 53 | 200758 | 197835 | 1269209 | 1269214 | 1269340) {
        return 1.0;
    }
    // sc_rogue.cpp4400–4425/5709–5771: finishers require the real MID2 four-piece.
    if !matches!(spell, 196819 | 328082 | 1269205 | 319175 | 319190 | 1269565) {
        return 0.0;
    }
    read_owned_aura(world, caster, TIER)
        .filter(|aura| matches!(aura.source, AuraSource::ItemSet(_)))
        .map_or(0.0, |aura| {
            aura.effects
                .iter()
                .find(|effect| effect.index == 0)
                .expect("native Subtlety four-piece residual fraction")
                .amount
                / 100.0
        })
}

fn shadow_multiplier(world: &World, ctx: &EffectCtx) -> f32 {
    // sc_rogue.cpp2083–2089: add only Shadow-school bonuses not already in the source.
    let school = ctx
        .spell
        .misc
        .as_ref()
        .expect("native source school")
        .school_mask;
    if school & SHADOW_SCHOOL != 0 {
        return 1.0;
    }
    [(382517, 0), (382017, 0), (382504, 1), (1268936, 1)]
        .into_iter()
        .filter(|&(talent, _)| has_talent(world, ctx.caster, talent))
        .map(|(talent, index)| 1.0 + script_effect_value(world, ctx.caster, talent, index) / 100.0)
        .product()
}

fn darkness_multiplier(world: &World, caster: Entity) -> f32 {
    // sc_rogue.cpp8569–8572: Lingering Darkness is scripted, absent from the whitelist.
    if !has_talent(world, caster, 457056) {
        return 1.0;
    }
    read_owned_aura(world, caster, 457273).map_or(1.0, |aura| {
        let amount = aura
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("native Lingering Darkness direct bonus")
            .amount;
        1.0 + amount * f32::from(aura.stacks.max(1)) / 100.0
    })
}

fn after_damage(world: &mut World, ctx: &EffectCtx, target: Entity, _critical: bool, damage: f32) {
    if damage <= 0.0 || !selected(world, ctx.caster) {
        return;
    }
    let Some(aura) = read_owned_aura(world, ctx.caster, LINGERING) else {
        return;
    };
    let fraction =
        source_fraction(world, ctx.caster, ctx.spell.id) * f32::from(aura.stacks) / 100.0;
    if fraction <= 0.0 {
        return;
    }
    // sc_rogue.cpp8567/8575 passes mitigated result_amount, including the original crit.
    let amount =
        damage * fraction * shadow_multiplier(world, ctx) * darkness_multiplier(world, ctx.caster);
    let spell = world
        .resource::<SpellStore>()
        .get(RESIDUAL)
        .expect("native Lingering Shadow payload")
        .clone();
    let residual = EffectCtx {
        caster: ctx.caster,
        spell: &spell,
        depth: ctx.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    // A resolved copy cannot roll another crit or recursively emit source callbacks.
    effects::deal_copied_damage(world, &residual, target, amount);
}
