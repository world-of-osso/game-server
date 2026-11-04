//! Mirror Image amplifies selected Refractive Images.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterNativeOnCast(after_cast)];

fn after_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if spell == 55342
        && world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == 1309497)
        })
    {
        // VERIFIED_WCL dtmp7Lg2abzyZVBG fight 3: both auras apply and expire together.
        combat::spell_cast::cast_triggered(world, caster, 1309793, Some(caster), depth + 1);
    }
}
