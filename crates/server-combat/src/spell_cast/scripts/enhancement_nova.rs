//! SimC76f8118 sc_shaman.cpp10126–10130/7043–7076: real Voltaic executes Fire Nova over its own dots.
use bevy::prelude::*;
use shared::components::Health;

use crate::auras::ActiveAuras;
use crate::class_progression::KnownSpells;
use crate::random::Rng;
use crate::spell_cast::{self, targets};

const TALENT: u32 = 1260666;
const FLAME_SHOCK: u32 = 188389;
const EXPLOSION: u32 = 333977;

pub(super) fn roll_and_cast(world: &mut World, caster: Entity) {
    let selected = world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(TALENT));
    if !selected {
        return;
    }
    // SimC10126 rolls native talent effect2 before checking its dot-dependent target list.
    let chance = super::script_effect_value(world, caster, TALENT, 1);
    if crate::random::thread_rng().gen_range(0.0..100.0) >= chance {
        return;
    }
    for target in select_own_flame_shocks(world, caster) {
        // Native333977 owns its explosion geometry, target modifiers, damage and four-piece reward.
        spell_cast::cast_triggered(world, caster, EXPLOSION, Some(target), 1);
    }
}

fn select_own_flame_shocks(world: &mut World, caster: Entity) -> Vec<Entity> {
    world
        .query::<(Entity, &ActiveAuras, &Health)>()
        .iter(world)
        .filter_map(|(target, active, health)| {
            let own_dot = active
                .iter()
                .any(|aura| aura.spell_id == FLAME_SHOCK && aura.caster == caster);
            let attackable = targets::same_map(world, caster, target)
                && targets::relation_matches(world, caster, target, targets::Relation::Enemy);
            (own_dot && attackable && health.current > 0.0).then_some(target)
        })
        .collect()
}
