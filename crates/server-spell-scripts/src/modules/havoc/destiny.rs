//! SimC76f8118 sc_demon_hunter.cpp2529–2555: paid base Fury extends live Meta.
use super::{combat, havoc::has_talent};
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;
use shared::components::PowerType;

const TALENT: u32 = 388116;
const META: u32 = 162264;
// Native Fury-cost actions in the literal Havoc action set. Background/full-mask
// copies retain no payment context and cannot manufacture a spender extension.
const SPENDERS: &[u32] = &[162794, 201427, 188499, 210152, 198013, 452497, 185123];
pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResourceOnlyContext(SPENDERS),
    SpellScriptHook::AfterForegroundSpecial(extend_from_paid_fury),
];

#[derive(Component, Default)]
struct FuryRemainder(f32);

fn extend_from_paid_fury(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !has_talent(world, caster, TALENT) || !paid_fury(spell) || !live_meta(world, caster) {
        return;
    }
    let base = spell
        .power_costs
        .iter()
        .filter(|cost| {
            cost.power_type == PowerType::Fury as i32 && cost.required_aura_spell_id == 0
        })
        .map(|cost| cost.mana_cost as f32)
        .sum::<f32>();
    let threshold = combat::spell_cast::scripts::script_effect_value(world, caster, TALENT, 1);
    let milliseconds = combat::spell_cast::scripts::script_effect_value(world, caster, TALENT, 0);
    assert!(
        base > 0.0 && threshold > 0.0 && milliseconds > 0.0,
        "native Shattered Destiny values must be positive"
    );
    let count = accumulate_base_fury(world, caster, base, threshold);
    if count > 0.0 {
        combat::auras::extend_owned_duration(
            world,
            caster,
            caster,
            &[META],
            (count * milliseconds) as u32,
        );
    }
}

fn paid_fury(spell: &SpellInfo) -> bool {
    spell.cast_policy_context.as_ref().is_some_and(|context| {
        !context.free_spender
            && context
                .resources
                .iter()
                .any(|(power, paid)| *power == PowerType::Fury && *paid > 0.0)
    })
}

fn live_meta(world: &World, caster: Entity) -> bool {
    let now = combat::auras::aura_now(world);
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == META
                && aura.caster == caster
                && aura.expires_at.is_some_and(|end| end > now)
        })
    })
}

fn accumulate_base_fury(world: &mut World, caster: Entity, base: f32, threshold: f32) -> f32 {
    let mut entity = world.entity_mut(caster);
    let mut remainder = entity.entry::<FuryRemainder>().or_default().into_mut();
    let total = remainder.0 + base;
    let count = (total / threshold).floor();
    remainder.0 = total - count * threshold;
    count
}
