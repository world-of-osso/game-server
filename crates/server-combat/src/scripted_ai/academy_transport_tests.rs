use super::*;
use shared::components::{CharacterAppearance, Rotation};

fn player(world: &mut World, map: InMap, pos: Vec3) -> Entity {
    world
        .spawn((
            Player {
                name: "Bramble".into(),
                race: 1,
                class: 8,
                appearance: CharacterAppearance::default(),
            },
            map,
            Position {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        ))
        .id()
}

#[test]
fn academy_pad_moves_owner_follower_and_pet_then_releases_control() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.add_systems(
        Update,
        (
            super::super::jump_movement::release_finished_jumps,
            finish_entrance_pad,
        )
            .chain(),
    );
    let map = InMap {
        map_id: 2526,
        instance_id: 7,
    };
    let world = app.world_mut();
    let owner = player(world, map, PAD);
    let foreign = player(
        world,
        InMap {
            instance_id: 8,
            ..map
        },
        PAD,
    );
    world.spawn((
        Npc {
            template_id: 195564,
            name: "Jump Pad".into(),
        },
        map,
        Position {
            x: PAD.x,
            y: PAD.y,
            z: PAD.z,
        },
    ));
    let follower = world
        .spawn((
            crate::followers::Follower {
                owner,
                profile: crate::followers::Profile {
                    entry: 198595,
                    name: "Garrick".into(),
                    class_id: 1,
                    spec_id: 73,
                    role: shared::group::GroupRole::Tank,
                    attack_spell: 0,
                    cooldown_spell: 0,
                    heal_spell: 0,
                    taunt_spell: 0,
                    form_spell: 0,
                    melee: true,
                },
            },
            map,
            Position {
                x: PAD.x + 1.0,
                y: PAD.y,
                z: PAD.z,
            },
        ))
        .id();
    let pet = world
        .spawn((
            map,
            Position {
                x: PAD.x - 1.0,
                y: PAD.y,
                z: PAD.z,
            },
            shared::components::Health {
                current: 100.0,
                max: 100.0,
            },
            crate::hunter_pet::HunterPet { owner, slot: 0 },
        ))
        .id();
    launch_entrance_pad(world);
    assert!(world.get::<MovementControl>(owner).unwrap().controlled);
    assert!(world.get::<JumpMovement>(follower).is_some());
    assert!(
        world.get::<JumpMovement>(pet).is_some(),
        "the owner's pet rides the pad"
    );
    assert!(world.get::<JumpMovement>(foreign).is_none());
    world
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(500));
    super::super::jump_movement::advance_jumps(world);
    assert_ne!(
        *world.get::<Position>(owner).unwrap(),
        Position {
            x: LANDING.x,
            y: LANDING.y,
            z: LANDING.z
        }
    );
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
    let control = app.world().get::<MovementControl>(owner).unwrap();
    assert!(!control.controlled);
    assert_eq!(control.epoch, 2, "landing invalidates in-flight input");
}

#[test]
fn academy_pad_crawth_approach_does_not_launch_return_flight() {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    let map = InMap {
        map_id: 2526,
        instance_id: 7,
    };
    let (_, pad, landing, _) = PADS[3];
    world.spawn((
        Npc {
            template_id: 192864,
            name: "Jump Pad".into(),
        },
        map,
        Position {
            x: pad.x,
            y: pad.y,
            z: pad.z,
        },
    ));
    // The live outbound landing-to-Eagles route passes the return NPC here.
    let approach = Vec3::new(2318.13, 908.47, 3051.64);
    let owner = player(&mut world, map, approach);
    launch_entrance_pad(&mut world);
    assert!(world.get::<JumpMovement>(owner).is_none());
    *world.get_mut::<Position>(owner).unwrap() = Position {
        x: pad.x,
        y: 908.47784,
        z: pad.z,
    };
    launch_entrance_pad(&mut world);
    assert!(
        world.get::<JumpMovement>(owner).is_some(),
        "entering the return pad must launch toward {landing:?}"
    );
}

