//! SimC76f8118 sc_shaman.cpp4545–4561/4682–4689/5540–5545/12041–12073:
//! native Stormflurry rolls schedule background Strikes with their retained Stormblast snapshot.
use super::combat;
use bevy::prelude::*;
use shared::components::ActiveSpec;

use combat::class_progression::KnownSpells;
use combat::spell_cast;
use combat::spell_info::{SpellInfo, SpellStore};

const STORMFLURRY: u32 = 344357;
const DELAY_MEAN_SECONDS: f64 = 0.200;
const DELAY_STD_SECONDS: f64 = 0.025;

struct PendingFlurry {
    at: f64,
    root: u32,
    target: Entity,
    stormblast: bool,
}

#[derive(Component, Default)]
struct PendingFlurries(Vec<PendingFlurry>);

#[derive(Component)]
struct ExecutingFlurry;

pub(super) fn is_executing(world: &World, caster: Entity) -> bool {
    world.get::<ExecutingFlurry>(caster).is_some()
}

pub(super) fn multiplier(world: &World, caster: Entity) -> f32 {
    if is_executing(world, caster) {
        super::script_effect_value(world, caster, STORMFLURRY, 1) / 100.0
    } else {
        1.0
    }
}

pub(super) fn weapon_multiplier(
    world: &World,
    caster: Entity,
    parent: &SpellInfo,
    child: u32,
) -> Option<f32> {
    (matches!(
        (parent.id, child),
        (17364, 32175 | 32176) | (115356, 115357 | 115360)
    ) && is_executing(world, caster))
    .then(|| multiplier(world, caster))
}

fn is_selected(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 263)
        && world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(STORMFLURRY))
}

pub(super) fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !matches!(spell.id, 17364 | 115356) || !is_selected(world, caster) {
        return;
    }
    let chance = super::script_effect_value(world, caster, STORMFLURRY, 0);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    let target = spell
        .cast_primary_target
        .expect("Stormflurry retains its actual native Strike target");
    let pending = PendingFlurry {
        at: spell_cast::now(world) + roll_delay_seconds(),
        root: spell.id,
        target,
        stormblast: super::enhancement_stormblast::snapshotted(world, caster),
    };
    if world.get::<PendingFlurries>(caster).is_none() {
        world.entity_mut(caster).insert(PendingFlurries::default());
    }
    world
        .get_mut::<PendingFlurries>(caster)
        .unwrap()
        .0
        .push(pending);
}

fn roll_delay_seconds() -> f64 {
    // Source12056: Gaussian mean200ms, standard deviation25ms, not a uniform jitter.
    let radial = (1.0 - combat::random::random::<f64>()).ln() * -2.0;
    let angle = std::f64::consts::TAU * combat::random::random::<f64>();
    (DELAY_MEAN_SECONDS + DELAY_STD_SECONDS * radial.sqrt() * angle.cos()).max(0.0)
}

pub(super) fn update(world: &mut World) {
    let now = spell_cast::now(world);
    let casters: Vec<Entity> = world
        .query_filtered::<Entity, With<PendingFlurries>>()
        .iter(world)
        .collect();
    for caster in casters {
        for pending in take_ready(world, caster, now) {
            execute(world, caster, pending);
        }
    }
}

fn take_ready(world: &mut World, caster: Entity, now: f64) -> Vec<PendingFlurry> {
    let mut pending = world.get_mut::<PendingFlurries>(caster).unwrap();
    let (ready, later) = std::mem::take(&mut pending.0)
        .into_iter()
        .partition(|flurry| flurry.at <= now);
    pending.0 = later;
    ready
}

fn execute(world: &mut World, caster: Entity, pending: PendingFlurry) {
    let mut payload = world
        .resource::<SpellStore>()
        .get(pending.root)
        .expect("native Stormflurry Strike root")
        .clone();
    payload.cast_primary_target = Some(pending.target);
    world.entity_mut(caster).insert(ExecutingFlurry);
    // Native weapon children resolve synchronously; the next delayed repeat captures this same snapshot.
    super::enhancement_stormblast::execute_flurry(world, caster, &payload, pending.stormblast);
    world.entity_mut(caster).remove::<ExecutingFlurry>();
}
