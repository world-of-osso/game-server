use super::*;
use shared::pet_battle::{PetJournal, PetQuality};

struct Fixed(u32);
impl BattleRng for Fixed {
    fn draw(&mut self) -> u32 {
        self.0
    }
}
pub(super) fn catalog() -> WildCatalog {
    let db = Connection::open_with_flags(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/world.db"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    WildCatalog::load(&db).unwrap()
}
fn journal() -> PetJournal {
    let mut journal = PetJournal::default();
    for id in 41..44 {
        journal
            .add_with_guid(39, 1, PetQuality::Rare, 3, id)
            .unwrap();
    }
    journal
}
fn owned_states(catalog: &WildCatalog, journal: &PetJournal) -> OwnedBattleStates {
    journal
        .pets
        .iter()
        .map(|pet| {
            (
                pet.id,
                crate::pet_journal_state::OwnedPetBattleState::initial(&catalog.turns, pet)
                    .unwrap(),
            )
        })
        .collect()
}

#[test]
fn wild_pet_battles_magma_wave_uses_legal_molten_hatchling_level_four_option() {
    let catalog = catalog();
    let mut journal = PetJournal::default();
    for (guid, species) in [(41, 428), (42, 39), (43, 39)] {
        journal
            .add_with_guid(species, 4, PetQuality::Rare, 3, guid)
            .unwrap();
    }
    let states = owned_states(&catalog, &journal);
    assert_eq!(states[&41].selected_abilities[2], 319);
    let encounter = Encounter::start(&catalog, &journal, &states, 378, 1, 1, 1).unwrap();
    assert_eq!(encounter.battle.teams[0].pets[0].species, 428);
    assert_eq!(encounter.battle.teams[0].pets[0].stats.power, 47);
    let (state, events) = encounter
        .battle
        .step(
            &catalog.turns,
            [Action::Ability(319), Action::Pass],
            &mut Fixed(500_000),
        )
        .unwrap();
    let amounts: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Damage { amount, .. } => Some(*amount),
            _ => None,
        })
        .collect();
    // Elemental319 is weak against Critter Rabbit: floor(23 * .67) = 15.
    assert_eq!(amounts, [15]);
    assert_eq!(state.teams[1].pets[0].health, 132);
    for slot in 0..3 {
        assert_eq!(
            state.teams[0].pets[slot].health,
            encounter.battle.teams[0].pets[slot].health
        );
    }
}

#[test]
fn wild_pet_battles_primal_cry_uses_legal_meadowstomper_level_two_pair() {
    let catalog = catalog();
    let mut journal = PetJournal::default();
    for (guid, species) in [(41, 1446), (42, 39), (43, 39)] {
        journal
            .add_with_guid(species, 2, PetQuality::Rare, 3, guid)
            .unwrap();
    }
    let states = owned_states(&catalog, &journal);
    assert_eq!(states[&41].selected_abilities[1], 920);
    let encounter = Encounter::start(&catalog, &journal, &states, 378, 1, 1, 1).unwrap();
    assert_eq!(encounter.battle.teams[0].pets[0].species, 1446);
    assert_eq!(encounter.battle.teams[0].pets[0].stats.power, 22);
    let (state, events) = encounter
        .battle
        .step(
            &catalog.turns,
            [Action::Ability(920), Action::Pass],
            &mut Fixed(500_000),
        )
        .unwrap();
    let damage: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Damage { amount, .. } => Some(*amount),
            _ => None,
        })
        .collect();
    assert_eq!(damage, [31]);
    assert_eq!(state.teams[1].pets[0].health, 116);
    assert_eq!(
        state.teams[1].pets[0].state(catalog.turns.state_id("Mod_SpeedPercent").unwrap()),
        -25
    );
    assert_eq!(state.auras.len(), 1);
    assert_eq!(state.auras[0].ability, 154);
    assert_eq!(
        state.teams[0].pets[1].health,
        encounter.battle.teams[0].pets[1].health
    );
    assert_eq!(
        state.teams[0].pets[2].health,
        encounter.battle.teams[0].pets[2].health
    );
}

#[test]
fn wild_pet_battles_rapid_fire_uses_legal_peddlefeet_level_ten_two_wave_loadout() {
    let catalog = catalog();
    let mut journal = PetJournal::default();
    for (guid, species) in [(41, 122), (42, 39), (43, 39)] {
        journal
            .add_with_guid(species, 10, PetQuality::Rare, 3, guid)
            .unwrap();
    }
    let mut states = owned_states(&catalog, &journal);
    states
        .get_mut(&41)
        .unwrap()
        .select_ability(&catalog.turns, journal.get(41).unwrap(), 1, 774)
        .unwrap();
    let encounter = Encounter::start(&catalog, &journal, &states, 378, 1, 1, 1).unwrap();
    assert_eq!(encounter.battle.teams[0].pets[0].species, 122);
    assert_eq!(encounter.battle.teams[0].pets[0].stats.power, 111);
    let (state, events) = encounter
        .battle
        .step(
            &catalog.turns,
            [Action::Ability(774), Action::Pass],
            &mut Fixed(500_000),
        )
        .unwrap();
    let damage: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Damage { amount, .. } => Some(*amount),
            _ => None,
        })
        .collect();
    assert_eq!(damage, [26, 19]);
    assert_eq!(state.teams[1].pets[0].health, 102);
    assert_eq!(
        state.teams[0].pets[1].health,
        encounter.battle.teams[0].pets[1].health
    );
    assert_eq!(
        state.teams[0].pets[2].health,
        encounter.battle.teams[0].pets[2].health
    );
}

