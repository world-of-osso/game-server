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
fn academy_pad_moves_owner_and_follower_then_releases_control() {
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
    launch_entrance_pad(world);
    assert!(world.get::<MovementControl>(owner).unwrap().controlled);
    assert!(world.get::<JumpMovement>(follower).is_some());
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
    for rider in [owner, follower] {
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

#[test]
#[ignore = "requires real Academy baked LOS geometry"]
fn academy_pad_arc_clears_real_academy_geometry() {
    let root = std::path::PathBuf::from(
        std::env::var_os("GAME_SERVER_GROUND_DIR").expect("ground assets"),
    );
    let mut map = los::LosMap::new(&root.join("los/2526"));
    let distance = PAD.distance(LANDING);
    let duration = distance / 50.0;
    let height = shared::skyriding::GRAVITY * duration * duration / 8.0;
    let at = |t: f32| PAD.lerp(LANDING, t) + Vec3::Y * (height * 4.0 * t * (1.0 - t) + 1.0);
    for i in 0..100 {
        let start = at(i as f32 / 100.0);
        let end = at((i + 1) as f32 / 100.0);
        assert!(
            map.clear(start, end).unwrap(),
            "collision along arc segment {i}: {start:?} -> {end:?}"
        );
    }
}
