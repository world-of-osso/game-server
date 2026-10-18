use super::effects2_tests::use_ability;
use super::*;

#[test]
fn pet_battle_bonestorm_hits_each_enemy_and_charges_only_the_actual_caster() {
    let catalog = catalog();
    for ally_active in 0..3 {
        for enemy_active in 0..3 {
            let mut initial = battle(&catalog, 3, 7, [649, 110, 123]);
            initial.teams[0].active = ally_active;
            initial.teams[1].active = enemy_active;
            let (state, events) = use_ability(&initial, &catalog, 649);
            assert_eq!(damage(&events), [78, 78, 78, 1000]);
            assert_eq!(
                state.teams[1].pets.each_ref().map(|pet| pet.health),
                [9922; 3]
            );
            for slot in 0..3 {
                assert_eq!(
                    state.teams[0].pets[slot].health,
                    if slot == ally_active { 9000 } else { 10000 }
                );
            }
            let caster = PetRef {
                team: 0,
                slot: ally_active,
            };
            assert!(events.iter().any(|event| matches!(event, Event::Damage { caster: source, target, amount: 1000, .. } if *source == caster && *target == caster)));
        }
    }
}

#[test]
fn pet_battle_bonestorm_individual_misses_do_not_cancel_other_hits_or_the_cost() {
    let catalog = catalog();
    for active in 0..3 {
        for missed in 0..3 {
            let mut initial = battle(&catalog, 3, 7, [649, 110, 123]);
            initial.teams[0].active = 2;
            initial.teams[1].active = active;
            initial.teams[1].pets[missed]
                .states
                .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
            let (state, events) = use_ability(&initial, &catalog, 649);
            assert_eq!(damage(&events), [78, 78, 1000]);
            for slot in 0..3 {
                assert_eq!(
                    state.teams[1].pets[slot].health,
                    if slot == missed { 10000 } else { 9922 }
                );
            }
            assert_eq!(state.teams[0].pets[2].health, 9000);
            assert_eq!(state.teams[0].pets[0].health, 10000);
            assert_eq!(state.teams[0].pets[1].health, 10000);
        }
    }
}

#[test]
fn pet_battle_bonestorm_keeps_dead_reserves_dead_and_still_charges_the_caster() {
    let catalog = catalog();
    let mut initial = battle(&catalog, 3, 7, [649, 110, 123]);
    initial.teams[1].active = 1;
    initial.teams[1].pets[0].health = 0;
    initial.teams[1].pets[0]
        .states
        .insert(catalog.state_id("Is_Dead").unwrap(), 1);
    let (state, events) = use_ability(&initial, &catalog, 649);
    assert_eq!(damage(&events), [78, 78, 1000]);
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [0, 9922, 9922]
    );
    assert_eq!(
        state.teams[0].pets.each_ref().map(|pet| pet.health),
        [9000, 10000, 10000]
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Revived(_)))
    );
}

#[test]
fn pet_battle_bonestorm_cost_uses_caster_maximum_not_current_or_enemy_health() {
    let catalog = catalog();
    for enemy_family in [0, 8] {
        for misses in [false, true] {
            let mut initial = battle(&catalog, 3, enemy_family, [649, 110, 123]);
            initial.teams[0].pets[0].stats.health = 10003;
            initial.teams[0].pets[0].health = 3000;
            for pet in &mut initial.teams[1].pets {
                pet.stats.health = 4711;
                pet.health = 4711;
                if misses {
                    pet.states
                        .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
                }
            }
            let (state, events) = use_ability(&initial, &catalog, 649);
            // floor(10003 * 10%) = 1000, independent of the enemy's type or evasion.
            assert_eq!(state.teams[0].pets[0].health, 2000);
            assert_eq!(state.teams[0].pets[0].stats.health, 10003);
            assert_eq!(damage(&events).last(), Some(&1000));
            let dealt = if misses {
                0
            } else if enemy_family == 0 {
                117
            } else {
                52
            };
            assert_eq!(
                state.teams[1].pets.each_ref().map(|pet| pet.health),
                [4711 - dealt; 3]
            );
            assert_eq!(state.teams[0].pets[1].health, 10000);
            assert_eq!(state.teams[0].pets[2].health, 10000);
        }
    }
}
