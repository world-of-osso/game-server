//! Permanent Unholy ghoul policy; Hunter/Warlock/Mage paths are unchanged.
use crate::stats::{CharacterStats, StatModifiers, WeaponAttackType};
use bevy::prelude::*;
use shared::components::{Npc, WeaponType};

#[derive(Component)]
struct DeathKnightGhoul;

pub(super) fn initialize(world: &mut World, owner: Entity, pet: Entity) {
    if super::owner_class(world, owner) != Some(6)
        || world
            .get::<Npc>(pet)
            .is_none_or(|npc| npc.template_id != 26125)
    {
        return;
    }
    world.entity_mut(pet).insert(DeathKnightGhoul);
    refresh_pet(world, owner, pet);
}

pub(super) fn refresh(world: &mut World) {
    let pets: Vec<_> = world
        .query_filtered::<(Entity, &super::HunterPet), With<DeathKnightGhoul>>()
        .iter(world)
        .map(|(pet, relation)| (pet, relation.owner))
        .collect();
    for (pet, owner) in pets {
        refresh_pet(world, owner, pet);
    }
}

fn refresh_pet(world: &mut World, owner: Entity, pet: Entity) {
    let owner_stats = *world
        .get::<CharacterStats>(owner)
        .expect("native DK ghoul requires derived owner stats");
    // SimC76f8118 sc_death_knight.cpp3343–3348/3362–3366: actual owner
    // melee AP, including the selected native Unholy Armaments contribution.
    let mut share = 0.474;
    if crate::auras::has_aura(world, owner, 1265971) {
        share *=
            1.0 + crate::spell_cast::scripts::script_effect_value(world, owner, 1265971, 2) / 100.0;
    }
    let attack_power = owner_stats.total_attack_power(
        WeaponAttackType::MainHand,
        world.get::<StatModifiers>(owner),
    ) * share;
    // sc_death_knight.cpp3168/2690–2710: two-second beast swings and live
    // owner critical/haste inheritance; native actions retain their coefficients.
    let stats = CharacterStats {
        attack_power,
        crit_pct: owner_stats.crit_pct,
        haste_pct: owner_stats.haste_pct,
        versatility_pct: owner_stats.versatility_pct,
        main_hand: crate::item_stats::WeaponDamage {
            min: 0.0,
            max: 0.0,
            speed: 2.0,
            kind: WeaponType::OneHand,
        },
        ..Default::default()
    };
    if world.get::<CharacterStats>(pet) != Some(&stats) {
        world.entity_mut(pet).insert(stats);
    }
}

pub(crate) fn power_stats(world: &World, pet: Entity) -> Option<(f32, f32)> {
    world.get::<DeathKnightGhoul>(pet)?;
    let stats = world
        .get::<CharacterStats>(pet)
        .expect("native ghoul stat policy");
    Some((stats.spell_power, stats.attack_power))
}
