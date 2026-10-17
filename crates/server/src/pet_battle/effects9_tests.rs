use super::effects2_tests::use_ability;
use super::*;

fn level_health_battle(catalog: &Catalog, ability: u32, level: i32) -> Battle {
    let mut initial = battle(catalog, 9, 4, [ability, 404, 123]);
    initial.teams[0].pets[0]
        .states
        .insert(catalog.state_id("Internal_InitialLevel").unwrap(), level);
    initial
}

#[test]
fn pet_battle_level_health_bonus_uses_authored_points_per_level_before_healing() {
    let catalog = catalog();
    for (ability, aura, points, healing) in
        [(576, 577, 5, 28), (611, 610, 5, 28), (1991, 1990, 10, 35)]
    {
        for level in [1, 7, 25] {
            let initial = level_health_battle(&catalog, ability, level);
            let (state, _) = use_ability(&initial, &catalog, ability);
            let bonus = points * level;
            assert_eq!(state.teams[0].pets[0].stats.health, 10000 + bonus);
            assert_eq!(
                state.teams[0].pets[0].health,
                10000 + bonus.min(healing * 6)
            );
            assert_eq!(state.auras[0].ability, aura);
            assert_eq!(state.teams[0].pets[1].stats.health, 10000);
            assert_eq!(state.teams[1].pets[0].stats.health, 10000);
        }
    }
}

#[test]
fn pet_battle_level_health_bonus_refresh_does_not_stack_maximum_health() {
    let catalog = catalog();
    let initial = level_health_battle(&catalog, 576, 7);
    let (mut state, _) = use_ability(&initial, &catalog, 576);
    for _ in 0..3 {
        state = state
            .step(&catalog, [Action::Pass; 2], &mut Fixed(500_000))
            .unwrap()
            .0;
    }
    let (state, _) = use_ability(&state, &catalog, 576);
    assert_eq!(state.teams[0].pets[0].stats.health, 10035);
    assert_eq!(state.teams[0].pets[0].health, 10035);
    assert_eq!(state.auras.len(), 1);
    assert_eq!(state.auras[0].remaining, 9);
}

#[test]
fn pet_battle_level_health_bonus_expiry_removes_capacity_and_caps_current_health() {
    let catalog = catalog();
    for (ability, duration) in [(576, 9), (611, 9), (1991, 5)] {
        let initial = level_health_battle(&catalog, ability, 7);
        let (mut state, _) = use_ability(&initial, &catalog, ability);
        for _ in 0..duration - 1 {
            state = state
                .step(&catalog, [Action::Pass; 2], &mut Fixed(500_000))
                .unwrap()
                .0;
        }
        assert!(state.teams[0].pets[0].stats.health > 10000);
        let (state, _) = state
            .step(&catalog, [Action::Pass; 2], &mut Fixed(500_000))
            .unwrap();
        assert_eq!(state.teams[0].pets[0].stats.health, 10000);
        assert_eq!(state.teams[0].pets[0].health, 10000);
        assert!(state.auras.is_empty());
    }
}

#[test]
fn pet_battle_level_health_bonus_preserves_existing_sunlight_percent_convention() {
    let catalog = catalog();
    for order in [[404, 576], [576, 404]] {
        let mut state = level_health_battle(&catalog, 576, 7);
        for ability in order {
            state = use_ability(&state, &catalog, ability).0;
        }
        assert_eq!(state.teams[0].pets[0].stats.health, 15052);
        assert_eq!(state.teams[0].pets[1].stats.health, 15000);
        assert_eq!(state.teams[1].pets[0].stats.health, 15000);
    }
}

#[test]
fn pet_battle_level_health_bonus_rejects_unqualified_aura_values_before_rng() {
    struct NoDraw;
    impl BattleRng for NoDraw {
        fn draw(&mut self) -> u32 {
            panic!("unqualified health bonus must reject before RNG");
        }
    }
    let mut catalog = catalog();
    catalog.ability_states.get_mut(&577).unwrap().insert(2, 501);
    let initial = level_health_battle(&catalog, 576, 7);
    assert!(
        initial
            .step(&catalog, [Action::Ability(576), Action::Pass], &mut NoDraw)
            .is_err()
    );
}
