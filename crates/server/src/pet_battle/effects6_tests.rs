use super::effects2_tests::use_ability;
use super::*;

fn create_refuge_battle(catalog: &Catalog) -> Battle {
    let mut state = battle(catalog, 6, 7, [1344, 110, 123]);
    let stats = state.teams[0].pets[0].stats;
    state.teams[0].pets = [(1430, 6), (1427, 4), (39, 9)].map(|(species, family)| {
        Pet::new(
            catalog,
            species,
            stats,
            family,
            [1344, 110, 123],
            States::new(),
        )
        .unwrap()
    });
    state
}

#[test]
fn pet_battle_refuge_shields_caster_and_only_critter_reserves_independently() {
    let catalog = catalog();
    let initial = create_refuge_battle(&catalog);
    let (state, _) = use_ability(&initial, &catalog, 1344);
    let targets: Vec<_> = state.auras.iter().map(|aura| aura.target).collect();
    assert_eq!(
        targets,
        vec![PetRef { team: 0, slot: 0 }, PetRef { team: 0, slot: 1 }]
    );
    for aura in &state.auras {
        assert_eq!(aura.ability, 759);
        assert_eq!(aura.charges, 1);
        assert_eq!(aura.remaining, 6);
        assert!(!aura.team_bound);
        assert_eq!(aura.caster, PetRef { team: 0, slot: 0 });
    }
    let (state, events) = state
        .step(
            &catalog,
            [Action::Pass, Action::Ability(110)],
            &mut Fixed(500_000),
        )
        .unwrap();
    assert!(damage(&events).is_empty());
    assert_eq!(state.teams[0].pets[0].health, 10000);
    assert_eq!(state.auras.len(), 1);
    assert_eq!(state.auras[0].target, PetRef { team: 0, slot: 1 });
    let (state, events) = state
        .step(
            &catalog,
            [Action::Swap(1), Action::Ability(110)],
            &mut Fixed(500_000),
        )
        .unwrap();
    assert!(damage(&events).is_empty());
    assert_eq!(state.teams[0].pets[1].health, 10000);
    assert!(state.auras.is_empty());
    let (state, events) = state
        .step(
            &catalog,
            [Action::Swap(2), Action::Ability(110)],
            &mut Fixed(500_000),
        )
        .unwrap();
    assert_eq!(damage(&events), vec![120]);
    assert_eq!(state.teams[0].pets[2].health, 9880);
    assert!(state.teams[1].pets.iter().all(|pet| pet.health == 10000));
}

#[test]
fn pet_battle_refuge_routes_other_friendly_slots_for_both_teams_and_every_active_slot() {
    let catalog = catalog();
    for team in 0..2 {
        for active in 0..3 {
            let mut initial = create_refuge_battle(&catalog);
            initial.teams[0].pets.rotate_right(active);
            initial.teams[0].active = active;
            let stats = initial.teams[1].pets[0].stats;
            let enemy = Pet::new(&catalog, 1427, stats, 4, [110, 119, 123], States::new()).unwrap();
            initial.teams[1].pets = [enemy.clone(), enemy.clone(), enemy];
            if team == 1 {
                initial.teams.swap(0, 1);
            }
            let caster = initial.active(team);
            let critter_slot = initial.teams[team]
                .pets
                .iter()
                .position(|pet| pet.family == 4)
                .unwrap();
            let mut actions = [Action::Pass; 2];
            actions[team] = Action::Ability(1344);
            let (state, _) = initial
                .step(&catalog, actions, &mut Fixed(500_000))
                .unwrap();
            let targets: std::collections::BTreeSet<_> =
                state.auras.iter().map(|aura| aura.target).collect();
            assert_eq!(
                targets,
                std::collections::BTreeSet::from([
                    caster,
                    PetRef {
                        team,
                        slot: critter_slot
                    }
                ])
            );
            for aura in &state.auras {
                assert_eq!(aura.caster, caster);
                assert_eq!(aura.remaining, 6);
                assert_eq!(aura.charges, 1);
                assert!(!aura.team_bound);
            }
        }
    }
}

