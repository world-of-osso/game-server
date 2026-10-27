//! Actual Infernal execution earns native Crashing Chaos uses.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, script_effect_value},
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::ActiveSpec;

const INFERNAL: u32 = 1122;
const TALENT: u32 = 417234;
const BUFF: u32 = 417282;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::NativeAfterCast(after_cast)];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == INFERNAL).then_some(earn_uses)
}

fn earn_uses(world: &mut World, caster: Entity, _: &SpellInfo) {
    if world
        .get::<ActiveSpec>(caster)
        .is_none_or(|spec| spec.0 != 267)
        || !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == TALENT)
        })
    {
        return;
    }
    // SimC76f8118 actions5080–5082, init839–841: trigger the native
    // reverse-use buff at the selected talent's native effect2 count.
    let uses = script_effect_value(world, caster, TALENT, 2) as u8;
    let mut buff = world
        .resource::<SpellStore>()
        .get(BUFF)
        .expect("native Crashing Chaos buff")
        .clone();
    buff.cast_aura_stacks = Some(uses);
    if !combat::spell_cast::launch_triggered_spell(world, caster, &buff, Some(caster), 1) {
        warn!(?caster, "earned Crashing Chaos buff refused");
    }
}
