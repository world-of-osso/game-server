use super::effects2_tests::use_ability;
use super::*;

#[test]
fn pet_battle_lock_on_marks_the_enemy_then_launches_and_requires_a_new_lock() {
    let catalog = catalog();
    let initial = battle(&catalog, 9, 0, [301, 110, 123]);
    let (ready, events) = use_ability(&initial, &catalog, 301);
    assert!(damage(&events).is_empty());
    assert_eq!(ready.teams[1].pets[0].health, 10000);
    assert_eq!(ready.auras.len(), 1);
    assert_eq!(ready.auras[0].ability, 300);
    assert_eq!(ready.auras[0].target, PetRef { team: 1, slot: 0 });
    assert_eq!(ready.auras[0].remaining, 1);
    let (launched, events) = use_ability(&ready, &catalog, 301);
    assert_eq!(damage(&events), vec![270]);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Damage {
            periodic: false,
            ..
        }
    )));
    assert_eq!(launched.teams[1].pets[0].health, 9730);
    assert_eq!(launched.teams[0].pets[0].health, 10000);
    assert!(launched.auras.is_empty());
    let (ready_again, events) = use_ability(&launched, &catalog, 301);
    assert!(damage(&events).is_empty());
    assert_eq!(ready_again.auras.len(), 1);
    assert_eq!(ready_again.teams[1].pets[0].health, 9730);
}

#[test]
fn pet_battle_lock_on_expires_after_an_unused_round() {
    let catalog = catalog();
    let initial = battle(&catalog, 9, 0, [301, 110, 123]);
    let (ready, _) = use_ability(&initial, &catalog, 301);
    let (expired, _) = ready
        .step(&catalog, [Action::Pass; 2], &mut Fixed(500_000))
        .unwrap();
    assert!(expired.auras.is_empty());
    let (ready_again, events) = use_ability(&expired, &catalog, 301);
    assert!(damage(&events).is_empty());
    assert_eq!(ready_again.auras.len(), 1);
    assert_eq!(ready_again.teams[1].pets[0].health, 10000);
}

#[test]
fn pet_battle_lock_on_marks_a_new_enemy_instead_of_shooting_the_old_target() {
    let catalog = catalog();
    let initial = battle(&catalog, 9, 0, [301, 110, 123]);
    let (ready, _) = use_ability(&initial, &catalog, 301);
    let (retargeted, events) = ready
        .step(
            &catalog,
            [Action::Ability(301), Action::Swap(1)],
            &mut Fixed(500_000),
        )
        .unwrap();
    assert!(damage(&events).is_empty());
    assert_eq!(retargeted.auras.len(), 1);
    assert_eq!(retargeted.auras[0].target, PetRef { team: 1, slot: 1 });
    let (launched, events) = use_ability(&retargeted, &catalog, 301);
    assert_eq!(damage(&events), vec![270]);
    assert_eq!(launched.teams[1].pets[0].health, 10000);
    assert_eq!(launched.teams[1].pets[1].health, 9730);
    assert!(launched.auras.is_empty());
}

#[test]
fn pet_battle_lock_on_missed_launch_consumes_the_target_mark() {
    let catalog = catalog();
    let initial = battle(&catalog, 9, 0, [301, 110, 123]);
    let (mut ready, _) = use_ability(&initial, &catalog, 301);
    ready.teams[1].pets[0]
        .states
        .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
    let (launched, events) = use_ability(&ready, &catalog, 301);
    assert!(damage(&events).is_empty());
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Miss { effect: 517, .. }))
    );
    assert_eq!(launched.teams[1].pets[0].health, 10000);
    assert!(launched.auras.is_empty());
}

#[test]
fn pet_battle_lock_on_setup_obeys_enemy_dodge() {
    let catalog = catalog();
    let mut initial = battle(&catalog, 9, 0, [301, 110, 123]);
    initial.teams[1].pets[0]
        .states
        .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
    let (state, events) = use_ability(&initial, &catalog, 301);
    assert!(damage(&events).is_empty());
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Miss { effect: 517, .. }))
    );
    assert!(state.auras.is_empty());
}

#[test]
fn pet_battle_lock_on_rejects_unqualified_duration_before_rng() {
    struct NoDraw;
    impl BattleRng for NoDraw {
        fn draw(&mut self) -> u32 {
            panic!("unqualified Lock-On must reject before RNG");
        }
    }
    let mut catalog = catalog();
    catalog.effects.get_mut(&517).unwrap().params[2] = 2;
    let initial = battle(&catalog, 9, 0, [301, 110, 123]);
    assert!(
        initial
            .step(&catalog, [Action::Ability(301), Action::Pass], &mut NoDraw)
            .is_err()
    );
}

#[test]
fn pet_battle_life_exchange_equalizes_active_health_without_touching_reserves() {
    let catalog = catalog();
    let mut initial = battle(&catalog, 9, 4, [277, 110, 123]);
    initial.teams[0].pets[0].health = 2000;
    initial.teams[1].pets[0].health = 6000;
    let (state, _) = use_ability(&initial, &catalog, 277);
    assert_eq!(state.teams[0].pets[0].health, 4000);
    assert_eq!(state.teams[1].pets[0].health, 4000);
    for team in 0..2 {
        for slot in 1..3 {
            assert_eq!(state.teams[team].pets[slot].health, 10000);
        }
    }
}

#[test]
fn pet_battle_life_exchange_caps_health_gained_at_the_casters_maximum() {
    let catalog = catalog();
    let mut initial = battle(&catalog, 9, 4, [277, 110, 123]);
    let caller = &mut initial.teams[0].pets[0];
    caller.stats.health = 3000;
    caller.base_health = 3000;
    caller.health = 1000;
    initial.teams[1].pets[0].health = 9000;
    let (state, _) = use_ability(&initial, &catalog, 277);
    assert_eq!(state.teams[0].pets[0].health, 3000);
    assert_eq!(state.teams[1].pets[0].health, 5000);
}

#[test]
fn pet_battle_life_exchange_miss_preserves_both_health_values() {
    let catalog = catalog();
    let mut initial = battle(&catalog, 9, 4, [277, 110, 123]);
    initial.teams[0].pets[0].health = 2000;
    initial.teams[1].pets[0].health = 6000;
    initial.teams[1].pets[0]
        .states
        .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
    let (state, events) = use_ability(&initial, &catalog, 277);
    assert_eq!(state.teams[0].pets[0].health, 2000);
    assert_eq!(state.teams[1].pets[0].health, 6000);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Miss { effect: 493, .. }))
    );
}

#[test]
fn pet_battle_life_exchange_rejects_unqualified_parameters_before_rng() {
    struct NoDraw;
    impl BattleRng for NoDraw {
        fn draw(&mut self) -> u32 {
            panic!("unqualified Life Exchange must reject before RNG");
        }
    }
    let mut catalog = catalog();
    catalog.effects.get_mut(&493).unwrap().params[1] = 100;
    let initial = battle(&catalog, 9, 4, [277, 110, 123]);
    assert!(
        initial
            .step(&catalog, [Action::Ability(277), Action::Pass], &mut NoDraw)
            .is_err()
    );
}
