use super::effects2_tests::use_ability;
use super::*;

#[test]
fn pet_battle_species_heal_counts_only_other_same_species_team_members() {
    let catalog = catalog();
    for team in 0..2 {
        for active in 0..3 {
            for matching in 0..=2 {
                let mut initial = battle(&catalog, 9, 9, [2260, 110, 123]);
                initial.teams[team].active = active;
                let mut reserves = 0;
                for slot in 0..3 {
                    let pet = &mut initial.teams[team].pets[slot];
                    pet.abilities = [2260, 110, 123];
                    if slot != active {
                        pet.species = if reserves < matching { 39 } else { 378 };
                        reserves += 1;
                    }
                }
                initial.teams[team].pets[active].health = 1000;
                let mut actions = [Action::Pass; 2];
                actions[team] = Action::Ability(2260);
                let (after, events) = initial.step(&catalog, actions, &mut Fixed(500000)).unwrap();
                assert_eq!(after.teams[team].pets[active].health, 1060 + matching * 12);
                for other_team in 0..2 {
                    for slot in 0..3 {
                        if (other_team, slot) != (team, active) {
                            assert_eq!(after.teams[other_team].pets[slot].health, 10000);
                        }
                    }
                }
                assert!(damage(&events).is_empty());
            }
        }
    }
}

#[test]
fn pet_battle_species_heal_uses_roster_identity_not_family_or_alive_filter() {
    let catalog = catalog();
    let mut initial = battle(&catalog, 9, 9, [2260, 110, 123]);
    initial.teams[0].pets[0].health = 1000;
    //Same species still counts when a reserve has fainted, as with Dragon's Call370.
    initial.teams[0].pets[1].health = 0;
    initial.teams[0].pets[1]
        .states
        .insert(catalog.state_id("Is_Dead").unwrap(), 1);
    initial.teams[0].pets[2].species = 378;
    let (after, _) = use_ability(&initial, &catalog, 2260);
    assert_eq!(after.teams[0].pets[0].health, 1072);
    assert_eq!(after.teams[0].pets[1].health, 0);
    assert_eq!(
        after.teams[0].pets[1].state(catalog.state_id("Is_Dead").unwrap()),
        1
    );
    assert_eq!(after.teams[0].pets[2].health, 10000);
}

#[test]
fn pet_battle_species_heal_preserves_accuracy_cap_and_healing_modifiers() {
    let catalog = catalog();
    let mut initial = battle(&catalog, 9, 9, [2260, 110, 123]);
    initial.teams[0].pets[0].health = 1000;
    initial.teams[0].pets[0]
        .states
        .insert(catalog.state_id("Mod_HealingDealtPercent").unwrap(), 50);
    initial.teams[0].pets[0]
        .states
        .insert(catalog.state_id("Mod_HealingTakenPercent").unwrap(), 50);
    let (healed, _) = use_ability(&initial, &catalog, 2260);
    assert_eq!(healed.teams[0].pets[0].health, 1189);
    initial.teams[0].pets[0].health = 9990;
    assert_eq!(
        use_ability(&initial, &catalog, 2260).0.teams[0].pets[0].health,
        10000
    );
    initial.teams[0].pets[0].health = 1000;
    initial.teams[0].pets[0]
        .states
        .insert(catalog.state_id("Stat_Accuracy").unwrap(), -100);
    let (missed, events) = use_ability(&initial, &catalog, 2260);
    assert_eq!(missed.teams[0].pets[0].health, 1000);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Miss { .. }))
    );
}

#[test]
fn pet_battle_species_heal_zero_variance_does_not_draw_critical_or_random_healing() {
    let catalog = catalog();
    let mut initial = battle(&catalog, 9, 9, [2260, 110, 123]);
    initial.teams[0].pets[0].health = 1000;
    initial.teams[0].pets[0].stats.speed = 200;
    let (after, _) = initial
        .step(&catalog, [Action::Ability(2260), Action::Pass], &mut NoDraw)
        .unwrap();
    assert_eq!(after.teams[0].pets[0].health, 1084);
}
