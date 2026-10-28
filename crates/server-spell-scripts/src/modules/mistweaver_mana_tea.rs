//! Mana Tea: native-description-derived spend accumulation and channel stacks.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource, ActiveAuras};
use combat::spell_cast::{self, CostPool, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::PowerType;

const TALENT: u32 = 115869;
const STACKS: u32 = 115867;
const CHANNEL: u32 = 115294;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterSpellPayment(earn_stacks),
    SpellScriptHook::AfterPeriodicEnergize(consume_stack),
];

#[derive(Component, Default)]
struct ManaSpent(f32);

fn earn_stacks(world: &mut World, caster: Entity, _spell: &SpellInfo, costs: &[(CostPool, f32)]) {
    if !auras::has_aura(world, caster, TALENT) { return; }
    let spent = spell_cast::spent(costs, PowerType::Mana);
    if spent <= 0.0 { return; }
    let talent = world.resource::<SpellStore>().get(TALENT).expect("Mana Tea talent").clone();
    let cost = spell_cast::applicable_costs(world, caster, &talent).expect("Mana Tea native cost")
        .into_iter().filter(|(pool, _, _)| *pool == CostPool::Power(PowerType::Mana))
        .map(|(_, amount, _)| amount).sum::<f32>();
    let multiplier = spell_cast::scripts::script_effect_value(world, caster, TALENT, 0);
    let threshold = cost * multiplier;
    assert!(threshold > 0.0, "Mana Tea native spending threshold must be positive");
    let total = world.get::<ManaSpent>(caster).map_or(0.0, |progress| progress.0) + spent;
    // Native-description-derived: retain the fractional spend toward the next stack.
    let earned = (total / threshold).floor() as u32;
    world.entity_mut(caster).insert(ManaSpent(total - earned as f32 * threshold));
    let base = world.get::<combat::stats::CharacterStats>(caster).expect("Mana Tea caster stats").crit_pct;
    let bonus = world.get::<combat::auras::AuraStatMods>(caster).map_or(0.0, |mods| mods.spell_crit_pct);
    let chance = (base + bonus).clamp(0.0, 100.0);
    for _ in 0..earned {
        apply_stack(world, caster);
        if combat::random::random::<f32>() * 100.0 < chance { apply_stack(world, caster); }
    }
}

fn apply_stack(world: &mut World, caster: Entity) {
    auras::apply_aura(world, &AuraApplication { caster, target: caster, spell_id: STACKS, effect_mask: 7, source: AuraSource::Cast });
}

fn consume_stack(world: &mut World, unit: Entity, spell: u32, effect: u8) {
    if spell != CHANNEL || effect != 0 { return; }
    let stack = world.get::<ActiveAuras>(unit).and_then(|auras| auras.iter().find(|aura| aura.spell_id == STACKS && aura.caster == unit).map(|aura| aura.instance_id));
    let Some(stack) = stack else {
        spell_cast::cancel_cast(world, unit);
        return;
    };
    auras::spend_stack(world, unit, stack);
    if !auras::has_aura(world, unit, STACKS) { spell_cast::cancel_cast(world, unit); }
}