#[test]
fn academy_pad_landing_rejects_in_flight_network_input() {
    use lightyear::prelude::{MessageReceiver, MessageSender};
    use shared::components::{MovementSpeed, PlayerMotion};
    use shared::protocol::{InputChannel, PlayerInput};
    let mut h =
        crate::spell_cast::test_harness::harness_with_plugins(Vec::new(), &[], 1, false, |app| {
            app.init_resource::<crate::friends::AutoAfkScheduler>();
            app.add_systems(
                FixedPreUpdate,
                (
                    super::super::jump_movement::release_finished_jumps,
                    finish_entrance_pad,
                )
                    .chain(),
            );
            app.add_systems(
                FixedUpdate,
                (
                    super::super::jump_movement::advance_jumps,
                    crate::networking::process_player_inputs,
                )
                    .chain(),
            );
        })
        .expect("world database");
    h.app
        .insert_resource(crate::terrain_height::test_helpers::flat_ground(
            33.0, 33.0, 0.0,
        ));
    let map = InMap {
        map_id: 2526,
        instance_id: 7,
    };
    h.app.world_mut().entity_mut(h.caster).insert((
        map,
        Position {
            x: PAD.x,
            y: PAD.y,
            z: PAD.z,
        },
        Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        MovementSpeed(0.0),
        PlayerMotion::default(),
        crate::networking::FallTracker::grounded(PAD.y),
    ));
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<PlayerInput>::default(),
        MessageReceiver::<PlayerInput>::default(),
    ));
    h.app.world_mut().spawn((
        Npc {
            template_id: 195564,
            name: "Jump Pad".into(),
        },
        map,
        Position {
            x: PAD.x,
            y: PAD.y,
            z: PAD.z,
        },
    ));
    launch_entrance_pad(h.app.world_mut());
    assert_eq!(
        h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .epoch,
        1
    );
    h.advance(100);
    let landed = *h.app.world().get::<Position>(h.caster).unwrap();
    assert_eq!(
        landed,
        Position {
            x: LANDING.x,
            y: LANDING.y,
            z: LANDING.z
        }
    );
    assert_eq!(
        h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .epoch,
        2
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<PlayerInput>>(h.connection)
        .unwrap()
        .send::<InputChannel>(PlayerInput {
            transport: None,
            direction: [1.0, 0.0, 0.0],
            facing_yaw: 0.0,
            jumping: false,
            running: true,
            swimming: false,
            flying: false,
            position: [1506.359, 900.0051, 2919.531],
            epoch: 1,
        });
    h.advance(2);
    assert_eq!(
        *h.app.world().get::<Position>(h.caster).unwrap(),
        landed,
        "in-flight input cannot rewind the landing"
    );
}

#[test]
fn academy_pad_other_real_npcs_reach_their_logged_destinations() {
    for &(entry, pad, landing, _) in &PADS[1..] {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        let map = InMap {
            map_id: 2526,
            instance_id: 7,
        };
        let owner = player(&mut world, map, pad);
        world.spawn((
            Npc {
                template_id: entry,
                name: "Jump Pad".into(),
            },
            map,
            Position {
                x: pad.x,
                y: pad.y,
                z: pad.z,
            },
        ));
        launch_entrance_pad(&mut world);
        assert!(world.get::<JumpMovement>(owner).is_some());
        world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(10));
        super::super::jump_movement::advance_jumps(&mut world);
        assert_eq!(
            *world.get::<Position>(owner).unwrap(),
            Position {
                x: landing.x,
                y: landing.y,
                z: landing.z
            }
        );
    }
}

#[test]
#[ignore = "requires real Academy baked LOS geometry"]
fn academy_pad_arc_clears_real_academy_geometry() {
    let root = std::path::PathBuf::from(
        std::env::var_os("GAME_SERVER_GROUND_DIR").expect("ground assets"),
    );
    let mut map = los::LosMap::new(&root.join("los/2526"));
    for &(entry, pad, landing, _) in PADS {
        let duration = pad.distance(landing) / 50.0;
        let height = shared::skyriding::GRAVITY * duration * duration / 8.0;
        let at = |t: f32| pad.lerp(landing, t) + Vec3::Y * (height * 4.0 * t * (1.0 - t) + 1.0);
        for i in 0..100 {
            let start = at(i as f32 / 100.0);
            let end = at((i + 1) as f32 / 100.0);
            assert!(
                map.clear(start, end).unwrap(),
                "pad {entry} at {pad:?} collision segment {i}: {start:?} -> {end:?}"
            );
        }
    }
}
