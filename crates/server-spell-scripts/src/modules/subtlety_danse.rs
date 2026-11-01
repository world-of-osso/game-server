//! SimC76f8118 Danse Macabre distinct foreground abilities per Shadow Dance.
use std::collections::HashSet;

use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

const DANCE: u32 = 185313;
const DANSE_MACABRE: u32 = 382528;
const LASHE_MACABRE: u32 = 1264397;
const COMBO_POINTS: i32 = 4;

#[derive(Component, Default)]
struct DanseAbilities(HashSet<u32>);

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterForegroundSpecial(after_cast)];

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id == DANCE {
        world.entity_mut(caster).remove::<DanseAbilities>();
        return;
    }
    if !auras::has_aura(world, caster, DANSE_MACABRE)
        || !auras::has_aura(world, caster, DANCE)
        || !uses_combo_points(spell)
    {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    let inserted = world
        .entity_mut(caster)
        .entry::<DanseAbilities>()
        .or_default()
        .get_mut()
        .0
        .insert(spell.id);
    if inserted {
        spell_cast::cast_triggered(world, caster, LASHE_MACABRE, Some(target), 1);
    }
}

fn uses_combo_points(spell: &SpellInfo) -> bool {
    let triggers_gcd = spell
        .cooldowns
        .as_ref()
        .is_some_and(|cooldown| cooldown.start_recovery_ms > 0);
    let spends = spell.power_costs.iter().any(|cost| {
        cost.power_type == COMBO_POINTS && (cost.mana_cost > 0 || cost.optional_cost > 0)
    });
    let generates = spell.effects.iter().any(|effect| {
        effect.effect == 30 && effect.misc_values[0] == COMBO_POINTS && effect.base_points > 0.0
    });
    triggers_gcd && (spends || generates)
}
