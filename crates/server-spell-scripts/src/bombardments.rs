//! Native Bombardments payload and explicitly installed SimC external attempts.
//! The live SpellScriptsPlugin never installs the simulated cadence.
use crate::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Component, Clone)]
pub(crate) struct SimulatedBombardments {
    mean: f64,
    deviation: f64,
    deadlines: BTreeMap<(Entity, u32), f64>,
}

/// SimC76f8118 sc_evoker.cpp1333–1335/8762–8811: external attempts, not damage ticks.
/// Only the parity simulator calls this installer with its SimC option values.
pub fn install_simulation(
    app: &mut App,
    caster: Entity,
    mean: f64,
    deviation: f64,
) -> Result<(), String> {
    if !mean.is_finite() || mean <= 0.0 || !deviation.is_finite() || deviation < 0.0 {
        return Err("Bombardments simulation requires a positive finite mean and nonnegative finite deviation".into());
    }
    if app.world().get::<SimulatedBombardments>(caster).is_some() {
        return Err("Bombardments simulation is already installed for this actor".into());
    }
    app.world_mut()
        .entity_mut(caster)
        .insert(SimulatedBombardments {
            mean,
            deviation,
            deadlines: BTreeMap::new(),
        });
    app.add_systems(Last, advance);
    Ok(())
}

pub(crate) fn launch_damage(world: &mut World, caster: Entity, target: Entity) -> bool {
    let Some(mut payload) = combat::spell_cast::load_triggered_spell(world, 434481, 1) else {
        return false;
    };
    let effect = payload
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Bombardments damage effect0");
    // Native434481 uses deterministic destination-area selectors. Like Crash Lightning,
    // count the native recipients immediately before launching the same native payload.
    let recipients = combat::spell_cast::targets::select_targets(
        world,
        caster,
        Some(target),
        &payload,
        effect,
        0,
        true,
    )
    .unwrap_or_else(|selector| panic!("native Bombardments target selector {selector}"));
    if recipients.is_empty() {
        return false;
    }
    // sc_evoker.cpp8230–8233: split the unmodified native payload, not its coefficients.
    payload.cast_damage_multiplier = Some(1.0 / recipients.len() as f32);
    combat::spell_cast::launch_triggered_spell(world, caster, &payload, Some(target), 1)
}

fn sample_delay(mean: f64, deviation: f64) -> f64 {
    if deviation == 0.0 {
        return mean;
    }
    // sc_evoker.cpp8765–8767 / rng.cpp331–350: normal truncated below max(sd/2,33ms).
    let minimum = (deviation / 2.0).max(0.033);
    loop {
        let radius = (-2.0 * (1.0 - combat::random::random::<f64>()).ln()).sqrt();
        let angle = std::f64::consts::TAU * combat::random::random::<f64>();
        let sample = mean + deviation * radius * angle.cos();
        if sample >= minimum {
            return sample;
        }
    }
}

fn owned_drivers(world: &mut World, caster: Entity) -> Vec<(Entity, u32)> {
    let mut query = world.query::<(Entity, &ActiveAuras)>();
    let mut drivers = Vec::new();
    for (target, auras) in query.iter(world) {
        if !auras
            .iter()
            .any(|aura| aura.spell_id == 434473 && aura.caster == caster)
        {
            continue;
        }
        for driver in auras
            .iter()
            .filter(|aura| aura.spell_id == 443788 && aura.caster == caster)
        {
            drivers.push((target, driver.instance_id));
        }
    }
    drivers.sort_unstable();
    drivers
}

fn advance(world: &mut World) {
    let now = combat::spell_cast::now(world);
    let actors: Vec<_> = world
        .query::<(Entity, &SimulatedBombardments)>()
        .iter(world)
        .map(|(entity, state)| (entity, state.clone()))
        .collect();
    for (caster, mut state) in actors {
        let drivers = owned_drivers(world, caster);
        let active: BTreeSet<_> = drivers.iter().copied().collect();
        state.deadlines.retain(|key, _| active.contains(key));
        for (target, instance) in drivers {
            let deadline = state
                .deadlines
                .entry((target, instance))
                .or_insert_with(|| now + sample_delay(state.mean, state.deviation));
            if now < *deadline {
                continue;
            }
            *deadline += sample_delay(state.mean, state.deviation);
            // sc_evoker.cpp8800–8811: actual native443788 admission owns chance and ICD.
            if !combat::auras::procs::try_native_policy_proc(world, target, instance) {
                continue;
            }
            if !launch_damage(world, caster, target) {
                warn!(
                    ?caster,
                    ?target,
                    "Simulated external Bombardments native payload refused"
                );
            }
        }
        world.entity_mut(caster).insert(state);
    }
}
