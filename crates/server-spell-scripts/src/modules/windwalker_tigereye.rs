//! Tigereye Brew's native stacks and Zenith consumption.
use bevy::prelude::*;

use super::combat;
use combat::auras::{self, ActiveAuras, AuraInstance};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const TIGEREYE: u32 = 1261703;
const BREW: u32 = 1261724;
const ZENITH: u32 = 1249625;
const BLACKOUT_KICK: u32 = 100784;

#[derive(Component, Default)]
struct ChiAccumulator(f32);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::CombatStatusChanged(begin_combat),
    SpellScriptHook::OnCast(accumulate_chi),
    SpellScriptHook::AfterAuraApplication(apply_zenith),
];

fn begin_combat(world: &mut World, caster: Entity, active: bool) {
    if !active || !auras::has_aura(world, caster, TIGEREYE) || auras::has_aura(world, caster, BREW) {
        return;
    }
    // SimC76f8118 monk/sc_monk.cpp7170–7183 starts combat at the native initial stack count.
    let count = spell_cast::scripts::script_effect_value(world, caster, TIGEREYE, 0) as u8;
    if !spell_cast::cast_triggered_with_overrides(world, caster, BREW, Some(caster), 1,
        spell_cast::TriggeredSpellOverrides { aura_stacks: Some(count), ..Default::default() }) {
        warn!(?caster, "Tigereye initial stack application refused");
    }
}

fn accumulate_chi(world: &mut World, caster: Entity, spell_id: u32, depth: u8) {
    if depth != 0 || !auras::has_aura(world, caster, TIGEREYE) { return; }
    let Some(spell) = world.resource::<combat::spell_info::SpellStore>().get(spell_id) else { return; };
    let Some(cost) = spell.power_costs.iter().find(|cost| cost.power_type == 12 &&
        (cost.required_aura_spell_id == 0 || auras::has_aura(world, caster, cost.required_aura_spell_id))) else { return; };
    let base = if spell_id == BLACKOUT_KICK {
        spell_cast::applicable_costs(world, caster, spell).ok().and_then(|costs| costs.into_iter().find_map(|(pool, amount, _)| {
            (pool == spell_cast::CostPool::Power(shared::components::PowerType::Chi)).then_some(amount)
        })).expect("Blackout Kick retains native Chi cost")
    } else { cost.mana_cost as f32 };
    let threshold = spell_cast::scripts::script_effect_value(world, caster, TIGEREYE, 1);
    assert!(threshold > 0.0, "native Tigereye Chi threshold must be positive");
    let value = world.get::<ChiAccumulator>(caster).map_or(0.0, |state| state.0) + base;
    // SimC76f8118 monk/sc_monk.cpp393–406: one stack per native threshold and retain remainder.
    let earned = value >= threshold;
    world.entity_mut(caster).insert(ChiAccumulator(if earned { value - threshold } else { value }));
    if earned && !spell_cast::try_cast_triggered(world, caster, BREW, Some(caster), depth + 1) {
        warn!(?caster, "Tigereye stack application refused");
    }
}

fn apply_zenith(world: &mut World, target: Entity, aura: &AuraInstance) {
    if aura.spell_id != ZENITH || aura.caster != target || !auras::has_aura(world, target, TIGEREYE) { return; }
    let brew = world.get::<ActiveAuras>(target).and_then(|auras| auras.iter().find(|item| item.spell_id == BREW && item.caster == target)).map(|item| (item.instance_id, item.stacks));
    let Some((instance, stacks)) = brew else { return; };
    let cap = spell_cast::scripts::script_effect_value(world, target, TIGEREYE, 2) as u8;
    let used = stacks.min(cap);
    let bonus = spell_cast::scripts::script_effect_value(world, target, BREW, 0) * f32::from(used);
    let current = aura.effects.iter().find(|effect| effect.index == 0).expect("native Zenith crit effect0").amount;
    // SimC76f8118 monk/sc_monk.cpp4583–4595 consumes stacks into Zenith's critical chance.
    assert!(auras::set_effect_amount(world, target, aura.instance_id, 0, current + bonus));
    for _ in 0..used { auras::spend_stack(world, target, instance); }
}
