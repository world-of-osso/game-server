//! Arms-only selected relationship hooks; each behavior lives in its own file.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::spell_cast::script_registry::SpellScriptHook;

#[path = "arms_bloodletting.rs"]
mod bloodletting;
#[path = "arms_heroic_might.rs"]
mod heroic_might;
#[path = "arms_warfare_binding.rs"]
mod warfare;
#[path = "arms_just_warming_up.rs"]
mod warming;
#[path = "arms_winding_up.rs"]
mod winding;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(warfare::register),
    SpellScriptHook::AfterElementalLaunch(warming::after_smash),
    SpellScriptHook::AfterElementalLaunch(heroic_might::after_heroic),
    SpellScriptHook::BeforeNativeOnCast(heroic_might::before_smash),
    SpellScriptHook::OwnsPolicyProc(winding::owns_proc),
    SpellScriptHook::AfterElementalLaunch(winding::after_cast),
    SpellScriptHook::AfterElementalLaunch(consume_executioners_precision),
    SpellScriptHook::AfterPolicyDirectDamage(bloodletting::after_mortal),
];

fn consume_executioners_precision(
    world: &mut World,
    caster: Entity,
    spell: &combat::spell_info::SpellInfo,
    _: bool,
) {
    if spell.id != 12294
        || world.get::<ActiveSpec>(caster).is_none_or(|spec| spec.0 != 71)
    {
        return;
    }
    // SimC76f8118 sc_warrior.cpp3102: Mortal Strike snapshots Precision,
    // then expires it, including when the strike is a background action.
    combat::auras::remove_auras(world, caster, combat::auras::RemoveReason::Consumed, |aura| {
        aura.spell_id == 386633 && aura.caster == caster
    });
}

fn selected(world: &World, caster: Entity, passive: u32) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 71)
        && combat::auras::has_aura(world, caster, passive)
}
