//! TC a352b1fa spell_evoker.cpp215–239: Disintegrate's Causality AuraScript.
use bevy::prelude::*;
use std::collections::BTreeMap;

use super::scripts::{AuraScript, AuraScripts};
use super::{ActiveAuras, AuraInstance, RemoveReason};

const DISINTEGRATE: u32 = 356995;
const CAUSALITY: u32 = 375777;
const EMPOWERS: &[u32] = &[359073, 357208];

/// TC Load runs once per aura object, not when a later tick gains the passive.
#[derive(Component, Default)]
struct LoadedCausality(BTreeMap<u32, bool>);

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        DISINTEGRATE,
        AuraScript {
            on_apply: Some(load_for_instance),
            on_remove: Some(remove_instance),
            on_periodic: Some(reduce_empower_recovery),
            ..Default::default()
        },
    );
}

fn load_for_instance(world: &mut World, unit: Entity, aura: &AuraInstance) {
    let enabled = super::has_aura(world, aura.caster, CAUSALITY);
    world
        .entity_mut(unit)
        .entry::<LoadedCausality>()
        .or_default()
        .get_mut()
        .0
        .entry(aura.instance_id)
        .or_insert(enabled);
}

fn remove_instance(world: &mut World, unit: Entity, aura: &AuraInstance, _reason: RemoveReason) {
    if let Some(mut loaded) = world.get_mut::<LoadedCausality>(unit) {
        loaded.0.remove(&aura.instance_id);
    }
}

fn read_causal_milliseconds(world: &World, caster: Entity) -> Option<i32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == CAUSALITY)?
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .map(|effect| effect.amount as i32) // TC GetAmountAsInt.
}

fn reduce_empower_recovery(world: &mut World, unit: Entity, aura: &AuraInstance, effect: u8) {
    let loaded = world
        .get::<LoadedCausality>(unit)
        .is_some_and(|state| state.0.get(&aura.instance_id) == Some(&true));
    if effect != 0 || !loaded {
        return;
    }
    let Some(milliseconds) = read_causal_milliseconds(world, aura.caster) else {
        return;
    };
    for &spell in EMPOWERS {
        crate::spell_cast::scripts::reduce_cooldown(
            world,
            aura.caster,
            spell,
            -f64::from(milliseconds) / 1000.0,
        );
    }
}