#[test]
fn wild_pet_battles_starfall_uses_legal_level_one_cosmos_and_heals_owned_reserves() {
    let catalog = catalog();
    let mut journal = PetJournal::default();
    for (guid, species) in [(41, 1972), (42, 39), (43, 39)] {
        journal
            .add_with_guid(species, 1, PetQuality::Rare, 3, guid)
            .unwrap();
    }
    let mut states = owned_states(&catalog, &journal);
    for state in states.values_mut() {
        state.health = 100;
    }
    assert_eq!(states[&41].selected_abilities, [258, 0, 0]);
    let mut encounter = Encounter::start(&catalog, &journal, &states, 378, 1, 1, 1).unwrap();
    assert_eq!(encounter.battle.teams[0].pets[0].species, 1972);
    assert_eq!(encounter.battle.teams[0].pets[0].stats.power, 11);
    let (state, _) = encounter
        .battle
        .step(
            &catalog.turns,
            [Action::Ability(258), Action::Pass],
            &mut Fixed(500_000),
        )
        .unwrap();
    assert_eq!(state.teams[0].pets.each_ref().map(|p| p.health), [109; 3]);
    assert_eq!(state.auras[0].ability, 596);
    encounter.battle = state;
    assert_eq!(
        encounter.owned_health(0),
        vec![(41, 109), (42, 109), (43, 109)]
    );
}

#[test]
fn wild_pet_battles_level_health_uses_actual_owned_level_and_clamps_persisted_capacity() {
    let catalog = catalog();
    for level in [2, 7, 25] {
        let mut journal = PetJournal::default();
        for (guid, species) in [(41, 250), (42, 39), (43, 39)] {
            journal
                .add_with_guid(species, level, PetQuality::Rare, 3, guid)
                .unwrap();
        }
        let mut states = owned_states(&catalog, &journal);
        states
            .get_mut(&41)
            .unwrap()
            .select_ability(&catalog.turns, journal.get(41).unwrap(), 2, 576)
            .unwrap();
        let mut encounter = Encounter::start(&catalog, &journal, &states, 378, 1, 1, 1).unwrap();
        let base_health = encounter.battle.teams[0].pets[0].stats.health;
        let (state, _) = encounter
            .battle
            .step(
                &catalog.turns,
                [Action::Ability(576), Action::Pass],
                &mut Fixed(500_000),
            )
            .unwrap();
        assert_eq!(
            state.teams[0].pets[0].stats.health,
            base_health + 5 * i32::from(level)
        );
        assert_eq!(
            state.teams[0].pets[0].health,
            state.teams[0].pets[0].stats.health
        );
        encounter.battle = state;
        assert_eq!(
            encounter
                .owned_health(0)
                .into_iter()
                .find(|(guid, _)| *guid == 41)
                .unwrap()
                .1,
            base_health
        );
    }
}

#[test]
fn wild_pet_battles_lock_on_uses_authored_tonk_target_mark_and_blast() {
    let catalog = catalog();
    let mut journal = PetJournal::default();
    for (guid, species) in [(41, 338), (42, 39), (43, 39)] {
        journal
            .add_with_guid(species, 4, PetQuality::Rare, 3, guid)
            .unwrap();
    }
    let mut states = owned_states(&catalog, &journal);
    states
        .get_mut(&41)
        .unwrap()
        .select_ability(&catalog.turns, journal.get(41).unwrap(), 3, 301)
        .unwrap();
    let identities: Vec<_> = journal.pets.iter().map(Identity::from).collect();
    let allies = build_team(&catalog, &identities, &states).unwrap();
    assert_eq!(allies.pets[0].species, 338);
    assert_eq!(allies.pets[0].abilities[2], 301);
    let enemies = build_team(
        &catalog,
        &std::array::from_fn::<_, 3, _>(|_| Identity {
            guid: None,
            species: 39,
            level: 4,
            breed: 3,
            quality: PetQuality::Rare,
        }),
        &OwnedBattleStates::new(),
    )
    .unwrap();
    let initial = Battle::new([allies, enemies]).unwrap();
    let actions = [Action::Ability(301), Action::Pass];
    let (ready, _) = initial
        .step(&catalog.turns, actions, &mut Fixed(500_000))
        .unwrap();
    assert_eq!(
        ready.teams[1].pets[0].health,
        initial.teams[1].pets[0].health
    );
    assert_eq!(ready.auras[0].ability, 300);
    assert_eq!(
        ready.auras[0].target,
        ::server::pet_battle::PetRef { team: 1, slot: 0 }
    );
    assert_eq!(ready.auras[0].remaining, 1);
    let (launched, _) = ready
        .step(&catalog.turns, actions, &mut Fixed(500_000))
        .unwrap();
    assert!(launched.teams[1].pets[0].health < initial.teams[1].pets[0].health);
    assert!(launched.auras.is_empty());
}

