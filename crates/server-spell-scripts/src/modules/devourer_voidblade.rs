//! SimC76f8118 sc_demon_hunter.cpp5830–5856: Voidblade executes its native child.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterNativeLaunch(after_launch)];

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != 1245412 || world.get::<ActiveSpec>(caster).map(|spec| spec.0) != Some(1480) {
        return;
    }
    let target = spell.cast_primary_target.expect("native Voidblade target");
    if !spell_cast::try_cast_triggered(world, caster, 1245414, Some(target), 1) {
        warn!(
            ?caster,
            ?target,
            "native Voidblade child 1245414 was refused"
        );
    }
}
