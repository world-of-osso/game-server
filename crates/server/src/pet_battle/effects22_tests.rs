use super::effects2_tests::use_ability;
use super::*;

#[test]
fn pet_battle_current_health_torment_hits_each_enemy_once_and_leeches_actual_damage() {
    let catalog = catalog();
    for ally_active in 0..3 {
        for enemy_active in 0..3 {
            let mut initial = battle(&catalog, 9, 9, [2263, 110, 123]);
            initial.teams[0].active = ally_active;
            initial.teams[1].active = enemy_active;
            initial.teams[0].pets[ally_active].health = 1000;
            for (pet, health) in initial.teams[1].pets.iter_mut().zip([1001, 501, 101]) {
                pet.health = health;
            }
            let (after, events) = use_ability(&initial, &catalog, 2263);
            assert_eq!(
                after.teams[1].pets.each_ref().map(|pet| pet.health),
                [801, 401, 81]
            );
            assert_eq!(after.teams[0].pets[ally_active].health, 1320);
            for slot in 0..3 {
                if slot != ally_active {
                    assert_eq!(after.teams[0].pets[slot].health, 10000);
                }
            }
            assert_eq!(damage(&events).iter().sum::<i32>(), 320);
            assert_eq!(damage(&events).len(), 3);
        }
    }
}

#[test]
fn pet_battle_current_health_torment_does_not_repeat_previous_leech_on_dead_or_missed_pet() {
    let catalog = catalog();
    for unavailable in 0..3 {
        for dead in [false, true] {
            let mut initial = battle(&catalog, 9, 9, [2263, 110, 123]);
            initial.teams[0].pets[0].health = 1000;
            // Keep a live active pet while each reserve is tested dead.
            initial.teams[1].active = (unavailable + 1) % 3;
            if dead {
                initial.teams[1].pets[unavailable].health = 0;
                initial.teams[1].pets[unavailable]
                    .states
                    .insert(catalog.state_id("Is_Dead").unwrap(), 1);
            } else {
                initial.teams[1].pets[unavailable]
                    .states
                    .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
            }
            let (after, events) = use_ability(&initial, &catalog, 2263);
            assert_eq!(damage(&events), [2000, 2000]);
            assert_eq!(after.teams[0].pets[0].health, 5000);
            assert_eq!(
                after.teams[1].pets[unavailable].health,
                if dead { 0 } else { 10000 }
            );
        }
    }
}

#[test]
fn pet_battle_current_health_torment_keeps_existing_hostile_percentage_type_convention() {
    let catalog = catalog();
    // Mechanical Torment is strong against Beast. Policy, not independent Retail proof.
    let mut initial = battle(&catalog, 9, 7, [2263, 110, 123]);
    initial.teams[0].pets[0].health = 1000;
    for pet in &mut initial.teams[1].pets {
        pet.health = 1001;
    }
    let (after, events) = use_ability(&initial, &catalog, 2263);
    assert_eq!(damage(&events), [300; 3]);
    assert_eq!(after.teams[0].pets[0].health, 1900);
    assert_eq!(
        after.teams[1].pets.each_ref().map(|pet| pet.health),
        [701; 3]
    );
}

#[test]
fn pet_battle_current_health_mind_games_costs_each_friendly_slot_without_family_scaling() {
    let catalog = catalog();
    for active in 0..3 {
        let mut initial = battle(&catalog, 9, 9, [2388, 110, 123]);
        initial.teams[0].active = active;
        for (pet, health) in initial.teams[0].pets.iter_mut().zip([1001, 501, 101]) {
            pet.health = health;
        }
        let (after, events) = use_ability(&initial, &catalog, 2388);
        assert_eq!(
            after.teams[0].pets.each_ref().map(|pet| pet.health),
            [751, 376, 76]
        );
        assert_eq!(
            after.teams[1].pets.each_ref().map(|pet| pet.health),
            [10000; 3]
        );
        assert_eq!(damage(&events).iter().sum::<i32>(), 400);
        assert_eq!(damage(&events).len(), 3);
    }
}

#[test]
fn pet_battle_current_health_torment_selector_variant_remains_explicit_gap() {
    let catalog = catalog();
    let initial = battle(&catalog, 9, 9, [2292, 110, 123]);
    let error = initial
        .step(&catalog, [Action::Ability(2292), Action::Pass], &mut NoDraw)
        .unwrap_err();
    assert!(error.contains("unsupported property 156"), "{error}");
}
