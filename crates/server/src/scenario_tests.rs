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
    assert!(!run.apply(&def, &catalog, CriteriaEvent::KillCreature(7361)));
    assert!(!run.snapshot(&def, &catalog).completed);
    assert!(run.apply(&def, &catalog, CriteriaEvent::KillCreature(7800)));
    let snapshot = run.snapshot(&def, &catalog);
    assert!(snapshot.completed);
    assert_eq!(snapshot.stage, 2);
    assert_eq!(snapshot.num_stages, 1);
    assert!(snapshot.criteria.is_empty());
    assert!(!run.apply(&def, &catalog, CriteriaEvent::KillCreature(7800)));
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
    assert!(!run.apply(&def, &catalog, CriteriaEvent::KillCreature(7361)));
    assert!(run.apply(&def, &catalog, CriteriaEvent::KillCreature(7800)));
    assert_eq!(run.snapshot(&def, &catalog).stage, 2);
    assert!(!run.snapshot(&def, &catalog).completed);
    assert!(run.apply(&def, &catalog, CriteriaEvent::KillCreature(7361)));
    assert!(run.snapshot(&def, &catalog).completed);
    assert!(run.criteria.earned_ids.is_empty());
}
