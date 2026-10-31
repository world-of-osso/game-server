//! Strength of Spirit uses the caster's live health for Expel Harm.
use bevy::prelude::*;
use shared::components::Health;

use super::combat;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::NamedHealingMultiplier(healing_multiplier)];

fn healing_multiplier(world: &World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    if !matches!(spell.id, 322101 | 451968) {
        return 1.0;
    }
    let selected = world
        .get::<combat::class_progression::TraitLoadout>(caster)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|trait_spell| trait_spell.spell_id == 387276)
        });
    if !selected {
        return 1.0;
    }
    let health = world
        .get::<Health>(caster)
        .expect("Expel Harm caster retains health");
    let missing = 1.0 - (health.current / health.max).max(0.0);
    let percent =
        combat::spell_cast::scripts::script_effect_value(world, caster, 387276, 0) / 100.0;
    // SimC76f8118 monk59–70/4028–4033 supplies 1 + missing * percent as
    // the parsed direct-heal bonus; the parse layer adds the base multiplier.
    2.0 + missing * percent
}
