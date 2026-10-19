//! Admin request → queued world operation proof; no live encounter progression claim.
use super::*;
use shared::components::{CombatStatus, Health, MovementControl, Position};

fn fixture() -> (App, Entity, Vec<Entity>) {
    let (mut app, owner, run) = super::copy_tests::fixture();
    join_party(app.world_mut(), owner, &run, &run.members).unwrap();
    let members = run.members.clone();
    app.world_mut().entity_mut(owner).insert((
        run,
        MovementControl::default(),
        CombatStatus(false),
    ));
    (app, owner, members)
}

fn stage(world: &mut World) -> crate::admin_types::AdminResponse {
    crate::admin_ipc::dispatch_staging_fixture(
        world,
        serde_json::json!({"StageFollowerParty": {
            "character_name":"Followproof", "map_id":2521,
            "x":1551.33, "y":-257.688, "z":126.01431
        }}),
    )
}

#[test]
fn admin_follower_staging_places_existing_party_and_preserves_health_and_identity() {
    let (mut app, owner, members) = fixture();
    let copy = *app.world().get::<InMap>(owner).unwrap();
    let before: Vec<_> = members
        .iter()
        .map(|unit| {
            let health = app.world().get::<Health>(*unit).unwrap();
            (
                *unit,
                health.current,
                health.max,
                app.world().get::<Follower>(*unit).unwrap().owner,
            )
        })
        .collect();
    let response = stage(app.world_mut());
    assert!(
        !matches!(response, crate::admin_types::AdminResponse::Error(_)),
        "{response:?}"
    );
    let expected = WorldLocation {
        map_id: 2521,
        position: Vec3::new(1551.33, -257.688, 126.01431),
        orientation: 0.0,
    }
    .bevy_position();
    for unit in std::iter::once(owner).chain(members.iter().copied()) {
        let position = app.world().get::<Position>(unit).unwrap();
        assert_eq!(
            (position.x, position.y, position.z),
            (expected.x, expected.y, expected.z)
        );
        assert_eq!(app.world().get::<InMap>(unit), Some(&copy));
    }
    for (unit, current, max, owner_before) in before {
        let health = app.world().get::<Health>(unit).unwrap();
        assert_eq!((health.current, health.max), (current, max));
        assert_eq!(
            app.world().get::<Follower>(unit).unwrap().owner,
            owner_before
        );
    }
    assert_eq!(app.world().get::<MovementControl>(owner).unwrap().epoch, 1);
}

#[test]
fn admin_follower_staging_refuses_owner_in_combat_without_moving_any_member() {
    let (mut app, owner, members) = fixture();
    let before: Vec<_> = std::iter::once(owner)
        .chain(members.iter().copied())
        .map(|unit| (unit, *app.world().get::<Position>(unit).unwrap()))
        .collect();
    app.world_mut().entity_mut(owner).insert(CombatStatus(true));
    assert!(
        matches!(stage(app.world_mut()),crate::admin_types::AdminResponse::Error(error) if error.contains("combat"))
    );
    for (unit, position) in before {
        let after = app.world().get::<Position>(unit).unwrap();
        assert_eq!(
            (after.x, after.y, after.z),
            (position.x, position.y, position.z)
        );
    }
}

#[test]
fn admin_follower_staging_refuses_active_encounter_without_moving_members() {
    use crate::scripted_ai::instance_script::{
        BossInfo, EncounterState, InstanceScript, InstanceScripts,
    };
    let (mut app, owner, _) = fixture();
    let copy = *app.world().get::<InMap>(owner).unwrap();
    let before = *app.world().get::<Position>(owner).unwrap();
    app.world_mut()
        .insert_resource(InstanceScripts(std::collections::HashMap::from([(
            copy,
            InstanceScript {
                name: "staging encounter fixture",
                bosses: vec![BossInfo {
                    state: EncounterState::InProgress,
                    encounter_id: 2583,
                }],
                boss_creatures: Vec::new(),
                handled_area_triggers: Default::default(),
                data: Default::default(),
                ramparts_sentry_deaths: 0,
            },
        )])));
    assert!(
        matches!(stage(app.world_mut()), crate::admin_types::AdminResponse::Error(error) if error.contains("encounter"))
    );
    let after = app.world().get::<Position>(owner).unwrap();
    assert_eq!((after.x, after.y, after.z), (before.x, before.y, before.z));
}

#[test]
fn admin_follower_staging_refuses_foreign_owned_party_without_moving_members() {
    let (mut app, owner, members) = fixture();
    let stranger = app.world_mut().spawn_empty().id();
    app.world_mut()
        .get_mut::<Follower>(members[0])
        .unwrap()
        .owner = stranger;
    let before = *app.world().get::<Position>(owner).unwrap();
    assert!(
        matches!(stage(app.world_mut()),crate::admin_types::AdminResponse::Error(error) if error.contains("owner"))
    );
    let after = app.world().get::<Position>(owner).unwrap();
    assert_eq!((after.x, after.y, after.z), (before.x, before.y, before.z));
}
