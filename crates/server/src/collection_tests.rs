use super::*;
use crate::persistence;
use shared::components::CombatStatus;

#[test]
fn summon_mount_marks_only_selected_mount_active() {
    let mut collection = PlayerCollectionState {
        mounts: vec![
            CollectionMount {
                mount_id: 101,
                name: "Swift Brown Steed".into(),
                known: true,
                active: false,
            },
            CollectionMount {
                mount_id: 102,
                name: "Black War Wolf".into(),
                known: true,
                active: true,
            },
        ],
        pets: vec![],
    };

    let message = summon_mount(&mut collection, 101).expect("summon mount");

    assert_eq!(message, "summoned Swift Brown Steed");
    assert!(collection.mounts[0].active);
    assert!(!collection.mounts[1].active);
}

#[test]
fn dismiss_pet_clears_active_pet() {
    let mut collection = PlayerCollectionState {
        mounts: vec![],
        pets: vec![CollectionPet {
            pet_id: 202,
            name: "Brown Rabbit".into(),
            known: true,
            active: true,
        }],
    };

    let message = dismiss_pet(&mut collection).expect("dismiss pet");

    assert_eq!(message, "dismissed Brown Rabbit");
    assert!(!collection.pets[0].active);
}

#[test]
fn collection_state_round_trips_through_persistence() {
    let db = persistence::temp_database();
    let collection = PlayerCollectionState {
        mounts: vec![CollectionMount {
            mount_id: 101,
            name: "Swift Brown Steed".into(),
            known: true,
            active: true,
        }],
        pets: vec![CollectionPet {
            pet_id: 202,
            name: "Brown Rabbit".into(),
            known: true,
            active: false,
        }],
    };

    persistence::save_collections(&db, 42, &collection);
    let loaded = persistence::load_collections(&db, 42);

    assert_eq!(loaded, collection);
}

#[test]
fn summon_pet_marks_only_selected_pet_active() {
    let mut collection = PlayerCollectionState {
        mounts: vec![],
        pets: vec![
            CollectionPet {
                pet_id: 202,
                name: "Brown Rabbit".into(),
                known: true,
                active: false,
            },
            CollectionPet {
                pet_id: 203,
                name: "Blue Moth".into(),
                known: true,
                active: true,
            },
        ],
    };

    let message = summon_pet(&mut collection, 202).expect("summon pet");

    assert_eq!(message, "summoned Brown Rabbit");
    assert!(collection.pets[0].active);
    assert!(!collection.pets[1].active);
}

#[test]
fn dismiss_mount_clears_active_mount() {
    let mut collection = PlayerCollectionState {
        mounts: vec![CollectionMount {
            mount_id: 101,
            name: "Swift Brown Steed".into(),
            known: true,
            active: true,
        }],
        pets: vec![],
    };

    let message = dismiss_mount(&mut collection).expect("dismiss mount");

    assert_eq!(message, "dismissed Swift Brown Steed");
    assert!(!collection.mounts[0].active);
}

#[test]
fn movement_speed_for_mount_doubles_run_speed() {
    assert_eq!(movement_speed_for_mount(false), shared::movement::RUN_SPEED);
    assert_eq!(
        movement_speed_for_mount(true),
        shared::movement::RUN_SPEED * 2.0
    );
}

#[test]
fn summon_mount_is_blocked_while_in_combat() {
    let error = combat_locked_request_error::<SummonMount>(CombatStatus(true).0)
        .expect("mounts should be blocked during combat");

    assert_eq!(error, "cannot mount while in combat");
}

#[test]
fn mounting_is_blocked_through_a_lull_in_a_fight_and_allowed_once_the_creature_dies() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, crate::combat_refs::expire_combat_refs_system);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(100),
    ));
    let health = shared::components::Health {
        current: 100.0,
        max: 100.0,
    };
    let player = app
        .world_mut()
        .spawn((
            health,
            shared::components::Player {
                name: "Rider".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
        ))
        .id();
    let wolf = app
        .world_mut()
        .spawn((health, shared::components::ThreatTable::default()))
        .id();
    crate::combat_refs::set_in_combat_with(app.world_mut(), player, wolf);
    let mount_error = |app: &App| {
        let in_combat = app.world().get::<CombatStatus>(player).unwrap().0;
        combat_locked_request_error::<SummonMount>(in_combat)
    };
    // 8 s without a hit.
    for _ in 0..80 {
        app.update();
    }
    assert_eq!(mount_error(&app), Some("cannot mount while in combat"));

    crate::combat::combat_stop(app.world_mut(), wolf);
    assert_eq!(mount_error(&app), None);
}

#[test]
fn dismiss_mount_is_not_blocked_while_in_combat() {
    let error = combat_locked_request_error::<DismissMount>(CombatStatus(true).0);

    assert_eq!(error, None);
}
