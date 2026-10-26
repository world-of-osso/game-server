//! Sweeping Strikes uses native family masks, secondary effectiveness and uses.
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

use super::combat;
use combat::auras::{ActiveAuras, spell_mods};
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterForegroundSpecial(grant_uses),
    SpellScriptHook::OnCast(clear_previous_hits),
    SpellScriptHook::ResolvedDamage(record_hit),
    SpellScriptHook::AfterCounterLaunch(consume_landed_use),
    SpellScriptHook::TargetContextDamageMultiplier(secondary_multiplier),
];

const SWEEPING_STRIKES: u32 = 260708;

#[derive(Component, Default)]
struct SweepingHits(BTreeMap<u32, BTreeSet<Entity>>);

fn grant_uses(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != SWEEPING_STRIKES {
        return;
    }
    // SimC76f8118 sc_warrior.cpp5291–5294: current-version effect4 is
    // the initial use count, not max_stacks or the default single application.
    let uses = script_effect_value(world, caster, SWEEPING_STRIKES, 3) as u8;
    assert!(uses > 0, "native Sweeping Strikes requires positive uses");
    assert!(combat::auras::set_self_cast_stacks(world, caster, SWEEPING_STRIKES, uses));
}

fn read_affected_instance(world: &World, caster: Entity, spell: &SpellInfo) -> Option<u32> {
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == SWEEPING_STRIKES && aura.caster == caster)?;
    let effect = aura.effects.iter().find(|effect| effect.index == 0)?;
    // sc_warrior.cpp1230: eligibility is the native first-effect family mask.
    spell_mods::class_mask_affects(aura, effect, spell).then_some(aura.instance_id)
}

fn secondary_multiplier(world: &World, ctx: &EffectCtx<'_>, target: Entity, _: bool) -> f32 {
    // sc_warrior.cpp1256–1273: only added recipients receive effect2 damage.
    if ctx.spell.cast_primary_target == Some(target)
        || read_affected_instance(world, ctx.caster, ctx.spell).is_none()
    {
        return 1.0;
    }
    script_effect_value(world, ctx.caster, SWEEPING_STRIKES, 1) / 100.0
}

fn clear_previous_hits(world: &mut World, caster: Entity, spell: u32, _: u8) {
    if let Some(mut hits) = world.get_mut::<SweepingHits>(caster) {
        hits.0.remove(&spell);
    }
}

fn record_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool, _: f32) {
    if read_affected_instance(world, ctx.caster, ctx.spell).is_none() {
        return;
    }
    world
        .entity_mut(ctx.caster)
        .entry::<SweepingHits>()
        .or_default()
        .into_mut()
        .0
        .entry(ctx.spell.id)
        .or_default()
        .insert(target);
}

fn consume_landed_use(world: &mut World, caster: Entity, action: u32) {
    let Some(recipients) = world
        .get_mut::<SweepingHits>(caster)
        .and_then(|mut hits| hits.0.remove(&action))
    else {
        return;
    };
    // sc_warrior.cpp1324–1325: one use per execution only after two hits;
    // an isolated primary or an entirely defended cast cannot spend a use.
    if recipients.len() < 2 {
        return;
    }
    let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == SWEEPING_STRIKES && aura.caster == caster)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        combat::auras::spend_stack(world, caster, instance);
    }
}
