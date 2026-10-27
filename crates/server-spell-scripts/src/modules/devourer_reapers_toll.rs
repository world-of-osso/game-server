//! SimC76f8118 sc_demon_hunter.cpp6697–6783: Reaper's Toll executes native damage.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, RemoveReason};
use combat::soul_fragments::{self, FragmentKind};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterNativeLaunch(after_launch)];

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !matches!(spell.id, 1239123 | 1245470)
        || world.get::<ActiveSpec>(caster).map(|spec| spec.0) != Some(1480)
    {
        return;
    }
    // SimC76f8118 cpp6742–6744 executes the native Fury child on the parent.
    if !spell_cast::try_cast_triggered(world, caster, 1239507, Some(caster), 1) {
        warn!(?caster, "native Reaper's Toll Fury 1239507 was refused");
    }
    let target = spell
        .cast_primary_target
        .expect("native Reaper's Toll target");
    if !spell_cast::try_cast_triggered(world, caster, 1239127, Some(target), 1) {
        warn!(
            ?caster,
            ?target,
            "native Reaper's Toll damage 1239127 was refused"
        );
        return;
    }
    // SimC76f8118 cpp6699–6717,6739–6742: the child spawns parent effect1's count.
    let count = spell_cast::scripts::script_effect_value(world, caster, spell.id, 0) as usize;
    for _ in 0..count {
        soul_fragments::spawn(world, caster, FragmentKind::Lesser, false)
            .expect("native Reaper's Toll fragment");
    }
    // SimC76f8118 cpp6747–6753 consumes Hungering Slash and grants Voidstep.
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.caster == caster && matches!(aura.spell_id, 1239525 | 1245523)
    });
    if !spell_cast::try_cast_triggered(world, caster, 1223157, Some(caster), 1) {
        warn!(?caster, "native Voidstep 1223157 was refused");
    }
}
