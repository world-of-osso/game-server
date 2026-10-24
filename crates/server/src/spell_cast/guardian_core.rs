//! Data-bound natural summon retirement: existing Imp fade PRD and named Dread expiry policy.
//! SimC76f8118 init1261–1265/pets963–983,1248–1253; native267102 supplies both chances.
use crate::random::Rng;
use bevy::prelude::*;

const DEMONIAC: u32 = 426115;
const CORE_CHANCE: u32 = 267102;
const CORE_BUFF: u32 = 264173;
const FADE_CAP: u32 = 21;

#[derive(Clone, Copy)]
enum RetirementRoll {
    AccumulatedFade,
    Independent,
}

struct RetirementBinding {
    root: u32,
    creature: u32,
    chance_effect: u8,
    roll: RetirementRoll,
}

const RETIREMENT_BINDINGS: &[RetirementBinding] = &[
    RetirementBinding {
        root: 104317,
        creature: 55659,
        chance_effect: 0,
        roll: RetirementRoll::AccumulatedFade,
    },
    // SIMC_DEMO_ALTERNATE_WILD_IMP_IDENTITY: same sourced WildImp fade PRD.
    RetirementBinding {
        root: 279910,
        creature: 143622,
        chance_effect: 0,
        roll: RetirementRoll::AccumulatedFade,
    },
    // SIMC_DEMONIC_CORE_DREADSTALKER_EXPIRY: no TC/TDB event hook.
    // Pinned pets1248–1253; actual267102 Effect1 owns the chance, not this policy.
    RetirementBinding {
        root: 193331,
        creature: 98035,
        chance_effect: 1,
        roll: RetirementRoll::Independent,
    },
    RetirementBinding {
        root: 193332,
        creature: 98035,
        chance_effect: 1,
        roll: RetirementRoll::Independent,
    },
];

#[derive(Component, Default)]
struct FadeAttempts(u32);

pub(crate) fn on_natural_retirement(world: &mut World, actor: Entity) {
    let Some((owner, binding)) = read_retirement_binding(world, actor) else {
        return;
    };
    if !world
        .get::<crate::class_progression::KnownSpells>(owner)
        .is_some_and(|known| known.contains(DEMONIAC))
    {
        return;
    }
    let chance = read_core_chance(world, binding.chance_effect);
    let earned = match binding.roll {
        RetirementRoll::AccumulatedFade => roll_energy_fade(world, owner, chance),
        RetirementRoll::Independent => crate::random::thread_rng().gen_bool(chance),
    };
    if earned {
        crate::spell_cast::cast_triggered(world, owner, CORE_BUFF, Some(owner), 0);
    }
}

fn read_retirement_binding(
    world: &World,
    actor: Entity,
) -> Option<(Entity, &'static RetirementBinding)> {
    let summon = world.get::<super::super::summon::SummonedCreature>(actor)?;
    let creature = world.get::<shared::components::Npc>(actor)?.template_id;
    RETIREMENT_BINDINGS
        .iter()
        .find(|binding| binding.root == summon.spell_id && binding.creature == creature)
        .map(|binding| (summon.owner, binding))
}

fn read_core_chance(world: &World, index: u8) -> f64 {
    world
        .resource::<crate::spell_info::SpellStore>()
        .get(CORE_CHANCE)
        .expect("Demoniac requires native Core chance metadata")
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("selected retirement source requires native267102 chance effect")
        .base_points as f64
        / 100.0
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
