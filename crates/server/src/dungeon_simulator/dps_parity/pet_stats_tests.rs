//! Native pet stat baseline before inheritance changes; no synthetic stat values.
use super::*;
use shared::components::{Health, UnitLevel};

fn record_pet_stats(world: &World, owner: Entity, pet: Entity, fixture: &str) {
    let stats = world.get::<crate::stats::CharacterStats>(owner).unwrap();
    let health = world.get::<Health>(pet).unwrap();
    let creature_armor = world
        .get::<crate::level_scaling::CreatureArmor>(pet)
        .map(|armor| armor.0);
    let pet_stats = world.get::<crate::stats::CharacterStats>(pet);
    let receipt = json!({"fixture": fixture, "owner": owner.to_bits(), "pet": pet.to_bits(),
        "pet_level": world.get::<UnitLevel>(pet).unwrap().0,
        "pet_entry": world.get::<shared::components::Npc>(pet).unwrap().template_id,
        "owner_stamina": stats.primary.stamina, "owner_armor": stats.armor,
        "owner_melee_ap": stats.attack_power, "owner_ranged_ap": stats.ranged_attack_power,
        "pet_current_hp": health.current, "pet_max_hp": health.max,
        "pet_creature_armor": creature_armor,
        "pet_character_armor": pet_stats.map(|stats| stats.armor),
        "pet_character_stamina": pet_stats.map(|stats| stats.primary.stamina)});
    println!("NATIVE_PET_STAT_BASELINE {receipt}");
    assert!(health.max > 0.0);
    assert_eq!(
        world
            .get::<crate::hunter_pet::HunterPet>(pet)
            .unwrap()
            .owner,
        owner
    );
}

#[test]
#[ignore = "native pet HP/armor baseline; requires prepared parity inputs"]
fn dps_parity_pet_stats_baseline_records_actual_native_hunter_pet() {
    let (run, pet) = super::power_tests::prepare_hunter_and_pet();
    record_pet_stats(
        run.app.world(),
        run.actor,
        pet,
        "native253-level90-exact-reference-gear",
    );
}