#[test]
fn wild_pet_battles_life_exchange_uses_legal_sprite_darter_and_persisted_injury() {
    let catalog = catalog();
    let mut journal = PetJournal::default();
    for (guid, species) in [(41, 87), (42, 39), (43, 39)] {
        journal
            .add_with_guid(species, 15, PetQuality::Rare, 3, guid)
            .unwrap();
    }
    let mut states = owned_states(&catalog, &journal);
    let caller = states.get_mut(&41).unwrap();
    caller
        .select_ability(&catalog.turns, journal.get(41).unwrap(), 2, 277)
        .unwrap();
    caller.health = 50;
    let identities: Vec<_> = journal.pets.iter().map(Identity::from).collect();
    let allies = build_team(&catalog, &identities, &states).unwrap();
    assert_eq!(allies.pets[0].species, 87);
    assert_eq!(allies.pets[0].health, 50);
    assert_eq!(allies.pets[0].abilities[1], 277);
    let enemies = build_team(
        &catalog,
        &std::array::from_fn::<_, 3, _>(|_| Identity {
            guid: None,
            species: 39,
            level: 4,
            breed: 3,
            quality: PetQuality::Rare,
        }),
        &OwnedBattleStates::new(),
    )
    .unwrap();
    let initial = Battle::new([allies, enemies]).unwrap();
    let expected = (50 + initial.teams[1].pets[0].health) / 2;
    let (state, _) = initial
        .step(
            &catalog.turns,
            [Action::Ability(277), Action::Pass],
            &mut Fixed(500_000),
        )
        .unwrap();
    assert_eq!(state.teams[0].pets[0].health, expected);
    assert_eq!(state.teams[1].pets[0].health, expected);
}

#[test]
fn wild_pet_battles_launch_rocket_uses_unlocked_lil_bling_setup_and_launch() {
    let catalog = catalog();
    let mut journal = PetJournal::default();
    for (guid, species) in [(41, 1320), (42, 39), (43, 39)] {
        journal
            .add_with_guid(species, 20, PetQuality::Rare, 3, guid)
            .unwrap();
    }
    let mut states = owned_states(&catalog, &journal);
    states
        .get_mut(&41)
        .unwrap()
        .select_ability(&catalog.turns, journal.get(41).unwrap(), 3, 293)
        .unwrap();
    let identities: Vec<_> = journal.pets.iter().map(Identity::from).collect();
    let allies = build_team(&catalog, &identities, &states).unwrap();
    assert_eq!(allies.pets[0].species, 1320);
    assert_eq!(allies.pets[0].abilities[2], 293);
    let enemies = build_team(&catalog, &identities, &states).unwrap();
    let initial = Battle::new([allies, enemies]).unwrap();
    let actions = [Action::Ability(293), Action::Pass];
    let (ready, _) = initial
        .step(&catalog.turns, actions, &mut Fixed(500_000))
        .unwrap();
    assert_eq!(
        ready.teams[1].pets[0].health,
        initial.teams[1].pets[0].health
    );
    assert_eq!(ready.auras[0].ability, 294);
    let (launched, events) = ready
        .step(&catalog.turns, actions, &mut Fixed(500_000))
        .unwrap();
    assert!(launched.teams[1].pets[0].health < initial.teams[1].pets[0].health);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Damage { .. }))
    );
    assert!(launched.auras.is_empty());
    for slot in 1..3 {
        assert_eq!(
            launched.teams[1].pets[slot].health,
            initial.teams[1].pets[slot].health
        );
        assert_eq!(
            launched.teams[0].pets[slot].health,
            initial.teams[0].pets[slot].health
        );
    }
}

#[test]
fn wild_pet_battles_armageddon_uses_legal_rascal_bot_loadout_without_mechanical_revival() {
    let catalog = catalog();
    let mut journal = PetJournal::default();
    for (guid, species) in [(41, 1256), (42, 39), (43, 39)] {
        journal
            .add_with_guid(species, 4, PetQuality::Rare, 3, guid)
            .unwrap();
    }
    let mut states = owned_states(&catalog, &journal);
    states
        .get_mut(&41)
        .unwrap()
        .select_ability(&catalog.turns, journal.get(41).unwrap(), 3, 1025)
        .unwrap();
    let identities: Vec<_> = journal.pets.iter().map(Identity::from).collect();
    let allies = build_team(&catalog, &identities, &states).unwrap();
    assert_eq!(allies.pets[0].species, 1256);
    assert_eq!(allies.pets[0].family, 9);
    let enemies = build_team(
        &catalog,
        &std::array::from_fn::<_, 3, _>(|_| Identity {
            guid: None,
            species: 39,
            level: 4,
            breed: 3,
            quality: PetQuality::Rare,
        }),
        &OwnedBattleStates::new(),
    )
    .unwrap();
    let initial = Battle::new([allies, enemies]).unwrap();
    let (state, events) = initial
        .step(
            &catalog.turns,
            [Action::Ability(1025), Action::Pass],
            &mut Fixed(500_000),
        )
        .unwrap();
    assert_eq!(state.teams[0].pets[0].health, 0);
    assert!(state.teams[0].pets[0].resurrection_used);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Revived(_)))
    );
    for (before, after) in initial.teams[1].pets.iter().zip(&state.teams[1].pets) {
        assert!(after.health < before.health);
    }
    assert_eq!(
        state.teams[0].pets[1].health,
        initial.teams[0].pets[1].health
    );
    assert_eq!(
        state.teams[0].pets[2].health,
        initial.teams[0].pets[2].health
    );
}

