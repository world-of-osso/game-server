use super::effects2_tests::use_ability;
use super::*;

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
