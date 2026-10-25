use super::*;

fn catalog() -> AchievementCatalog {
    let db = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    load_achievement_catalog(&db).unwrap()
}

fn gnomeregan() -> ScenarioDefinition {
    ScenarioDefinition {
        id: 649,
        name: "Gnomeregan".into(),
        kind: 3,
        flags: 10,
        steps: vec![ScenarioStep {
            id: 1176,
            title: "Gnomeregan".into(),
            description: "Defeat Mekgineer Thermaplugg.".into(),
            root: 36986,
        }],
    }
}

#[test]
fn scenarios_active_tree_uses_fresh_criteria_and_completes_once() {
    let catalog = catalog();
    let def = gnomeregan();
    let mut run = ScenarioRun::default();
    assert_eq!(run.snapshot(&def, &catalog).stage, 1);
    assert!(!run.apply(
        &def,
        &catalog,
        CriteriaEvent::KillCreature(7361),
        &PlayerConditions::default()
    ));
    assert!(!run.snapshot(&def, &catalog).completed);
    assert!(run.apply(
        &def,
        &catalog,
        CriteriaEvent::KillCreature(7800),
        &PlayerConditions::default()
    ));
    let snapshot = run.snapshot(&def, &catalog);
    assert!(snapshot.completed);
    assert_eq!(snapshot.stage, 2);
    assert_eq!(snapshot.num_stages, 1);
    assert!(snapshot.criteria.is_empty());
    assert!(!run.apply(
        &def,
        &catalog,
        CriteriaEvent::KillCreature(7800),
        &PlayerConditions::default()
    ));
    assert_eq!(run.criteria.criteria_progress[&18529], 1);
    assert_eq!(ScenarioRun::default().snapshot(&def, &catalog).stage, 1);
}

#[test]
fn scenarios_ordered_progress_does_not_credit_future_steps_or_lifetime_achievements() {
    let catalog = catalog();
    // Two-stage service fixture uses the authentic Gnomeregan leaf and its Grubbis
    // bonus leaf; not an authored scenario definition and never loaded in production.
    let mut def = gnomeregan();
    def.steps.push(ScenarioStep {
        id: 1177,
        title: "Grubbis".into(),
        description: "Defeat Grubbis.".into(),
        root: 36988,
    });
    let mut run = ScenarioRun::default();
    assert!(!run.apply(
        &def,
        &catalog,
        CriteriaEvent::KillCreature(7361),
        &PlayerConditions::default()
    ));
    assert!(run.apply(
        &def,
        &catalog,
        CriteriaEvent::KillCreature(7800),
        &PlayerConditions::default()
    ));
    assert_eq!(run.snapshot(&def, &catalog).stage, 2);
    assert!(!run.snapshot(&def, &catalog).completed);
    assert!(run.apply(
        &def,
        &catalog,
        CriteriaEvent::KillCreature(7361),
        &PlayerConditions::default()
    ));
    assert!(run.snapshot(&def, &catalog).completed);
    assert!(run.criteria.earned_ids.is_empty());
}

#[test]
fn scenarios_entry_and_real_death_boundary_are_copy_scoped_and_clear_on_exit() {
    use crate::maps::instances::{InstanceEntrant, Now};
    use shared::components::CharacterAppearance;
    let db = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let store = crate::maps::store::load_map_store(&db).unwrap();
    let mut manager = crate::maps::InstanceManager::default();
    let now = Now {
        elapsed: 0.0,
        unix: 1791763200,
    };
    let copies: Vec<_> = [17, 18]
        .into_iter()
        .map(|character_id| {
            manager
                .create_map(
                    &store,
                    90,
                    InstanceEntrant {
                        character_id,
                        leader: character_id,
                        group: None,
                        difficulty_id: 1,
                        ..Default::default()
                    },
                    now,
                )
                .unwrap()
                .map
        })
        .collect();
    assert_ne!(copies[0], copies[1]);
    let mut world = World::new();
    world.insert_resource(manager);
    world.insert_resource(catalog());
    world.init_resource::<ScenarioRuns>();
    world.init_resource::<crate::maps::ServerClock>();
    world.insert_resource(ScenarioCatalog {
        definitions: HashMap::from([(649, gnomeregan())]),
        bindings: HashMap::from([((90, 1), (649, 649))]),
    });
    let players: Vec<_> = copies
        .iter()
        .enumerate()
        .map(|(index, map)| {
            let mut history = PlayerAchievementState::default();
            history.criteria_progress.insert(18529, 999);
            let player = world
                .spawn((
                    Player {
                        name: format!("Scenarios{index}"),
                        race: 1,
                        class: 8,
                        appearance: CharacterAppearance::default(),
                    },
                    history,
                    *map,
                ))
                .id();
            world.spawn((
                Authenticated {
                    account_id: index as u64 + 17,
                },
                PlayerLink(player),
                InWorld {
                    character_id: index as u64 + 17,
                },
            ));
            player
        })
        .collect();
    send_entry_progress(&mut world);
    for map in &copies {
        let stage = snapshot(&world, *map).scenario.unwrap();
        assert!(!stage.completed);
        assert_eq!(stage.criteria[0].current, 0);
    }
    let boss = world
        .spawn((
            Npc {
                template_id: 7800,
                name: "Mekgineer Thermaplugg".into(),
            },
            copies[0],
        ))
        .id();
    super::super::record_creature_death(&mut world, boss, Some(players[0]));
    assert!(snapshot(&world, copies[0]).scenario.unwrap().completed);
    assert!(!snapshot(&world, copies[1]).scenario.unwrap().completed);
    // Live disconnect/re-entry keeps the copy's completed state, not a new run.
    send_entry_progress(&mut world);
    assert!(snapshot(&world, copies[0]).scenario.unwrap().completed);
    world.entity_mut(players[0]).insert(InMap::continent(0));
    send_entry_progress(&mut world);
    assert!(snapshot(&world, InMap::continent(0)).scenario.is_none());
}

