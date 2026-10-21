//! TC a352b1fa spell_shaman.cpp1793–1867: normalized Flame Shock ticks and RestoreCharge.
//! SimC76f8118 sc_shaman.cpp7230–7258,12000–12020: defer reset/consumption mid-Lava Burst.
use bevy::prelude::*;
use shared::casting::{CastState, CastType};

use crate::auras::ActiveAuras;
use crate::maps::InMap;
use crate::spell_info::{SpellInfo, SpellStore};

const PASSIVE: u32 = 77756;
pub(crate) const PROC: u32 = 77762;
pub(crate) const FLAME_SHOCK: u32 = 188389;
pub(super) const LAVA_BURST: u32 = 51505;
const PROC_RADIUS: f32 = 100.0;
const PROTECTED_TICKS: f64 = 6.0;

#[derive(Component)]
struct NormalizedTicks {
    passive_instance: u32,
    ticks: f64,
}

pub(crate) fn roll_flame_shock_tick(world: &mut World, caster: Entity) {
    let Some((instance, chance)) = passive_chance(world, caster) else {
        return;
    };
    let count = active_flame_shocks(world, caster);
    assert!(count > 0, "a real Flame Shock tick retains its owned dot");
    let ticks = advance_normalized_ticks(world, caster, instance, count);
    if ticks < PROTECTED_TICKS {
        return;
    }
    let miss_chance = (1.0 - f64::from(chance) / 100.0).max(0.0);
    let chance = 1.0 - miss_chance.powf(ticks);
    if rand::random::<f64>() < chance {
        world.get_mut::<NormalizedTicks>(caster).unwrap().ticks = 0.0;
        crate::spell_cast::cast_triggered(world, caster, PROC, Some(caster), 1);
    }
}

fn passive_chance(world: &World, caster: Entity) -> Option<(u32, f32)> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == PASSIVE)
        .map(|aura| {
            let chance = aura
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .expect("native Lava Surge retains its chance effect")
                .amount;
            (aura.instance_id, chance)
        })
}

fn active_flame_shocks(world: &mut World, caster: Entity) -> usize {
    let from =
        crate::spell_cast::targets::position(world, caster).expect("ticking Shaman position");
    let map = world.get::<InMap>(caster).copied();
    let mut query = world.query::<(Entity, &ActiveAuras, &InMap)>();
    query
        .iter(world)
        .filter(|(unit, auras, target_map)| {
            Some(**target_map) == map
                && crate::spell_cast::targets::position(world, *unit)
                    .is_some_and(|to| from.distance_squared(to) <= PROC_RADIUS * PROC_RADIUS)
                && auras
                    .iter()
                    .any(|aura| aura.spell_id == FLAME_SHOCK && aura.caster == caster)
        })
        .count()
}

fn advance_normalized_ticks(world: &mut World, caster: Entity, instance: u32, count: usize) -> f64 {
    let reset = world
        .get::<NormalizedTicks>(caster)
        .is_none_or(|state| state.passive_instance != instance);
    if reset {
        world.entity_mut(caster).insert(NormalizedTicks {
            passive_instance: instance,
            ticks: 0.0,
        });
    }
    let mut state = world.get_mut::<NormalizedTicks>(caster).unwrap();
    state.ticks += 1.0 / count as f64;
    state.ticks
}

pub(crate) fn restore_on_proc_hit(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    let in_flight = world
        .get::<CastState>(caster)
        .is_some_and(|cast| cast.spell_id == LAVA_BURST && cast.cast_type == CastType::Normal);
    if !in_flight {
        restore_lava_charge(world, caster);
    }
}

pub(crate) fn restore_deferred_charge(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let earned_midcast = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == PROC && !spell.cast_proc_mods.contains(&aura.instance_id))
    });
    if earned_midcast {
        restore_lava_charge(world, caster);
    }
}

fn restore_lava_charge(world: &mut World, caster: Entity) {
    let category = world
        .resource::<SpellStore>()
        .get(LAVA_BURST)
        .expect("native Lava Burst")
        .categories
        .as_ref()
        .expect("native Lava Burst charge category")
        .charge_category;
    crate::spell_cast::restore_charges(world, caster, category, 1);
}
