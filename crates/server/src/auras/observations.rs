//! Test-only passive receipts at native application/removal/stack-spend boundaries.
use bevy::prelude::*;
use serde::Serialize;
use std::collections::BTreeMap;

use super::{ActiveAuras, AuraInstance, RemoveReason};

const TRACKED: &[(u32, u32)] = &[(16870, 113043), (114108, 158478)];

#[derive(Default, Serialize)]
struct ProcUsage {
    talent_present: bool,
    initial_charges: u64,
    applications: u64,
    charges_gained: u64,
    consumed: u64,
    expired: u64,
    other_removed: u64,
    overcap_refreshes: u64,
}

#[derive(Resource)]
struct AuraObservations {
    caster: Entity,
    usage: BTreeMap<u32, ProcUsage>,
}

fn own_stacks(world: &World, caster: Entity, spell: u32) -> u8 {
    world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == caster && aura.spell_id == spell)
        })
        .map_or(0, |aura| aura.stacks.max(1))
}

pub(crate) fn start(world: &mut World, caster: Entity) {
    let usage = TRACKED
        .iter()
        .map(|&(spell, talent)| {
            let row = ProcUsage {
                talent_present: own_stacks(world, caster, talent) > 0,
                initial_charges: u64::from(own_stacks(world, caster, spell)),
                ..Default::default()
            };
            (spell, row)
        })
        .collect();
    world.insert_resource(AuraObservations { caster, usage });
}

pub(super) fn previous_stacks(world: &World, target: Entity, aura: &AuraInstance) -> Option<u8> {
    let probe = world.get_resource::<AuraObservations>()?;
    let observed = target == probe.caster
        && aura.caster == probe.caster
        && probe.usage.contains_key(&aura.spell_id);
    observed.then(|| own_stacks(world, probe.caster, aura.spell_id))
}

pub(super) fn applied(world: &mut World, aura: &AuraInstance, previous: Option<u8>) {
    let Some(previous) = previous else {
        return;
    };
    let mut probe = world.resource_mut::<AuraObservations>();
    let row = probe
        .usage
        .get_mut(&aura.spell_id)
        .expect("observed native proc");
    let gained = aura.stacks.max(1).saturating_sub(previous);
    row.applications += 1;
    row.charges_gained += u64::from(gained);
    row.overcap_refreshes += u64::from(gained == 0);
}

fn record_loss(
    world: &mut World,
    target: Entity,
    caster: Entity,
    spell: u32,
    charges: u64,
    reason: RemoveReason,
) {
    let Some(mut probe) = world.get_resource_mut::<AuraObservations>() else {
        return;
    };
    if target != probe.caster || caster != probe.caster {
        return;
    }
    let Some(row) = probe.usage.get_mut(&spell) else {
        return;
    };
    match reason {
        RemoveReason::Consumed => row.consumed += charges,
        RemoveReason::Expired => row.expired += charges,
        _ => row.other_removed += charges,
    }
}

pub(super) fn removed(
    world: &mut World,
    target: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    record_loss(
        world,
        target,
        aura.caster,
        aura.spell_id,
        u64::from(aura.stacks.max(1)),
        reason,
    );
}

pub(super) fn spent_stack(world: &mut World, target: Entity, caster: Entity, spell: u32) {
    record_loss(world, target, caster, spell, 1, RemoveReason::Consumed);
}

pub(crate) fn snapshot(world: &World) -> serde_json::Value {
    let probe = world.resource::<AuraObservations>();
    let rows: BTreeMap<_, _> = probe
        .usage
        .iter()
        .map(|(spell, row)| {
            let available = row.initial_charges + row.charges_gained;
            let charge_rate = (available > 0).then(|| row.consumed as f64 / available as f64);
            let application_rate =
                (row.applications > 0).then(|| row.consumed as f64 / row.applications as f64);
            let mut value = serde_json::to_value(row).expect("native proc counters serialize");
            value["charge_consumption_rate"] = serde_json::json!(charge_rate);
            value["application_consumption_rate"] = serde_json::json!(application_rate);
            value["remaining_charges"] = serde_json::json!(own_stacks(world, probe.caster, *spell));
            (*spell, value)
        })
        .collect();
    serde_json::to_value(rows).expect("native proc report serializes")
}
