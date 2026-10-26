//! Actual Infernal native driver activation, without synthetic ticks or resources.
use super::combat;
use bevy::prelude::*;
use combat::random as rand;
use combat::spell_cast::{guardian, script_registry::SpellScriptHook, summon::SummonedCreature};
use shared::components::{ActiveSpec, Npc};

const INFERNAL_ROOT: u32 = 111685;
const INFERNAL: u32 = 89;
const EMBERS: u32 = 264364;
const IMMOLATION: u32 = 19483;
const ARISE_MEAN_SECONDS: f64 = 1.0;
const ARISE_STDDEV_SECONDS: f64 = 0.1;
const IMMOLATION_EXTRA_SECONDS: f64 = 0.75;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::SummonInitialized(schedule_native_drivers)];

fn schedule_native_drivers(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(actor) else {
        return;
    };
    let infernal = summon.spell_id == INFERNAL_ROOT
        && world
            .get::<Npc>(actor)
            .is_some_and(|npc| npc.template_id == INFERNAL);
    let destruction = world
        .get::<ActiveSpec>(summon.owner)
        .is_some_and(|spec| spec.0 == 267);
    if infernal && destruction {
        guardian::initialize_action_delays(world, actor, sample_native_delays);
    }
}

fn sample_native_delays() -> Vec<(u32, f64)> {
    // SimC76f8118 pets.cpp2103–2119: normal1s/stddev0.1s arise;
    // Embers starts then, Immolation after another uniform0–0.75s, without its zero tick.
    let arise = sample_arise_delay();
    let immolation =
        arise + rand::Rng::gen_range(&mut rand::thread_rng(), 0.0..IMMOLATION_EXTRA_SECONDS);
    vec![(EMBERS, arise), (IMMOLATION, immolation)]
}

fn sample_arise_delay() -> f64 {
    // The pinned timespan Gaussian is truncated below zero (rng.hpp).
    loop {
        let radial = 1.0 - rand::random::<f64>();
        let angle = std::f64::consts::TAU * rand::random::<f64>();
        let normal = (-2.0 * radial.ln()).sqrt() * angle.cos();
        let delay = ARISE_MEAN_SECONDS + ARISE_STDDEV_SECONDS * normal;
        if delay >= 0.0 {
            return delay;
        }
    }
}