#[test]
fn pet_battle_refuge_expires_each_unused_shield_after_six_subsequent_rounds() {
    let catalog = catalog();
    let initial = create_refuge_battle(&catalog);
    let mut state = use_ability(&initial, &catalog, 1344).0;
    for elapsed in 1..6 {
        state = state
            .step(&catalog, [Action::Pass; 2], &mut Fixed(500_000))
            .unwrap()
            .0;
        assert_eq!(state.auras.len(), 2);
        assert!(state.auras.iter().all(|aura| aura.remaining == 6 - elapsed));
    }
    state = state
        .step(&catalog, [Action::Pass; 2], &mut Fixed(500_000))
        .unwrap()
        .0;
    assert!(state.auras.is_empty());
    let blocked = catalog.state_id("Special_BlockedAttackCount").unwrap();
    assert!(
        state.teams[0]
            .pets
            .iter()
            .all(|pet| pet.state(blocked) == 0)
    );
}

#[test]
fn pet_battle_refuge_other_caster_refreshes_without_stacking_and_preserves_source() {
    let catalog = catalog();
    let mut initial = create_refuge_battle(&catalog);
    initial.teams[0].pets[0] = Pet::new(
        &catalog,
        1427,
        initial.teams[0].pets[0].stats,
        4,
        [1344, 110, 123],
        States::new(),
    )
    .unwrap();
    let state = use_ability(&initial, &catalog, 1344).0;
    let state = state
        .step(
            &catalog,
            [Action::Swap(1), Action::Pass],
            &mut Fixed(500_000),
        )
        .unwrap()
        .0;
    let state = use_ability(&state, &catalog, 1344).0;
    assert_eq!(state.auras.len(), 2);
    let blocked = catalog.state_id("Special_BlockedAttackCount").unwrap();
    for aura in &state.auras {
        assert_eq!(aura.caster, PetRef { team: 0, slot: 1 });
        assert_eq!(aura.charges, 1);
        assert_eq!(aura.remaining, 6);
        assert_eq!(state.pet(aura.target).state(blocked), 1);
    }
}

#[test]
fn pet_battle_refuge_does_not_apply_to_or_revive_a_fainted_critter() {
    let catalog = catalog();
    let mut initial = create_refuge_battle(&catalog);
    initial.teams[0].pets[1].health = 0;
    initial.teams[0].pets[1]
        .states
        .insert(catalog.state_id("Is_Dead").unwrap(), 1);
    let (state, events) = use_ability(&initial, &catalog, 1344);
    assert_eq!(state.auras.len(), 1);
    assert_eq!(state.auras[0].target, PetRef { team: 0, slot: 0 });
    assert_eq!(state.teams[0].pets[1].health, 0);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Revived(_)))
    );
}

#[test]
fn pet_battle_refuge_keeps_its_charge_through_periodic_damage() {
    let catalog = catalog();
    let mut initial = create_refuge_battle(&catalog);
    initial.teams[1].pets[0].abilities[0] = 152;
    let poisoned = initial
        .step(
            &catalog,
            [Action::Pass, Action::Ability(152)],
            &mut Fixed(500_000),
        )
        .unwrap()
        .0;
    let (state, events) = use_ability(&poisoned, &catalog, 1344);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Damage { periodic: true, .. }))
    );
    assert!(state.teams[0].pets[0].health < poisoned.teams[0].pets[0].health);
    let shield = state
        .auras
        .iter()
        .find(|aura| aura.ability == 759 && aura.target == PetRef { team: 0, slot: 0 })
        .unwrap();
    assert_eq!(shield.charges, 1);
}

#[test]
fn pet_battle_refuge_does_not_block_or_consume_charges_for_outgoing_attacks() {
    let catalog = catalog();
    let initial = create_refuge_battle(&catalog);
    let state = use_ability(&initial, &catalog, 1344).0;
    let (state, events) = use_ability(&state, &catalog, 110);
    assert_eq!(damage(&events), vec![120]);
    assert_eq!(state.auras.len(), 2);
    assert!(state.auras.iter().all(|aura| aura.charges == 1));
}

#[test]
fn pet_battle_refuge_unknown_family_selector_rejects_before_rng() {
    struct NoDraw;
    impl BattleRng for NoDraw {
        fn draw(&mut self) -> u32 {
            panic!("unestablished223 selector must reject before RNG");
        }
    }
    let mut catalog = catalog();
    catalog.effects.get_mut(&2723).unwrap().params[3] = 6;
    let initial = create_refuge_battle(&catalog);
    assert!(
        initial
            .step(&catalog, [Action::Ability(1344), Action::Pass], &mut NoDraw)
            .is_err()
    );
}
