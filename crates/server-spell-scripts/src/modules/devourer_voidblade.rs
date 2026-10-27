//! SimC76f8118 sc_demon_hunter.cpp5830–5856,5874–5889: Voidblade and Pierce execute their native child.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::AfterNativeLaunch(after_launch),
];

fn register(scripts: &mut combat::auras::scripts::AuraScripts) {
    scripts.0.insert(
        1239525,
        combat::auras::scripts::AuraScript {
            on_remove: Some(clear_reapers_replacement),
            ..Default::default()
        },
    );
}

pub(super) fn clear_reapers_replacement(
    world: &mut World,
    caster: Entity,
    _: &combat::auras::AuraInstance,
    _: combat::auras::RemoveReason,
) {
    combat::auras::remove_auras(
        world,
        caster,
        combat::auras::RemoveReason::Consumed,
        |aura| aura.caster == caster && aura.spell_id == 1245523,
    );
}

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if world.get::<ActiveSpec>(caster).map(|spec| spec.0) != Some(1480) {
        return;
    }
    // SimC76f8118 cpp3203–3219,5502,5830: Hunt and Voidblade/Pierce grant Hungering Slash.
    if matches!(spell.id, 1245412 | 1245483 | 1246167)
        && world
            .get::<combat::class_progression::KnownSpells>(caster)
            .is_some_and(|known| known.contains(1239519))
    {
        grant_hungering_slash(world, caster);
    }
    if !matches!(spell.id, 1245412 | 1245483) {
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

fn grant_hungering_slash(world: &mut World, caster: Entity) {
    if !spell_cast::try_cast_triggered(world, caster, 1239525, Some(caster), 1) {
        error!(?caster, "native Hungering Slash aura was refused");
    }
    grant_reapers_replacement(world, caster);
}

pub(super) fn grant_reapers_replacement(world: &mut World, caster: Entity) {
    if combat::auras::has_aura(world, caster, 1217607)
        && combat::auras::has_aura(world, caster, 1239525)
        && world
            .get::<combat::class_progression::KnownSpells>(caster)
            .is_some_and(|known| known.contains(452402))
        && !spell_cast::try_cast_triggered(world, caster, 1245523, Some(caster), 1)
    {
        error!(?caster, "native Reaper's Toll replacement was refused");
    }
}
