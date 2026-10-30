use super::*;
use shared::components::{CharacterAppearance, Npc};
use shared::group::GroupRole;

#[test]
#[ignore = "requires map 2923 baked navmesh"]
fn voidscar_portal_moves_owned_party_to_logged_atroxus_floor() {
    let ground = std::path::PathBuf::from(std::env::var_os("GAME_SERVER_GROUND_DIR").unwrap());
    let mut nav = navigation::NavigationMap::open(&ground.join("navmesh/2923")).unwrap();
    let start = Vec3::new(4614.76, -109.9542, 403.0);
    let portal = Vec3::new(4615.12, -103.6306, 430.05);
    let boss = Vec3::new(4800.443, -62.1775, 452.21875);
    assert!(nav.complete_path(start, portal).is_ok());
    assert!(nav.complete_path(LANDING, boss).is_ok());

    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.add_systems(
        Update,
        (
            super::super::jump_movement::release_finished_jumps,
            finish_portal_flight,
        )
            .chain(),
    );
    let world = app.world_mut();
    let map = InMap {
        map_id: 2923,
        instance_id: 7,
    };
    let pos = Position {
        x: portal.x,
        y: portal.y,
        z: portal.z,
    };
    let owner = world
        .spawn((
            Player {
                name: "Voidwalk".into(),
                race: 1,
                class: 8,
                appearance: CharacterAppearance::default(),
            },
            map,
            pos,
        ))
        .id();
    let object = world
        .spawn((
            map,
            pos,
            crate::game_objects::GameObjectScript(SCRIPT.into()),
        ))
        .id();
    let follower = world
        .spawn((
            Npc {
                template_id: 209057,
                name: "Captain Garrick".into(),
            },
            map,
            pos,
            crate::followers::Follower {
                owner,
                profile: crate::followers::Profile {
                    entry: 209057,
                    name: "Captain Garrick".into(),
                    class_id: 2,
                    spec_id: 66,
                    role: GroupRole::Tank,
                    attack_spell: 0,
                    cooldown_spell: 0,
                    heal_spell: 0,
                    taunt_spell: 0,
                    form_spell: 0,
                    melee: true,
                },
            },
        ))
        .id();
    let pet = world
        .spawn((
            map,
            pos,
            shared::components::Health {
                current: 100.0,
                max: 100.0,
            },
            crate::hunter_pet::HunterPet {
                owner: follower,
                slot: 0,
            },
        ))
        .id();
    let foreign = world
        .spawn((
            Player {
                name: "Other".into(),
                race: 1,
                class: 8,
                appearance: CharacterAppearance::default(),
            },
            InMap {
                instance_id: 8,
                ..map
            },
            pos,
        ))
        .id();
    super::super::game_object_gossip_hello(world, owner, object);
    assert!(world.get::<JumpMovement>(owner).is_some());
    assert!(world.get::<JumpMovement>(follower).is_some());
    assert!(world.get::<JumpMovement>(pet).is_some());
    assert!(world.get::<JumpMovement>(foreign).is_none());
    world
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(10));
    super::super::jump_movement::advance_jumps(world);
    app.update();
    for rider in [owner, follower, pet] {
        assert_eq!(
            *app.world().get::<Position>(rider).unwrap(),
            Position {
                x: LANDING.x,
                y: LANDING.y,
                z: LANDING.z
            }
        );
        assert!(app.world().get::<JumpMovement>(rider).is_none());
    }
    assert!(
        !app.world()
            .get::<MovementControl>(owner)
            .unwrap()
            .controlled
    );
}