#[test]
fn wild_pet_battles_miss_branch_uses_authored_owned_loadouts_and_persisted_health() {
    let catalog = catalog();
    for (species, ability, slot) in [(1343, 762, 3), (1320, 989, 2)] {
        let mut journal = PetJournal::default();
        for (guid, member) in [(41, species), (42, 39), (43, 39)] {
            journal
                .add_with_guid(member, 4, PetQuality::Rare, 3, guid)
                .unwrap();
        }
        let mut states = owned_states(&catalog, &journal);
        let caller = states.get_mut(&41).unwrap();
        caller
            .select_ability(&catalog.turns, journal.get(41).unwrap(), slot, ability)
            .unwrap();
        caller.health = 100;
        let identities: Vec<_> = journal.pets.iter().map(Identity::from).collect();
        let allies = build_team(&catalog, &identities, &states).unwrap();
        let enemies = build_team(
            &catalog,
            &std::array::from_fn::<_, 3, _>(|_| Identity {
                guid: None,
                species: 39,
                level: 4,
                breed: 3,
                quality: PetQuality::Rare,
            }),
            &OwnedBattleStates::new(),
        )
        .unwrap();
        let enemy_health = enemies.pets.each_ref().map(|pet| pet.health);
        let initial = Battle::new([allies, enemies]).unwrap();
        let (state, events) = initial
            .step(
                &catalog.turns,
                [Action::Ability(ability), Action::Pass],
                &mut Fixed(500_000),
            )
            .unwrap();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::Miss { .. }))
        );
        assert_eq!(
            state.teams[1].pets.each_ref().map(|pet| pet.health),
            enemy_health
        );
        if ability == 762 {
            assert_eq!(state.teams[0].pets[0].health, 100);
            assert_eq!(state.auras.len(), 1);
            assert_eq!(
                state.auras[0].target,
                ::server::pet_battle::PetRef { team: 0, slot: 0 }
            );
        } else {
            assert!(state.teams[0].pets[0].health > 100);
            assert!(state.auras.is_empty());
        }
    }
}

#[test]
fn wild_pet_battles_refuge_uses_owned_species_families_for_targeted_shields() {
    let catalog = catalog();
    let mut journal = PetJournal::default();
    for (guid, species) in [(41, 1430), (42, 1427), (43, 39)] {
        journal
            .add_with_guid(species, 15, PetQuality::Rare, 3, guid)
            .unwrap();
    }
    let mut states = owned_states(&catalog, &journal);
    states
        .get_mut(&41)
        .unwrap()
        .select_ability(&catalog.turns, journal.get(41).unwrap(), 2, 1344)
        .unwrap();
    let identities: Vec<_> = journal.pets.iter().map(Identity::from).collect();
    let allies = build_team(&catalog, &identities, &states).unwrap();
    assert_eq!(allies.pets.each_ref().map(|pet| pet.family), [6, 4, 9]);
    let enemies = build_team(
        &catalog,
        &std::array::from_fn::<_, 3, _>(|_| Identity {
            guid: None,
            species: 1427,
            level: 25,
            breed: 3,
            quality: PetQuality::Rare,
        }),
        &OwnedBattleStates::new(),
    )
    .unwrap();
    let initial = Battle::new([allies, enemies]).unwrap();
    let (state, _) = initial
        .step(
            &catalog.turns,
            [Action::Ability(1344), Action::Pass],
            &mut Fixed(500_000),
        )
        .unwrap();
    let targets: Vec<_> = state
        .auras
        .iter()
        .map(|aura| (aura.target.team, aura.target.slot))
        .collect();
    assert_eq!(targets, vec![(0, 0), (0, 1)]);
    assert_eq!(
        state.teams[0].pets.each_ref().map(|pet| pet.health),
        initial.teams[0].pets.each_ref().map(|pet| pet.health)
    );
    assert!(state.auras.iter().all(|aura| aura.charges == 1));
}

