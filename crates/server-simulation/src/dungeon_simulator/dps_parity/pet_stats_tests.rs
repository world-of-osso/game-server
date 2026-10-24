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

#[test]
#[ignore = "native summon final-owner ordering RED; requires prepared parity inputs"]
fn dps_parity_pet_stats_fresh_summon_arrives_full_under_final_owner_stats() {
    let (run, pet) = super::power_tests::prepare_hunter_and_pet();
    let world = run.app.world();
    record_pet_stats(world, run.actor, pet, "fresh-summon-final-owner");
    let health = world.get::<Health>(pet).unwrap();
    assert_eq!(health.max, expected_hunter_max_hp(world, run.actor, pet));
    assert_eq!(
        health.current, health.max,
        "fresh native Call Pet must not keep the pre-owner-stat maximum as current health"
    );
}

#[test]
#[ignore = "native pet owner armor refresh; requires prepared parity inputs"]
fn dps_parity_pet_stats_actual_equipment_removal_refreshes_owner_armor() {
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    let owner_armor = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .armor;
    let equipped = run
        .app
        .world()
        .get::<crate::stats::EquippedItems>(run.actor)
        .unwrap()
        .clone();
    let modifiers = run
        .app
        .world()
        .get::<crate::stats::EquippedItemModifiers>(run.actor)
        .unwrap()
        .clone();
    run.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItems>(run.actor)
        .unwrap()
        .items
        .clear();
    run.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItemModifiers>(run.actor)
        .unwrap()
        .items
        .clear();
    for _ in 0..4 {
        run.app.update();
    }
    let unarmored = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .armor;
    assert!(
        unarmored < owner_armor,
        "removing actual reference gear must lower owner armor"
    );
    let level = f32::from(run.app.world().get::<UnitLevel>(pet).unwrap().0);
    assert_eq!(
        run.app
            .world()
            .get::<crate::level_scaling::CreatureArmor>(pet)
            .unwrap()
            .0,
        (level * 50.0 + unarmored * 0.70).trunc()
    );
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert((equipped, modifiers));
    for _ in 0..4 {
        run.app.update();
    }
    assert_eq!(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .armor,
        owner_armor
    );
    assert_eq!(
        run.app
            .world()
            .get::<crate::level_scaling::CreatureArmor>(pet)
            .unwrap()
            .0,
        (level * 50.0 + owner_armor * 0.70).trunc()
    );
}

fn expected_hunter_max_hp(world: &World, owner: Entity, pet: Entity) -> f32 {
    let data = world.resource::<crate::world_data::WorldData>();
    let entry = world
        .get::<shared::components::Npc>(pet)
        .unwrap()
        .template_id;
    let level = world.get::<UnitLevel>(pet).unwrap().0;
    let template = data.template_for(entry, 0).unwrap();
    let tuning_id = template
        .tuning
        .as_ref()
        .map_or(0, |tuning| tuning.content_tuning_id);
    let (native_health, _) = data.expected_stats.creature_health_armor(
        level,
        template.health_scaling_expansion,
        tuning_id,
        template.unit_class,
    );
    // TC Pet.cpp898–925: absent pet_levelstats uses ExpectedStat, not the
    // imported legacy class-level placeholder health. No fake intrinsic stats.
    let base = (native_health * template.health_modifier).max(1.0).trunc();
    let stamina = world
        .get::<crate::stats::CharacterStats>(owner)
        .unwrap()
        .primary
        .stamina;
    // TC Guardian::UpdateStats1187–1190 truncates the resulting native stat;
    // UpdateMaxHealth1284–1310 adds only bonus stamina ×10 for hunter pets.
    base + (stamina * 0.30).trunc() * 10.0
}

#[test]
#[ignore = "native TC pet inheritance RED; requires prepared parity inputs"]
fn dps_parity_pet_stats_spawn_inherits_native_owner_health_and_armor() {
    let (run, pet) = super::power_tests::prepare_hunter_and_pet();
    let world = run.app.world();
    record_pet_stats(world, run.actor, pet, "spawn-inheritance-before-assert");
    let owner = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap();
    // TC Pet.cpp879 base armor applies before optional pet_levelstats.
    let level = f32::from(world.get::<UnitLevel>(pet).unwrap().0);
    let expected_armor = (level * 50.0 + owner.armor * 0.70).trunc();
    let actual_armor = world
        .get::<crate::level_scaling::CreatureArmor>(pet)
        .map(|armor| armor.0);
    assert_eq!(
        (world.get::<Health>(pet).unwrap().max, actual_armor),
        (
            expected_hunter_max_hp(world, run.actor, pet),
            Some(expected_armor)
        ),
        "TC native owner Stamina30%/×10HP and armor70%; no SimC mixing"
    );
}

#[test]
#[ignore = "native TC pet owner-stat refresh RED; requires prepared parity inputs"]
fn dps_parity_pet_stats_native_owner_stamina_change_refreshes_and_removes() {
    const FORTITUDE: u32 = 21562;
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    let current_hp = run.app.world().get::<Health>(pet).unwrap().current;
    let before = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .primary
        .stamina;
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        FORTITUDE,
        Some(run.actor),
        1,
    );
    for _ in 0..4 {
        run.app.update();
    }
    let boosted = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .primary
        .stamina;
    assert!(
        boosted > before,
        "actual native Fortitude must change owner stamina first"
    );
    record_pet_stats(
        run.app.world(),
        run.actor,
        pet,
        "native-fortitude-owner-change",
    );
    assert_eq!(
        run.app.world().get::<Health>(pet).unwrap().current,
        current_hp,
        "native maximum growth must not heal the pet"
    );
    assert_eq!(
        run.app.world().get::<Health>(pet).unwrap().max,
        expected_hunter_max_hp(run.app.world(), run.actor, pet)
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == FORTITUDE,
    );
    for _ in 0..4 {
        run.app.update();
    }
    assert_eq!(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .primary
            .stamina,
        before
    );
    assert_eq!(
        run.app.world().get::<Health>(pet).unwrap().max,
        expected_hunter_max_hp(run.app.world(), run.actor, pet)
    );
    assert_eq!(
        run.app.world().get::<Health>(pet).unwrap().current,
        current_hp,
        "removal below maximum must preserve current health"
    );
}

#[test]
#[ignore = "native pet foreign owner control; requires prepared parity inputs"]
fn dps_parity_pet_stats_foreign_controller_stats_do_not_change_the_pet_or_dummy() {
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    let controller = run
        .app
        .world()
        .get::<crate::followers::Follower>(run.actor)
        .unwrap()
        .owner;
    let before = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(controller)
        .unwrap()
        .primary
        .stamina;
    let original = run.app.world().get::<Health>(pet).unwrap().max;
    let armor = run
        .app
        .world()
        .get::<crate::level_scaling::CreatureArmor>(pet)
        .unwrap()
        .0;
    let dummy = run.app.world().get::<Health>(run.targets[0]).unwrap().max;
    crate::spell_cast::cast_triggered(run.app.world_mut(), controller, 21562, Some(controller), 1);
    for _ in 0..4 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(controller)
            .unwrap()
            .primary
            .stamina
            > before,
        "real foreign controller stats must change before exclusion assertion"
    );
    assert_eq!(run.app.world().get::<Health>(pet).unwrap().max, original);
    assert_eq!(
        run.app
            .world()
            .get::<crate::level_scaling::CreatureArmor>(pet)
            .unwrap()
            .0,
        armor
    );
    assert_eq!(
        run.app.world().get::<Health>(run.targets[0]).unwrap().max,
        dummy
    );
}
