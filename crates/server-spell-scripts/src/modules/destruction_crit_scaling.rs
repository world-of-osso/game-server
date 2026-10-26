//! Conflagration of Chaos damage uses live caster spell crit, not its guaranteed outcome.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraStatMods;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::script_registry::{DamageAmountHook, SpellScriptHook};
use combat::stats::CharacterStats;
use shared::components::ActiveSpec;

const CONFLAGRATION_OF_CHAOS: u32 = 387108;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::DamageAmount(damage_lookup)];

fn damage_lookup(spell: u32) -> Option<DamageAmountHook> {
    matches!(spell, 17962 | 17877).then_some(scale_selected_damage)
}

fn scale_selected_damage(world: &World, caster: Entity, _target: Entity, amount: f32) -> f32 {
    let destruction = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 267);
    let selected = world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == CONFLAGRATION_OF_CHAOS)
    });
    if !destruction || !selected {
        return amount;
    }
    // SimC76f8118 actions4511–4517/4838–4844: (1+cache.spell_crit_chance).
    // Exclude the ability-specific guaranteed crit modifier. This existing
    // damage boundary also serves pure predictions and skips resolved copies.
    // Chaos Bolt already has its native batch5 relationship; do not apply twice.
    let base = world
        .get::<CharacterStats>(caster)
        .expect("selected Destruction caster requires native character stats")
        .crit_pct;
    let spell_bonus = world
        .get::<AuraStatMods>(caster)
        .map_or(0.0, |mods| mods.spell_crit_pct);
    amount * (1.0 + (base + spell_bonus) / 100.0)
}
