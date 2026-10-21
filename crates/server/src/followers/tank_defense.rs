//! FOLLOWER_TANK_DEFENSIVE_SEQUENCING: explicit project policy, not a Retail APL.
use bevy::prelude::*;

use super::kit::Entry;
use crate::auras::{
    ActiveAuras,
    spell_mods::{self, SpellModOp},
};
use crate::spell_info::SpellStore;

const MANUAL_MAJORS: &[u32] = &[31850, 86659, 871];
const FALLBACK_HEALTH: f32 = 0.50;

// Temporary API-shaped shim: healer-ai is not on master yet. No fabricated
// estimator. Replace these definitions with the real read_patient/Patient import
// after the lead lands healer-ai; its shared horizon is currently3seconds.
struct Patient {
    incoming_per_second: f32,
    projected_death_seconds: f32,
}
fn read_patient(_world: &World, _target: Entity) -> Option<Patient> {
    None
}

fn major_emergency(health: f32, projection: Option<(f32, f32)>) -> bool {
    // Same documented horizon as healer-ai; import its constant when it lands.
    const EMERGENCY_SECONDS: f32 = 3.0;
    match projection {
        Some((incoming, death_seconds)) if incoming > 0.0 => death_seconds <= EMERGENCY_SECONDS,
        _ => health < FALLBACK_HEALTH,
    }
}

pub(super) fn allows(world: &World, unit: Entity, health: Option<f32>) -> bool {
    let active = world.get::<ActiveAuras>(unit).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| MANUAL_MAJORS.contains(&aura.spell_id))
    });
    if active {
        return false;
    }
    let Some(health) = health else {
        return false;
    };
    let projection = read_patient(world, unit)
        .map(|patient| (patient.incoming_per_second, patient.projected_death_seconds));
    major_emergency(health, projection)
}

/// Only opted-in major slots are reordered; every other priority position stays.
pub(super) fn sort_major_entries(world: &World, unit: Entity, entries: &mut [Entry]) {
    let positions: Vec<_> = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| entry.conditions.tank_defensive_sequence.then_some(index))
        .collect();
    let mut majors: Vec<_> = positions
        .iter()
        .map(|index| entries[*index].clone())
        .collect();
    majors.sort_by(|left, right| {
        calculate_cooldown_ms(world, unit, left.spell_id).total_cmp(&calculate_cooldown_ms(
            world,
            unit,
            right.spell_id,
        ))
    });
    for (index, entry) in positions.into_iter().zip(majors) {
        entries[index] = entry;
    }
}

fn calculate_cooldown_ms(world: &World, unit: Entity, spell_id: u32) -> f32 {
    let spell = world
        .resource::<SpellStore>()
        .get(spell_id)
        .expect("source-qualified tank major requires native spell metadata");
    let recovery = spell.cooldowns.as_ref().map_or(0, |source| {
        source.recovery_ms.max(source.category_recovery_ms)
    });
    let charge = spell
        .categories
        .as_ref()
        .map_or(0, |source| source.charge_recovery_ms);
    spell_mods::modify(
        world,
        unit,
        spell,
        SpellModOp::Cooldown,
        recovery.max(charge) as f32,
    )
    .max(0.0)
}

#[cfg(test)]
mod tests {
    use super::major_emergency;

    #[test]
    fn rotation_fidelity_tank_projection_uses_healer_horizon_when_available() {
        let cases = [
            (0.9, Some((10.0, 2.0))),
            (0.1, Some((1.0, 20.0))),
            (0.49, None),
            (0.49, Some((0.0, f32::INFINITY))),
            (0.50, None),
            (0.9, Some((10.0, 3.0))),
            (0.9, Some((10.0, 3.01))),
        ];
        let actual: Vec<_> = cases
            .into_iter()
            .map(|(health, projection)| major_emergency(health, projection))
            .collect();
        assert_eq!(
            actual,
            [true, false, true, true, false, true, false],
            "observed incoming uses shared3s horizon, not raw HP; no projection retains50% fallback"
        );
    }
}
