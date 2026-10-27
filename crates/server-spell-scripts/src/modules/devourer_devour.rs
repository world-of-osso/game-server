//! Devour Fury and Consume/Devour fragment production.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::KnownSpells;
use combat::soul_fragments::{self, FragmentKind};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::OnCast(on_cast)];

fn on_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if spell == 473662 {
        spawn_fragments(world, caster);
        return;
    }
    if spell != 1217610 {
        return;
    }
    spawn_fragments(world, caster);
    // SimC76f8118 sc_demon_hunter.cpp2468–2474,5774–5779 executes
    // Devour's native Fury action on each cast, independent of hit outcome.
    if !spell_cast::try_cast_triggered(world, caster, 1288123, Some(caster), depth + 1) {
        warn!(?caster, "Devour native Fury payload 1288123 was refused");
    }
}

fn spawn_fragments(world: &mut World, caster: Entity) {
    if !world.get::<KnownSpells>(caster).is_some_and(|known| known.contains(1215617)) {
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
