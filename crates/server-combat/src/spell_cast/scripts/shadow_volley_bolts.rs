//! Native Volley recipients and50ms child launch cadence.
use super::script_effect_value;
use crate::spell_cast::{self, targets};
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;
const VOID_VOLLEY: u32 = 1242173;
const VOLLEY_BOLT: u32 = 1242189;

pub(super) fn volley(world: &mut World, caster: Entity, primary: Entity, depth: u8) {
    if depth >= spell_cast::effects::MAX_TRIGGER_DEPTH {
        warn!("Void Volley trigger depth exhausted");
        return;
    }
    let root = world
        .resource::<SpellStore>()
        .get(VOID_VOLLEY)
        .expect("Volley root")
        .clone();
    let center = targets::position(world, primary).expect("Volley hit target position");
    let bolt = world
        .resource::<SpellStore>()
        .get(VOLLEY_BOLT)
        .expect("Volley requires1242189")
        .clone();
    let units = targets::area_units(
        world,
        caster,
        center,
        targets::radius(&root.effects[0], 1),
        targets::Relation::Enemy,
        0,
    );
    for unit in units {
        let count = script_effect_value(
            world,
            caster,
            VOID_VOLLEY,
            if unit == primary { 0 } else { 2 },
        ) as usize;
        launch_bolts(world, caster, unit, &bolt, count, depth + 1);
    }
}

fn launch_bolts(
    world: &mut World,
    caster: Entity,
    unit: Entity,
    bolt: &SpellInfo,
    count: usize,
    depth: u8,
) {
    let ctx = spell_cast::effects::EffectCtx {
        caster,
        spell: bolt,
        depth,
        dest: None,
        chain_multiplier: 1.0,
    };
    // SimC76f8118 sc_priest_shadow.cpp982–1014 explicitly repeats every50ms.
    for index in 1..=count {
        spell_cast::queued_triggers::schedule_full_trigger(
            world,
            &ctx,
            unit,
            bolt.id,
            index as u32 * 50,
        );
    }
}
