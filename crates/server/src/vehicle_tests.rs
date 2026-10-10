use crate::vehicle::{Seat, VehicleStore, board, exit, reconcile_passengers};
use crate::{
    group::{GroupRegistry, OnlineMember, OnlineView},
    maps::InMap,
    persistence::ClientId,
};
use bevy::prelude::*;
use shared::components::{Health, Mounted, MovementControl, Position, Rotation, VehiclePassenger};

#[test]
fn vehicle_local_catalog_links_mammoth_and_chopper_to_actual_model_seats() {
    let conn = rusqlite::Connection::open_with_flags(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = super::load_store(&crate::ground::ground_data_dir(), &conn).unwrap();
    assert_eq!(store.creature_vehicles[&32633], 312);
    assert_eq!(store.creature_vehicles[&32286], 318);
    assert_eq!(store.vehicles[&312], [0, 2764, 2765, 0, 0, 0, 0, 0]);
    let left = store.seat_offsets[&(27237, 2764)];
    let right = store.seat_offsets[&(27237, 2765)];
    assert!(left.y > 3.0 && right.y > 3.0);
    assert!(
        left.x * right.x < 0.0,
        "opposite passenger sides: {left:?} / {right:?}"
    );
    assert!(store.seat_offsets.contains_key(&(25870, 2804)));
}

#[test]
fn vehicle_real_mammoth_passenger_position_matches_the_rendered_mount_frame() {
    let (mut world, driver, passenger, _) = setup();
    let conn = rusqlite::Connection::open_with_flags(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    world.insert_resource(super::load_store(&crate::ground::ground_data_dir(), &conn).unwrap());
    board(&mut world, passenger, driver).unwrap();
    let position = world.get::<Position>(passenger).unwrap();
    // Local M2 234720 attachment40: (-1.278439,0.793948,3.151280), plus
    // NpcVisualRoot's -90deg model alignment and the driver at (10,2,20).
    let expected = Vec3::new(10.793948, 5.1512804, 18.721561);
    assert!(
        Vec3::new(position.x, position.y, position.z).abs_diff_eq(expected, 0.0001),
        "seat position {position:?}, expected {expected:?}"
    );
}

fn setup() -> (World, Entity, Entity, Entity) {
    let mut world = World::new();
    let spawn = |world: &mut World, id| {
        world
            .spawn((
                ClientId(id),
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                Position {
                    x: 10.0,
                    y: 2.0,
                    z: 20.0,
                },
                Rotation {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                InMap::continent(0),
                shared::components::Zone { id: 12 },
                MovementControl::default(),
            ))
            .id()
    };
    let driver = spawn(&mut world, 1);
    let passenger = spawn(&mut world, 2);
    let outsider = spawn(&mut world, 3);
    let online: OnlineView = [
        (1, driver, "Driver"),
        (2, passenger, "Passenger"),
        (3, outsider, "Outsider"),
    ]
    .into_iter()
    .map(|(id, entity, name)| {
        (
            id,
            OnlineMember {
                entity: entity.to_bits(),
                name: name.into(),
                class: 1,
                level: 1,
                portrait: Default::default(),
                dungeon_difficulty: 1,
            },
        )
    })
    .collect();
    let mut groups = GroupRegistry::default();
    groups.sync_presence(&online);
    groups.invite(1, "Passenger", &online);
    groups.respond_invite(2, true);
    world.insert_resource(groups);
    let mut store = VehicleStore::default();
    store.vehicles.insert(312, [0, 2764, 2765, 0, 0, 0, 0, 0]);
    store.seats.insert(
        2764,
        Seat {
            flags: 0x02000000,
            offset: Vec3::ZERO,
        },
    );
    store.seats.insert(
        2765,
        Seat {
            flags: 0x02000000,
            offset: Vec3::ZERO,
        },
    );
    store
        .seat_offsets
        .insert((27237, 2764), Vec3::new(1.0, 3.0, 0.0));
    store
        .seat_offsets
        .insert((27237, 2765), Vec3::new(-1.0, 3.0, 0.0));
    world.insert_resource(store);
    world.entity_mut(driver).insert(Mounted {
        mount_display_id: 27237,
        vehicle_id: 312,
        ..Default::default()
    });
    (world, driver, passenger, outsider)
}

#[test]
fn vehicle_boards_group_member_into_sparse_empty_seat_and_exit_frees_it() {
    let (mut world, driver, passenger, _) = setup();
    assert_eq!(board(&mut world, passenger, driver), Ok(()));
    assert_eq!(
        world.get::<Mounted>(driver).unwrap().seats[1],
        Some(passenger.to_bits())
    );
    assert_eq!(
        world.get::<VehiclePassenger>(passenger).unwrap().seat_id,
        2764
    );
    assert!(world.get::<MovementControl>(passenger).unwrap().controlled);
    exit(&mut world, passenger);
    assert!(world.get::<VehiclePassenger>(passenger).is_none());
    assert!(!world.get::<MovementControl>(passenger).unwrap().controlled);
    assert_eq!(world.get::<Mounted>(driver).unwrap().seats[1], None);
}

#[test]
fn vehicle_rejects_non_group_boarding_and_full_vehicle_without_state_changes() {
    let (mut world, driver, passenger, outsider) = setup();
    assert!(board(&mut world, outsider, driver).is_err());
    world.get_mut::<Mounted>(driver).unwrap().seats[1] = Some(99);
    world.get_mut::<Mounted>(driver).unwrap().seats[2] = Some(100);
    assert!(board(&mut world, passenger, driver).is_err());
    assert!(world.get::<VehiclePassenger>(passenger).is_none());
    assert!(!world.get::<MovementControl>(passenger).unwrap().controlled);
}

#[test]
fn vehicle_passenger_follows_driver_rotation_and_translation() {
    let (mut world, driver, passenger, _) = setup();
    board(&mut world, passenger, driver).unwrap();
    world.entity_mut(driver).insert((
        Position {
            x: 30.0,
            y: 5.0,
            z: 40.0,
        },
        Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    reconcile_passengers(&mut world);
    let pos = world.get::<Position>(passenger).unwrap();
    assert!((pos.x - 30.0).abs() < 1e-5);
    assert!((pos.y - 8.0).abs() < 1e-5);
    assert!((pos.z - 39.0).abs() < 1e-5);
}

#[test]
fn vehicle_ejects_on_dismount_death_disconnect_and_zone_transfer() {
    for cause in 0..5 {
        let (mut world, driver, passenger, _) = setup();
        board(&mut world, passenger, driver).unwrap();
        match cause {
            0 => {
                world.entity_mut(driver).remove::<Mounted>();
            }
            1 => {
                world.get_mut::<Health>(driver).unwrap().current = 0.0;
            }
            2 => {
                world.despawn(driver);
            }
            3 => {
                world.entity_mut(driver).insert(InMap::continent(1));
            }
            _ => {
                world
                    .entity_mut(driver)
                    .insert(shared::components::Zone { id: 13 });
            }
        }
        reconcile_passengers(&mut world);
        assert!(
            world.get::<VehiclePassenger>(passenger).is_none(),
            "cause {cause}"
        );
        assert!(!world.get::<MovementControl>(passenger).unwrap().controlled);
    }
}