#[test]
fn achievements_criteria_real_scenario_completion_credits_each_copy_player_once() {
    use shared::components::CharacterAppearance;
    let db = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let achievements = load_achievement_catalog(&db).unwrap();
    // Production loader admits authored Arena of Annihilation58 (five kill+event32548 steps).
    let scenarios = load_scenarios(&db, &achievements).unwrap();
    assert_eq!(scenarios.definitions[&58].steps.len(), 5);
    let mut world = World::new();
    world.insert_resource(achievements);
    world.insert_resource(scenarios);
    world.init_resource::<crate::maps::ServerClock>();
    // MAP_SCENARIO1031 is still rejected by InstanceManager admission; two copies are bound
    // directly so the completion producer is exercised, not map entry.
    let arena = InMap {
        map_id: 1031,
        instance_id: 7,
    };
    let other = InMap {
        map_id: 1031,
        instance_id: 8,
    };
    world.insert_resource(ScenarioRuns(HashMap::from([
        (arena, (58, ScenarioRun::default())),
        (other, (58, ScenarioRun::default())),
    ])));
    let [participant, bystander] = [arena, other].map(|map| {
        world
            .spawn((
                Player {
                    name: format!("Scenario{}", map.instance_id),
                    race: 2,
                    class: 1,
                    appearance: CharacterAppearance::default(),
                },
                PlayerAchievementState::default(),
                map,
            ))
            .id()
    });
    let challengers = [63311, 63312, 63313, 63318, 64894];
    for (index, entry) in challengers.into_iter().enumerate() {
        let state = world.get::<PlayerAchievementState>(participant).unwrap();
        assert!(
            !state.earned_ids.contains(&7271),
            "early grant at step {index}"
        );
        assert!(!state.criteria_progress.contains_key(&21813));
        let npc = world
            .spawn((
                Npc {
                    template_id: entry,
                    name: "Challenger".into(),
                },
                arena,
            ))
            .id();
        super::super::record_creature_death(&mut world, npc, Some(participant));
        super::super::record_player_game_event(&mut world, participant, 32548);
    }
    assert!(snapshot(&world, arena).scenario.unwrap().completed);
    let state = world.get::<PlayerAchievementState>(participant).unwrap();
    assert!(state.earned_ids.contains(&7271));
    assert_eq!(state.criteria_progress.get(&21813), Some(&1));
    // A finished run cannot be completed again by later authored events.
    super::super::record_player_game_event(&mut world, participant, 32548);
    let npc = world
        .spawn((
            Npc {
                template_id: 64894,
                name: "Challenger".into(),
            },
            arena,
        ))
        .id();
    super::super::record_creature_death(&mut world, npc, Some(participant));
    assert_eq!(
        world
            .get::<PlayerAchievementState>(participant)
            .unwrap()
            .criteria_progress
            .get(&21813),
        Some(&1)
    );
    let bystander = world.get::<PlayerAchievementState>(bystander).unwrap();
    assert!(!snapshot(&world, other).scenario.unwrap().completed);
    assert!(!bystander.earned_ids.contains(&7271));
    assert!(!bystander.criteria_progress.contains_key(&21813));
}
