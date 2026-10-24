//! Taking-spell ownership: TC Spell.cpp3679–3694 saves/clears/restores nested casts.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;

use crate::spell_info::{AppliedSpellModifiers, SpellInfo};

type RecordingSlots = Arc<Mutex<HashMap<Entity, AppliedSpellModifiers>>>;

/// Scoped metadata stays off actor archetypes and therefore cannot reorder target queries.
#[derive(Resource, Default)]
struct TakingSpellModifiers(RecordingSlots);

pub(crate) struct RecordingScope {
    slots: RecordingSlots,
    caster: Entity,
    previous: Option<AppliedSpellModifiers>,
}

impl Drop for RecordingScope {
    fn drop(&mut self) {
        let mut slots = self.slots.lock().expect("taking spellmod slots poisoned");
        match self.previous.take() {
            Some(previous) => {
                slots.insert(self.caster, previous);
            }
            None => {
                slots.remove(&self.caster);
            }
        }
    }
}

/// Continuations retain their receipt; a template begins a new one.
pub(crate) fn clone_for_recording(spell: &SpellInfo) -> SpellInfo {
    let mut prepared = spell.clone();
    prepared
        .cast_applied_modifiers
        .get_or_insert_with(Default::default);
    prepared
}

/// A nested triggered invocation is a new spell, even if its input was a cast clone.
pub(crate) fn clone_for_new_recording(spell: &SpellInfo) -> SpellInfo {
    let mut prepared = spell.clone();
    prepared.cast_applied_modifiers = Some(Default::default());
    prepared
}

pub(crate) fn enter_recording(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
) -> Option<RecordingScope> {
    let applied = spell.cast_applied_modifiers.clone()?;
    let slots = Arc::clone(&world.get_resource_or_init::<TakingSpellModifiers>().0);
    let previous = slots
        .lock()
        .expect("taking spellmod slots poisoned")
        .insert(caster, applied);
    Some(RecordingScope {
        slots,
        caster,
        previous,
    })
}

/// Pure readiness queries outside an actual invocation have no taking-spell slot.
pub(super) fn read_active(world: &World, caster: Entity) -> Option<AppliedSpellModifiers> {
    world
        .get_resource::<TakingSpellModifiers>()?
        .0
        .lock()
        .expect("taking spellmod slots poisoned")
        .get(&caster)
        .cloned()
}
