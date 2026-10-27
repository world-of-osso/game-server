//! Devour Fury and Consume/Devour fragment production.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::KnownSpells;
use combat::soul_fragments::{self, FragmentKind};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OnCast(on_cast),
    SpellScriptHook::AfterNativeLaunch(consume_soulburst),
];

fn on_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if spell == 473662 {
        spawn_fragments(world, caster);
        return;
    }
    if spell != 1217610 {
        return;
    }
    spawn_fragments(world, caster);
    // SimC76f8118 cpp5772–5794 reduces Reap's cooldown by Meta effect14.
    let reduction = spell_cast::scripts::script_effect_value(world, caster, 1217607, 13) / 1000.0;
    spell_cast::scripts::reduce_cooldown(world, caster, 1226019, f64::from(reduction));
    // SimC76f8118 sc_demon_hunter.cpp2468–2474,5774–5779 executes
    // Devour's native Fury action on each cast, independent of hit outcome.
    if !spell_cast::try_cast_triggered(world, caster, 1288123, Some(caster), depth + 1) {
        warn!(?caster, "Devour native Fury payload 1288123 was refused");
    }
}

fn consume_soulburst(world: &mut World, caster: Entity, spell: &combat::spell_info::SpellInfo) {
    if !matches!(spell.id, 473662 | 1217610) || !combat::auras::has_aura(world, caster, 1297433) {
        return;
    }
    let target = spell.cast_primary_target.expect("Soulburst cast retains target");
    // SimC76f8118 cpp5666–5686,5753–5765: Soulburst deals damage then expires.
    if !spell_cast::try_cast_triggered(world, caster, 1297432, Some(target), 1) {
        error!(?caster, "native Soulburst damage was refused");
        return;
    }
    combat::auras::remove_auras(world, caster, combat::auras::RemoveReason::Consumed, |aura| aura.caster == caster && aura.spell_id == 1297433);
    if combat::auras::has_aura(world, caster, 1296616) {
        if !spell_cast::try_cast_triggered(world, caster, 1238495, Some(caster), 1) {
            error!(?caster, "Soulburst Moment of Craving was refused");
        }
        spell_cast::scripts::reduce_cooldown(world, caster, 1226019, f64::MAX);
        let count = spell_cast::scripts::script_effect_value(world, caster, 1296616, 0).max(0.0) as usize;
        for _ in 0..count {
            soul_fragments::spawn(world, caster, FragmentKind::Lesser, false).expect("native Soulburst fragment");
        }
    }
}

fn spawn_fragments(world: &mut World, caster: Entity) {
    if !world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(1215617))
    {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp5753–5762: Predator's Thirst spawns
    // Shattered Souls effect3's native count when Consume or Devour executes.
    let count = spell_cast::scripts::script_effect_value(world, caster, 1227619, 2) as usize;
    for _ in 0..count {
        soul_fragments::spawn(world, caster, FragmentKind::Lesser, false)
            .expect("native Consume/Devour fragment");
    }
}
