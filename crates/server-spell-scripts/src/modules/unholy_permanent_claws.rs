//! Autopilot abilities for the controlled Unholy ghoul.
use super::combat;
use bevy::prelude::*;
use combat::combat::AutoAttack;
use combat::hunter_pet::HunterPet;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;
use combat::stats::CharacterStats;
use shared::components::{ActiveSpec, Npc, Position};

#[derive(Component)]
struct Energy {
    value: f64,
    updated: f64,
    ready: f64,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AdditionalReachActions(autonomous_actions),
    SpellScriptHook::AfterCastUpdate(update),
];

fn autonomous_actions(world: &World, pet: Entity) -> Vec<u32> {
    let Some(owner) = world.get::<HunterPet>(pet).map(|pet| pet.owner) else {
        return Vec::new();
    };
    let ghoul = world
        .get::<Npc>(pet)
        .is_some_and(|npc| npc.template_id == 26125);
    let unholy = world
        .get::<ActiveSpec>(owner)
        .is_some_and(|spec| spec.0 == 252);
    if !ghoul || !unholy {
        return Vec::new();
    }
    let transformed =
        combat::spell_cast::script_registry::reach_source_selected(world, owner, 1233448)
            || combat::auras::has_aura(world, pet, 1233448);
    if transformed {
        vec![91776, 91778]
    } else {
        vec![91776]
    }
}

// server-combat compiles this file in its own tests (cfg(test)), where no test uses it.
#[cfg(all(feature = "test-support", not(test)))]
pub fn set_ghoul_energy(world: &mut World, pet: Entity, value: f64) {
    let now = combat::auras::aura_now(world);
    let mut energy = world.get_mut::<Energy>(pet).expect("actual ghoul Energy");
    energy.value = value;
    energy.updated = now;
}

#[cfg(all(feature = "test-support", not(test)))]
pub fn ghoul_energy(world: &World, pet: Entity) -> f64 {
    world.get::<Energy>(pet).expect("actual ghoul Energy").value
}

fn update(world: &mut World) {
    let now = combat::auras::aura_now(world);
    let pets: Vec<_> = world
        .query::<(Entity, &HunterPet, &Npc)>()
        .iter(world)
        .filter_map(|(pet, relation, npc)| {
            (npc.template_id == 26125).then_some((pet, relation.owner))
        })
        .collect();
    for (pet, owner) in pets {
        if world
            .get::<ActiveSpec>(owner)
            .is_some_and(|spec| spec.0 == 252)
        {
            update_pet(world, pet, owner, now);
        }
    }
}

fn update_pet(world: &mut World, pet: Entity, owner: Entity, now: f64) {
    if world.get::<Energy>(pet).is_none() {
        world.entity_mut(pet).insert(Energy {
            value: 100.0,
            updated: now,
            ready: now,
        });
    }
    let haste = world
        .get::<CharacterStats>(owner)
        .expect("ghoul owner stats")
        .haste_pct;
    let devotion = world
        .get::<combat::auras::ActiveAuras>(pet)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 1270491))
        .and_then(|aura| {
            aura.effects
                .iter()
                .find(|effect| effect.index == 2)
                .map(|effect| effect.amount * aura.stack_multiplier())
        })
        .unwrap_or(0.0);
    let energy = world.get::<Energy>(pet).unwrap();
    // SimC76f8118 sc_death_knight.cpp3168–3180/3399–3405: ghoul Energy
    // regenerates with owner haste and each active Unholy Devotion stack.
    let value = (energy.value
        + (now - energy.updated).max(0.0)
            * 10.0
            * (1.0 + f64::from(haste) / 100.0)
            * (1.0 + f64::from(devotion) / 100.0))
        .min(100.0);
    let ready = energy.ready;
    let mut energy = world.get_mut::<Energy>(pet).unwrap();
    energy.value = value;
    energy.updated = now;
    let Some(target) = world.get::<AutoAttack>(pet).map(|attack| attack.victim) else {
        return;
    };
    let (Some(a), Some(b)) = (world.get::<Position>(pet), world.get::<Position>(target)) else {
        return;
    };
    let distance_sq = (a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2);
    if distance_sq > combat::combat::MELEE_RANGE.powi(2) || now < ready {
        return;
    }
    let transformed = combat::auras::has_aura(world, pet, 1233448);
    let has_transformation = world
        .get::<combat::class_progression::KnownSpells>(owner)
        .is_some_and(|known| known.contains(1233448));
    if has_transformation && !transformed {
        return;
    }
    let spell = if transformed { 91778 } else { 91776 };
    let cost = world
        .resource::<SpellStore>()
        .get(spell)
        .expect("native ghoul attack")
        .power_costs
        .iter()
        .find(|cost| cost.power_type == 3)
        .expect("native ghoul energy cost")
        .mana_cost as f64;
    // SimC76f8118 sc_death_knight.cpp3270–3324/3427–3439: sweeping
    // claws during transformation; otherwise claw only above 70 Energy.
    if value < cost || (!transformed && value <= 70.0) {
        return;
    }
    if spell_cast::try_cast_triggered(world, pet, spell, Some(target), 1) {
        let mut energy = world.get_mut::<Energy>(pet).unwrap();
        energy.value -= cost;
        energy.ready = now + 1.0 / (1.0 + f64::from(haste) / 100.0);
    }
}
