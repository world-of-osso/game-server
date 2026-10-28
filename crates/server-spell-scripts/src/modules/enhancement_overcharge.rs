//! SimC76f8118 sc_shaman.cpp13798–13820: Overcharge uses live owner crit.
use super::combat;
use bevy::prelude::*;
use combat::auras::stat_mods::AuraStatMods;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;
use combat::stats::CharacterStats;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::ActualCriticalDamageMultiplier(multiplier)];

fn multiplier(world: &World, caster: Entity, spell: &SpellInfo) -> f32 {
    if spell.school_mask() & 8 == 0
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 263)
        || !combat::auras::has_aura(world, caster, 1251026)
    {
        return 1.0;
    }
    let weapon_action = spell
        .categories
        .as_ref()
        .is_some_and(|category| category.defense_type == 2)
        || matches!(spell.id, 469270 | 1252431);
    let spell_bonus = if weapon_action {
        0.0
    } else {
        world
            .get::<AuraStatMods>(caster)
            .map_or(0.0, |stats| stats.spell_crit_pct)
    };
    let crit = world
        .get::<CharacterStats>(caster)
        .expect("Overcharge owner requires crit stats")
        .crit_pct
        + spell_bonus;
    let percent = world
        .resource::<combat::spell_info::SpellStore>()
        .get(1251026)
        .expect("selected Overcharge exists")
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("Overcharge effect2 exists")
        .base_points;
    1.0 + percent / 100.0 * crit / 100.0 * 0.5
}