#[test]
fn wild_pet_battles_dragons_call_counts_same_species_not_family_or_enemy_team() {
    let catalog = catalog();
    for (matching_reserves, expected_damage) in [(0, 53), (1, 68), (2, 83)] {
        let mut journal = PetJournal::default();
        for slot in 0..3 {
            let species = if slot == 0 || slot <= matching_reserves {
                2621
            } else {
                3256
            };
            journal
                .add_with_guid(species, 2, PetQuality::Rare, 3, 41 + slot as u64)
                .unwrap();
        }
        let states = owned_states(&catalog, &journal);
        let identities: Vec<_> = journal.pets.iter().map(Identity::from).collect();
        let allies = build_team(&catalog, &identities, &states).unwrap();
        assert_eq!(allies.pets[0].stats.power, 23);
        let enemies = build_team(
            &catalog,
            &std::array::from_fn::<_, 3, _>(|_| Identity {
                guid: None,
                species: 2621,
                level: 25,
                breed: 3,
                quality: PetQuality::Rare,
            }),
            &OwnedBattleStates::new(),
        )
        .unwrap();
        let battle = Battle::new([allies, enemies]).unwrap();
        let (_, events) = battle
            .step(
                &catalog.turns,
                [Action::Ability(2283), Action::Pass],
                &mut Fixed(500_000),
            )
            .unwrap();
        let damage: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                Event::Damage { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect();
        // Current70391 root2283: (25 + 7*matching reserves) * (1 + .05*power23).
        assert_eq!(
            damage,
            vec![expected_damage],
            "matching reserves={matching_reserves}"
        );
    }
}

#[test]
fn wild_pet_battles_bench_resurrection_reloads_dead_owned_reserves() {
    let catalog = catalog();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("dead-reserves.redb");
    let mut journal = PetJournal::default();
    for (id, species) in [(41, 2959), (42, 39), (43, 39)] {
        journal
            .add_with_guid(species, 25, PetQuality::Rare, 3, id)
            .unwrap();
    }
    let mut states = owned_states(&catalog, &journal);
    states
        .get_mut(&41)
        .unwrap()
        .select_ability(&catalog.turns, journal.get(41).unwrap(), 2, 2347)
        .unwrap();
    states.get_mut(&42).unwrap().health = 0;
    states.get_mut(&43).unwrap().health = 0;
    {
        let db = redb::Database::create(&path).unwrap();
        crate::pet_journal_state::save_journal_and_states(&db, 42, &journal, &states).unwrap();
    }
    let db = redb::Database::open(&path).unwrap();
    let states = crate::pet_journal_state::load_states(&db, 42).unwrap();
    let identities: Vec<_> = journal.pets.iter().map(Identity::from).collect();
    let allies = build_team(&catalog, &identities, &states).unwrap();
    let dead = catalog.turns.state_id("Is_Dead").unwrap();
    assert_eq!(
        allies.pets[1].state(dead),
        1,
        "persisted zero HP is dead before any round"
    );
    assert_eq!(allies.pets[2].state(dead), 1);
    let enemies = build_team(
        &catalog,
        &std::array::from_fn::<_, 3, _>(|_| Identity {
            guid: None,
            species: 39,
            level: 25,
            breed: 3,
            quality: PetQuality::Rare,
        }),
        &OwnedBattleStates::new(),
    )
    .unwrap();
    let battle = Battle::new([allies, enemies]).unwrap();
    let (battle, _) = battle
        .step(
            &catalog.turns,
            [Action::Ability(2347), Action::Pass],
            &mut Fixed(500_000),
        )
        .unwrap();
    for pet in &battle.teams[0].pets[1..] {
        assert_eq!(pet.health, pet.stats.health * 5 / 100);
        assert_eq!(pet.state(dead), 0);
    }
}

#[test]
fn owned_battle_team_uses_chosen_abilities_injuries_and_living_front_pet() {
    let catalog = catalog();
    let mut journal = journal();
    journal.pets[0].level = 25;
    let mut states = owned_states(&catalog, &journal);
    states
        .get_mut(&41)
        .unwrap()
        .select_ability(&catalog.turns, journal.get(41).unwrap(), 1, 202)
        .unwrap();
    states.get_mut(&41).unwrap().health = 0;
    states.get_mut(&42).unwrap().health = 80;
    let identities: Vec<_> = journal.pets.iter().map(Identity::from).collect();
    let team = build_team(&catalog, &identities, &states).unwrap();
    assert_eq!(team.active, 1);
    assert_eq!(team.pets[0].health, 0);
    assert_eq!(team.pets[0].abilities[0], 202);
    assert_eq!(team.pets[1].health, 80);
    states.remove(&41);
    assert!(
        build_team(&catalog, &identities, &states).is_err(),
        "missing owned state never restores full health"
    );
}

#[test]
fn wild_pet_battle_production_start_round_capture_and_account_persistence_boundary() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("production.redb");
    let database = redb::Database::create(&path).unwrap();
    crate::persistence::create_tables(&database);
    let (character_id, _) =
        crate::persistence::create_character(&database, 42, "Petcatcher", 1, 1).unwrap();
    let mut owned = journal();
    // Valid level2/near-threshold XP setup; the actual terminal reward raises pet41 to3.
    owned.pets[0].level = 2;
    owned.pets[0].xp = shared::pet_battle::xp_to_next_level(2) - 1;
    crate::battle_pets::save_journal(&database, 42, &owned);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.finish();
    app.cleanup();
    app.insert_resource(GameDatabase { db: database });
    app.insert_resource(crate::battle_pets::load_test_catalog());
    app.insert_resource(catalog());
    app.init_resource::<BattleIds>();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    app.insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    bevy::app::Plugin::build(&crate::achievement::AchievementPlugin, &mut app);
    app.add_systems(Update, crate::battle_pets::load_account_journals);
    let position = Position {
        x: -8949.0,
        y: 82.62,
        z: 132.0,
    };
    let owner = app
        .world_mut()
        .spawn((
            crate::collection::default_collection_state(),
            shared::components::Player {
                name: "Petcatcher".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            shared::components::Level(1),
            crate::achievement::PlayerAchievementState::default(),
            position,
            crate::maps::InMap::continent(0),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            Authenticated { account_id: 42 },
            InWorld { character_id },
            PlayerLink(owner),
            lightyear::prelude::RemoteId(lightyear::prelude::PeerId::Local(1)),
            lightyear::prelude::Connected,
            lightyear::connection::host::HostClient { buffer: Vec::new() },
            MessageSender::<WildPetBattleUpdate>::default(),
            MessageReceiver::<WildPetBattleUpdate>::default(),
        ))
        .id();
    let creature = app
        .world_mut()
        .spawn((
            Npc {
                template_id: 61080,
                name: "Rabbit".into(),
            },
            position,
            crate::maps::InMap::continent(1),
            Health {
                current: 100.0,
                max: 100.0,
            },
            UnitLevel(1),
        ))
        .id();
    app.update();
    assert!(
        !app.world()
            .get::<crate::achievement::PlayerAchievementState>(owner)
            .unwrap()
            .account
            .as_ref()
            .unwrap()
            .earned_ids
            .contains(&7433),
        "seeded starting level is not XP credit"
    );
    assert!(
        start_encounter(app.world_mut(), connection, owner, creature.to_bits())
            .unwrap_err()
            .contains("map")
    );
    app.world_mut()
        .entity_mut(creature)
        .insert(crate::maps::InMap::continent(0));
    start_encounter(app.world_mut(), connection, owner, creature.to_bits()).unwrap();
    assert!(start_encounter(app.world_mut(), connection, owner, creature.to_bits()).is_err());
    let id = app
        .world()
        .get::<ActiveEncounter>(owner)
        .unwrap()
        .encounter
        .id;
    assert!(
        advance_encounter(
            app.world_mut(),
            owner,
            WildPetBattleActionRequest {
                battle_id: id,
                round: 99,
                action: WildPetBattleAction::Pass
            }
        )
        .unwrap_err()
        .contains("stale")
    );
    app.world_mut()
        .get_mut::<ActiveEncounter>(owner)
        .unwrap()
        .encounter
        .battle
        .teams[0]
        .pets[0]
        .health = 73;
    app.world_mut()
        .get_mut::<ActiveEncounter>(owner)
        .unwrap()
        .encounter
        .battle
        .teams[1]
        .pets[0]
        .health = 1;
    app.world_mut()
        .get_mut::<ActiveEncounter>(owner)
        .unwrap()
        .encounter
        .trap_failures = 4;
    for _ in 0..5 {
        let Some(active) = app.world().get::<ActiveEncounter>(owner) else {
            break;
        };
        let round = active.encounter.battle.round;
        advance_encounter(
            app.world_mut(),
            owner,
            WildPetBattleActionRequest {
                battle_id: id,
                round,
                action: WildPetBattleAction::Trap,
            },
        )
        .unwrap();
    }
    app.update();
    let delivered: Vec<_> = app
        .world_mut()
        .get_mut::<MessageReceiver<WildPetBattleUpdate>>(connection)
        .unwrap()
        .receive()
        .collect();
    let terminal_snapshot = delivered
        .iter()
        .rev()
        .find_map(|update| match update {
            WildPetBattleUpdate::Start(state)
            | WildPetBattleUpdate::State(state)
            | WildPetBattleUpdate::Round { state, .. } => Some(state),
            _ => None,
        })
        .expect("client receives authoritative battle state");
    assert_eq!(
        terminal_snapshot.teams[1][0].health, 1,
        "terminal HP reaches the client before End"
    );
    assert!(matches!(
        delivered.last(),
        Some(WildPetBattleUpdate::End {
            captured_pet_id: Some(_),
            ..
        })
    ));
    assert!(app.world().get::<ActiveEncounter>(owner).is_none());
    assert!(app.world().get::<WildPetClaim>(creature).is_none());
    assert_eq!(app.world().get::<Health>(creature).unwrap().current, 0.0);
    assert!(
        app.world()
            .get::<crate::combat::DeathState>(creature)
            .is_some()
    );
    let captured = app
        .world()
        .get::<AccountPetJournal>(owner)
        .unwrap()
        .journal
        .clone();
    assert_eq!(captured.count_species(378), 1);
    assert_eq!(
        app.world()
            .get::<crate::achievement::PlayerAchievementState>(owner)
            .unwrap()
            .criteria_progress
            .get(&19749),
        Some(&1)
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<crate::achievement::PlayerAchievementState>(owner)
            .unwrap()
            .criteria_progress
            .get(&19749),
        Some(&1)
    );
    assert_eq!(captured.get(41).unwrap().level, 3);
    let state = app
        .world()
        .get::<crate::achievement::PlayerAchievementState>(owner)
        .unwrap();
    assert_eq!(state.criteria_progress.get(&20980), Some(&1));
    assert_eq!(state.criteria_progress.get(&21043), Some(&1));
    assert!(state.account.as_ref().unwrap().earned_ids.contains(&7433));
    app.update();
    assert_eq!(
        app.world()
            .get::<crate::achievement::PlayerAchievementState>(owner)
            .unwrap()
            .criteria_progress
            .get(&20980),
        Some(&1)
    );
    let saved =
        crate::pet_journal_state::load_states(&app.world().resource::<GameDatabase>().db, 42)
            .unwrap();
    assert_eq!(
        saved[&41].health, 73,
        "level-up must not manufacture a heal"
    );
    let captured_id = captured
        .pets
        .iter()
        .find(|pet| pet.species_id == 378)
        .unwrap()
        .id;
    assert_eq!(
        saved[&captured_id].health, 1,
        "capture preserves actual terminal HP"
    );
    let next = app
        .world_mut()
        .spawn((
            Npc {
                template_id: 61080,
                name: "Rabbit".into(),
            },
            position,
            crate::maps::InMap::continent(0),
            Health {
                current: 100.0,
                max: 100.0,
            },
            UnitLevel(1),
        ))
        .id();
    start_encounter(app.world_mut(), connection, owner, next.to_bits()).unwrap();
    assert_eq!(
        app.world()
            .get::<ActiveEncounter>(owner)
            .unwrap()
            .encounter
            .battle
            .teams[0]
            .pets[0]
            .health,
        73,
        "later encounter consumes persisted injury instead of full-health initialization"
    );
    drop(app);
    let database = redb::Database::open(&path).unwrap();
    assert_eq!(
        crate::pet_journal_state::load_states(&database, 42).unwrap()[&41].health,
        73
    );
    assert_eq!(crate::battle_pets::load_journal(&database, 42), captured);
    assert_eq!(crate::battle_pets::load_journal(&database, 43).count(), 0);
    assert_eq!(
        crate::persistence::load_achievements(&database, character_id)
            .criteria_progress
            .get(&19749),
        Some(&1)
    );
}

#[test]
fn wild_pet_battle_current_wild_xp_supports_carry_pets_without_a_legacy_level_cap() {
    assert_eq!(earned_xp(1, 1), 75);
    assert_eq!(earned_xp(1, 25), 435);
    assert_eq!(earned_xp(24, 1), 0);
}

#[test]
fn wild_pet_battle_trap_requires_a_living_active_ally() {
    let catalog = catalog();
    let journal = journal();
    let mut encounter = Encounter::start(
        &catalog,
        &journal,
        &owned_states(&catalog, &journal),
        378,
        1,
        99,
        100,
    )
    .unwrap();
    encounter.battle.teams[1].pets[0].health = 1;
    encounter.battle.teams[0].pets[0].health = 0;
    assert!(!encounter.can_trap(&journal));
}

#[test]
fn wild_pet_battle_real_rabbit_ability_round_is_replayable_and_swap_is_ordered() {
    let catalog = catalog();
    let journal = journal();
    let encounter = Encounter::start(
        &catalog,
        &journal,
        &owned_states(&catalog, &journal),
        378,
        1,
        99,
        100,
    )
    .unwrap();
    assert_eq!(
        encounter.snapshot(&catalog, &journal).teams[1][0].species_id,
        378
    );
    let (first, events) = encounter
        .step(
            &catalog,
            &journal,
            WildPetBattleAction::Ability(1),
            &mut Fixed(500_000),
        )
        .unwrap();
    let (replay, replay_events) = encounter
        .step(
            &catalog,
            &journal,
            WildPetBattleAction::Ability(1),
            &mut Fixed(500_000),
        )
        .unwrap();
    assert_eq!(first.battle, replay.battle);
    assert_eq!(events, replay_events);
    let hud = first.snapshot(&catalog, &journal);
    assert_eq!(hud.teams[0][0].quality, 3);
    assert_eq!(hud.teams[0][0].icon, 656_559);
    assert_eq!(hud.teams[0][0].abilities[0].icon, 133_832);
    assert_eq!(hud.teams[0][0].abilities[0].family, 9);
    assert_eq!(hud.teams[0][0].xp, journal.get(41).unwrap().xp);
    assert!(hud.teams[0][0].next_level_xp > 0);
    let damage = encounter.battle.teams[1].pets[0].health - first.battle.teams[1].pets[0].health;
    assert!(hud.feedback.iter().any(|event| event.team == 1
        && event.slot == 0
        && event.text == damage.to_string()
        && !event.healing));
    assert!(
        !events
            .iter()
            .any(|text| text.contains("PetRef") || text.contains("Order("))
    );
    assert_eq!(hud.feedback, replay.snapshot(&catalog, &journal).feedback);
    assert!(first.battle.teams[1].pets[0].health < encounter.battle.teams[1].pets[0].health);
    let (swapped, _) = first
        .step(
            &catalog,
            &journal,
            WildPetBattleAction::Swap(2),
            &mut Fixed(500_000),
        )
        .unwrap();
    assert_eq!(swapped.battle.teams[0].active, 1);
}
#[test]
fn wild_pet_battle_trap_is_strictly_below_35_and_capture_respects_species_limit() {
    let catalog = catalog();
    let mut journal = journal();
    let mut encounter = Encounter::start(
        &catalog,
        &journal,
        &owned_states(&catalog, &journal),
        378,
        1,
        99,
        100,
    )
    .unwrap();
    let enemy = &mut encounter.battle.teams[1].pets[0];
    enemy.health = (enemy.stats.health * 35 + 99) / 100;
    assert!(
        encounter
            .step(&catalog, &journal, WildPetBattleAction::Trap, &mut Fixed(0))
            .is_err()
    );
    encounter.battle.teams[1].pets[0].health = 1;
    let (captured, _) = encounter
        .step(&catalog, &journal, WildPetBattleAction::Trap, &mut Fixed(0))
        .unwrap();
    assert_eq!(captured.battle.outcome, Some(Outcome::Winner(0)));
    assert!(captured.captured.is_some());
    captured
        .apply_rewards(&catalog, &mut journal, Some(999))
        .unwrap();
    assert_eq!(journal.get(999).unwrap().species_id, 378);
    for id in 998..1000 {
        if journal.get(id).is_none() {
            journal
                .add_with_guid(378, 1, PetQuality::Common, 3, id)
                .unwrap();
        }
    }
    journal
        .add_with_guid(378, 1, PetQuality::Common, 3, 1000)
        .unwrap();
    assert!(
        encounter
            .step(&catalog, &journal, WildPetBattleAction::Trap, &mut Fixed(0))
            .is_err()
    );
}
#[test]
fn wild_pet_battle_forfeit_has_no_rewards_and_win_xp_survives_reopen() {
    let catalog = catalog();
    let mut journal = journal();
    let encounter = Encounter::start(
        &catalog,
        &journal,
        &owned_states(&catalog, &journal),
        378,
        1,
        99,
        100,
    )
    .unwrap();
    let (forfeited, _) = encounter
        .step(
            &catalog,
            &journal,
            WildPetBattleAction::Forfeit,
            &mut Fixed(0),
        )
        .unwrap();
    assert_eq!(forfeited.battle.outcome, Some(Outcome::Winner(1)));
    assert!(
        forfeited
            .apply_rewards(&catalog, &mut journal, None)
            .unwrap()
            .is_empty()
    );
    let mut winner = encounter;
    winner.battle.teams[1].pets[0].health = 1;
    let (winner, _) = winner
        .step(
            &catalog,
            &journal,
            WildPetBattleAction::Ability(1),
            &mut Fixed(500_000),
        )
        .unwrap();
    assert_eq!(winner.battle.outcome, Some(Outcome::Winner(0)));
    let rewards = winner.apply_rewards(&catalog, &mut journal, None).unwrap();
    assert_eq!(rewards.len(), 1);
    assert_eq!(rewards[0].xp_gained, 75);
    assert_eq!(journal.get(41).unwrap().level, 2);
    assert_eq!(journal.get(41).unwrap().xp, 25);
    assert_eq!(journal.get(42).unwrap().xp, 0);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("wild.redb");
    {
        let db = redb::Database::create(&path).unwrap();
        crate::battle_pets::save_journal(&db, 42, &journal);
    }
    let db = redb::Database::open(&path).unwrap();
    assert_eq!(crate::battle_pets::load_journal(&db, 42), journal);
}

#[test]
fn petbattle_effect_hud_turn_lock_disables_abilities_without_guessed_lockdown() {
    let catalog = catalog();
    let journal = journal();
    let mut encounter = Encounter::start(
        &catalog,
        &journal,
        &owned_states(&catalog, &journal),
        378,
        1,
        99,
        100,
    )
    .unwrap();
    assert!(encounter.snapshot(&catalog, &journal).teams[0][0].abilities[0].usable);
    let turn_lock = catalog.turns.state_id("turnLock").unwrap();
    encounter.battle.teams[0].pets[0]
        .states
        .insert(turn_lock, 1);
    let ability = &encounter.snapshot(&catalog, &journal).teams[0][0].abilities[0];
    assert!(!ability.usable);
    assert_eq!(ability.lockout, 0);
    encounter.battle.teams[0].pets[0]
        .states
        .insert(turn_lock, 0);
    let ability_id = encounter.battle.teams[0].pets[0].abilities[0];
    encounter.battle.teams[0].pets[0].continuation = Some((ability_id, 2));
    assert!(!encounter.snapshot(&catalog, &journal).teams[0][0].abilities[0].usable);
    encounter.battle.teams[0].pets[0].continuation = None;
    assert!(encounter.snapshot(&catalog, &journal).teams[0][0].abilities[0].usable);
}

#[test]
fn petbattle_effect_hud_reports_real_mind_games_slot_lockdowns() {
    let catalog = catalog();
    let journal = journal();
    let mut encounter = Encounter::start(
        &catalog,
        &journal,
        &owned_states(&catalog, &journal),
        378,
        1,
        99,
        100,
    )
    .unwrap();
    encounter.battle.teams[0].pets[0].abilities = [110, 123, 2390];
    encounter.battle = encounter
        .battle
        .step(
            &catalog.turns,
            [Action::Ability(2390), Action::Pass],
            &mut Fixed(500000),
        )
        .unwrap()
        .0;
    let snapshot = encounter.snapshot(&catalog, &journal);
    let abilities = &snapshot.teams[0][0].abilities;
    assert_eq!(abilities[0].id, 110);
    assert_eq!(abilities[1].id, 123);
    for ability in &abilities[..2] {
        assert_eq!(ability.lockout, 9);
        assert_eq!(ability.cooldown, 0);
        assert!(!ability.usable);
    }
    assert_eq!(abilities[2].lockout, 0);
    assert!(abilities[2].usable);
    assert!(snapshot.teams[1][0].abilities[0].usable);
}

#[test]
fn petbattle_effect_hud_routes_weather_and_team_hazard_separately_across_swap() {
    let catalog = catalog();
    let journal = journal();
    let mut encounter = Encounter::start(
        &catalog,
        &journal,
        &owned_states(&catalog, &journal),
        378,
        1,
        99,
        100,
    )
    .unwrap();
    encounter.battle.teams[0].pets[0].abilities = [404, 190, 527];
    encounter.battle.teams[0].pets[1].abilities = [254, 110, 123];
    for team in &mut encounter.battle.teams {
        for pet in &mut team.pets {
            pet.health = 10000;
            pet.stats.health = 10000;
            pet.base_health = 10000;
        }
    }
    encounter.battle = encounter
        .battle
        .step(
            &catalog.turns,
            [Action::Ability(404), Action::Pass],
            &mut Fixed(500000),
        )
        .unwrap()
        .0;
    let snapshot = encounter.snapshot(&catalog, &journal);
    assert_eq!(snapshot.weather.as_ref().map(|a| a.ability_id), Some(403));
    assert!(snapshot.teams.iter().flatten().all(|p| p.auras.is_empty()));
    encounter.battle = encounter
        .battle
        .step(
            &catalog.turns,
            [Action::Ability(190), Action::Pass],
            &mut Fixed(500000),
        )
        .unwrap()
        .0;
    let snapshot = encounter.snapshot(&catalog, &journal);
    assert_eq!(snapshot.team_auras[1].len(), 1);
    assert_eq!(snapshot.team_auras[1][0].ability_id, 189);
    assert!(!snapshot.team_auras[1][0].is_buff);
    encounter.battle = encounter
        .battle
        .step(
            &catalog.turns,
            [Action::Ability(527), Action::Pass],
            &mut Fixed(500000),
        )
        .unwrap()
        .0;
    let snapshot = encounter.snapshot(&catalog, &journal);
    assert_eq!(snapshot.team_auras[1][0].ability_id, 189);
    assert!(
        snapshot.teams[1]
            .iter()
            .all(|p| p.auras.iter().any(|a| a.ability_id == 526 && !a.is_buff))
    );
    encounter.battle = encounter
        .battle
        .step(
            &catalog.turns,
            [Action::Swap(1), Action::Pass],
            &mut Fixed(500000),
        )
        .unwrap()
        .0;
    encounter.battle = encounter
        .battle
        .step(
            &catalog.turns,
            [Action::Ability(254), Action::Pass],
            &mut Fixed(500000),
        )
        .unwrap()
        .0;
    let snapshot = encounter.snapshot(&catalog, &journal);
    assert!(
        snapshot.team_auras[0]
            .iter()
            .any(|a| a.ability_id == 255 && a.is_buff)
    );
    assert!(snapshot.teams.iter().flatten().all(|p| {
        p.auras
            .iter()
            .all(|a| ![403, 189, 255].contains(&a.ability_id))
    }));
}
