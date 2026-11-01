//! Blood Draw threshold and cooldown for Blood death knights.
use bevy::prelude::*;
use shared::components::{ActiveSpec, Health};
use shared::protocol::{CombatLogEvent, CombatLogKind};

use super::combat;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EffectEnabled(native_effect_enabled),
    SpellScriptHook::HealthChangeObserved(blood_draw_on_damage),
];

const TALENT: u32 = 374598;
const DAMAGE: u32 = 374606;
const COOLDOWN: u32 = 374609;
const REDUCTION: u32 = 454871;

// TC spell_dk.cpp spell_dk_blood_draw checks cooldown aura 374609 for both
// triggered spells. SimC sc_death_knight.cpp blood_draw_t checks the same
// duration and the native health threshold on each Blood damage event.
fn native_effect_enabled(world: &World, caster: Entity, spell: u32, index: u8) -> bool {
    !(spell == TALENT && matches!(index, 0 | 3) && is_blood(world, caster))
}

fn is_blood(world: &World, unit: Entity) -> bool {
    world
        .get::<ActiveSpec>(unit)
        .is_some_and(|spec| spec.0 == 250)
}

fn blood_draw_on_damage(world: &mut World, event: &CombatLogEvent) {
    if event.kind != CombatLogKind::Damage {
        return;
    }
    let Some(unit) = event.target.and_then(Entity::try_from_bits) else {
        return;
    };
    if !is_blood(world, unit)
        || !combat::auras::has_aura(world, unit, TALENT)
        || combat::auras::has_aura(world, unit, COOLDOWN)
    {
        return;
    }
    let threshold = world
        .resource::<SpellStore>()
        .get(TALENT)
        .expect("Blood Draw talent")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Blood Draw health threshold")
        .base_points;
    if !world
        .get::<Health>(unit)
        .is_some_and(|health| health.current <= health.max * threshold / 100.0)
    {
        return;
    }
    spell_cast::cast_triggered(world, unit, REDUCTION, Some(unit), 1);
    spell_cast::cast_triggered(world, unit, DAMAGE, Some(unit), 1);
}
