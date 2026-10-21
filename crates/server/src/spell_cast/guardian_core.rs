//! SimC76f8118 warlock init1261–1265/pets963–983: natural Imp fade PRD.
//! Native267102 effect0 supplies nominal chance;21-attempt cap is pinned behavior.
use crate::random::Rng;
use bevy::prelude::*;

const DEMONIAC: u32 = 426115;
const CORE_CHANCE: u32 = 267102;
const CORE_BUFF: u32 = 264173;
const FADE_CAP: u32 = 21;

#[derive(Component, Default)]
struct FadeAttempts(u32);

pub(super) fn on_energy_fade(world: &mut World, owner: Entity) {
    if !world
        .get::<crate::class_progression::KnownSpells>(owner)
        .is_some_and(|known| known.contains(DEMONIAC))
    {
        return;
    }
    let chance = world
        .resource::<crate::spell_info::SpellStore>()
        .get(CORE_CHANCE)
        .expect("Demoniac requires native Core chance metadata")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points as f64
        / 100.0;
    if roll_energy_fade(world, owner, chance) {
        crate::spell_cast::cast_triggered(world, owner, CORE_BUFF, Some(owner), 0);
    }
}

fn roll_energy_fade(world: &mut World, owner: Entity, chance: f64) -> bool {
    let coefficient = calculate_fade_coefficient(chance);
    if world.get::<FadeAttempts>(owner).is_none() {
        world.entity_mut(owner).insert(FadeAttempts::default());
    }
    let mut attempts = world.get_mut::<FadeAttempts>(owner).unwrap();
    attempts.0 += 1;
    let proc_chance = if attempts.0 >= FADE_CAP {
        1.0
    } else {
        coefficient * f64::from(attempts.0)
    };
    if !crate::random::thread_rng().gen_bool(proc_chance.min(1.0)) {
        return false;
    }
    attempts.0 = 0;
    true
}

/// SimC76f8118 actions2966: successful ordinary Demonbolt consumes one Core.
/// Native264173 has no proc charges; consume an actual stack, not the whole aura.
pub(crate) fn consume_core(
    world: &mut World,
    owner: Entity,
    _spell: &crate::spell_info::SpellInfo,
) {
    let instance = world
        .get::<crate::auras::ActiveAuras>(owner)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == CORE_BUFF))
        .map(|aura| aura.instance_id);
    if let Some(instance) = instance {
        crate::auras::spend_stack(world, owner, instance);
    }
}

// Solve E[attempts] = 1/p for the capped accumulated distribution.
// This derives the PRD constant from native chance rather than copying numeric10% data.
fn calculate_fade_coefficient(chance: f64) -> f64 {
    assert!(
        chance > 1.0 / f64::from(FADE_CAP) && chance <= 1.0,
        "native Demoniac chance must support the pinned capped distribution"
    );
    let target = 1.0 / chance;
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..48 {
        let middle = (low + high) / 2.0;
        if expected_fade_attempts(middle) > target {
            low = middle;
        } else {
            high = middle;
        }
    }
    (low + high) / 2.0
}

fn expected_fade_attempts(coefficient: f64) -> f64 {
    let mut survival = 1.0;
    let mut expected = 0.0;
    for attempt in 1..=FADE_CAP {
        expected += survival;
        survival *= 1.0 - (coefficient * f64::from(attempt)).min(1.0);
    }
    expected
}

#[cfg(test)]
mod tests {
    #[test]
    fn demoniac_capped_distribution_matches_native_nominal_chance() {
        for nominal in [0.1, 0.2, 0.5] {
            let coefficient = super::calculate_fade_coefficient(nominal);
            let actual = 1.0 / super::expected_fade_attempts(coefficient);
            assert!((actual - nominal).abs() < 1e-10);
        }
    }
}
