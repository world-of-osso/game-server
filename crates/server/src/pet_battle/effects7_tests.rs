use super::effects2_tests::use_ability;
use super::*;

#[test]
fn pet_battle_armageddon_hits_each_enemy_then_kills_its_mechanical_caster_without_failsafe() {
    let catalog = catalog();
    let initial = battle(&catalog, 9, 0, [1025, 110, 123]);
    let (state, events) = use_ability(&initial, &catalog, 1025);
    assert_eq!(damage(&events), vec![120, 120, 120]);
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [9880; 3]
    );
    assert_eq!(
        state.teams[0].pets.each_ref().map(|pet| pet.health),
        [0, 10000, 10000]
    );
    assert!(state.teams[0].pets[0].resurrection_used);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Fainted(PetRef { team: 0, slot: 0 })))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Revived(_)))
    );
}

#[test]
fn pet_battle_armageddon_routes_both_teams_and_all_active_slots() {
    let catalog = catalog();
    for team in 0..2 {
        for caller_slot in 0..3 {
            for enemy_slot in 0..3 {
                let mut initial = battle(&catalog, 9, 0, [1025, 110, 123]);
                initial.teams[0].active = caller_slot;
                initial.teams[1].active = enemy_slot;
                if team == 1 {
                    initial.teams.swap(0, 1);
                }
                let mut actions = [Action::Pass; 2];
                actions[team] = Action::Ability(1025);
                let (state, _) = initial
                    .step(&catalog, actions, &mut Fixed(500_000))
                    .unwrap();
                assert_eq!(
                    state.teams[1 - team].pets.each_ref().map(|pet| pet.health),
                    [9880; 3]
                );
                for (slot, pet) in state.teams[team].pets.iter().enumerate() {
                    assert_eq!(pet.health, if slot == caller_slot { 0 } else { 10000 });
                }
                assert!(state.teams[team].pets[caller_slot].resurrection_used);
            }
        }
    }
}

#[test]
fn pet_battle_armageddon_uses_each_enemy_family_without_sharing_one_hit() {
    let catalog = catalog();
    let mut initial = battle(&catalog, 9, 0, [1025, 110, 123]);
    let stats = initial.teams[1].pets[0].stats;
    initial.teams[1].pets = [0, 7, 6].map(|family| {
        Pet::new(&catalog, 39, stats, family, [110, 119, 123], States::new()).unwrap()
    });
    let (state, events) = use_ability(&initial, &catalog, 1025);
    assert_eq!(damage(&events), vec![120, 180, 80]);
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [9880, 9820, 9920]
    );
}

#[test]
fn pet_battle_armageddon_misses_do_not_cancel_the_casters_sacrifice() {
    let catalog = catalog();
    let mut initial = battle(&catalog, 9, 0, [1025, 110, 123]);
    for pet in &mut initial.teams[1].pets {
        pet.states
            .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
    }
    let (state, events) = use_ability(&initial, &catalog, 1025);
    assert!(damage(&events).is_empty());
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::Miss { .. }))
            .count(),
        3
    );
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [10000; 3]
    );
    assert_eq!(state.teams[0].pets[0].health, 0);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Revived(_)))
    );
}

#[test]
fn pet_battle_armageddon_skips_dead_enemy_reserves_and_preserves_living_allies() {
    let catalog = catalog();
    let mut initial = battle(&catalog, 9, 0, [1025, 110, 123]);
    initial.teams[1].pets[1].health = 0;
    initial.teams[1].pets[1]
        .states
        .insert(catalog.state_id("Is_Dead").unwrap(), 1);
    let (state, events) = use_ability(&initial, &catalog, 1025);
    assert_eq!(damage(&events), vec![120, 120]);
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [9880, 0, 9880]
    );
    assert_eq!(
        state.teams[0].pets.each_ref().map(|pet| pet.health),
        [0, 10000, 10000]
    );
}

#[test]
fn pet_battle_armageddon_finishes_its_self_kill_after_the_last_enemy_faints() {
    let catalog = catalog();
    let mut initial = battle(&catalog, 9, 0, [1025, 110, 123]);
    for pet in &mut initial.teams[1].pets {
        pet.health = 1;
    }
    let (state, events) = use_ability(&initial, &catalog, 1025);
    assert_eq!(damage(&events), vec![1, 1, 1]);
    assert_eq!(state.teams[0].pets[0].health, 0);
    assert_eq!(state.outcome, Some(Outcome::Winner(0)));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Revived(_)))
    );
}

#[test]
fn pet_battle_armageddon_unqualified_reverse_kill_rejects_before_rng() {
    struct NoDraw;
    impl BattleRng for NoDraw {
        fn draw(&mut self) -> u32 {
            panic!("unqualified Armageddon kill must reject before RNG");
        }
    }
    let mut catalog = catalog();
    catalog.effects.get_mut(&2330).unwrap().params[3] = 1;
    let initial = battle(&catalog, 9, 0, [1025, 110, 123]);
    assert!(
        initial
            .step(&catalog, [Action::Ability(1025), Action::Pass], &mut NoDraw)
            .is_err()
    );
}
