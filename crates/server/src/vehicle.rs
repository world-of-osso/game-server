//! Player-owned passenger mounts. VehicleSeat CanEnterOrExit is 0x02000000
//! (TrinityCore a352b1fa DBCEnums.h); seat enum -> M2 attachment mapping is
//! solarityclient vehicle_passenger_c::vehicle_seat_attachment (native A2D3F0).
use crate::{maps::InMap, networking::PlayerLink, persistence::ClientId};
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, ReplicationDirtyQueue};
use shared::components::{Health, Mounted, MovementControl, Position, Rotation, VehiclePassenger};
use shared::protocol::{BoardVehicle, ExitVehicle};
use std::collections::HashMap;

pub(crate) mod isle;

const CAN_ENTER_OR_EXIT: u32 = 0x02000000;
const CAN_CONTROL: u32 = 0x00000800;
const BOARD_DISTANCE: f32 = 5.0;

#[derive(Clone, Copy)]
pub(crate) struct Seat {
    pub flags: u32,
    pub offset: Vec3,
}
#[derive(Resource, Default)]
pub(crate) struct VehicleStore {
    pub vehicles: HashMap<u32, [u32; 8]>,
    pub seats: HashMap<u32, Seat>,
    creature_vehicles: HashMap<u32, u32>,
    npc_vehicles: HashMap<u32, u32>,
    /// Seat position in the unit frame, including model alignment, scale and DB2 offset.
    seat_offsets: HashMap<(u32, u32), Vec3>,
}
#[derive(Component)]
struct PassengerOrigin {
    map: InMap,
    display: u32,
}

pub(crate) struct VehiclePlugin;
impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        // Diagnose missing ground before loading vehicle attachments from the same cache.
        app.add_systems(
            PostStartup,
            load_vehicle_store.after(crate::ground::open_ground_map),
        );
        app.add_systems(Update, receive_requests);
        app.add_systems(FixedPostUpdate, reconcile_passengers);
    }
}

fn mark_dirty(world: &World, entity: Entity) {
    if let Some(queue) = world.get_resource::<ReplicationDirtyQueue>() {
        queue.mark(entity);
    }
}
fn set_control(world: &mut World, entity: Entity, controlled: bool) {
    if let Some(mut control) = world.get_mut::<MovementControl>(entity) {
        control.controlled = controlled;
        control.epoch = control.epoch.wrapping_add(1);
    }
    mark_dirty(world, entity);
}
fn alive(world: &World, entity: Entity) -> bool {
    world
        .get::<Health>(entity)
        .is_some_and(|health| health.current > 0.0)
        && world.get::<ClientId>(entity).is_some()
        && world
            .get::<crate::networking::PendingDespawn>(entity)
            .is_none()
}
fn vec(position: &Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}

pub(crate) fn board(world: &mut World, passenger: Entity, driver: Entity) -> Result<(), String> {
    validate_board(world, passenger, driver)?;
    let mounted = world
        .get::<Mounted>(driver)
        .ok_or("Driver is not mounted")?;
    let store = world.resource::<VehicleStore>();
    let seats = store
        .vehicles
        .get(&mounted.vehicle_id)
        .ok_or("Mount has no passenger seats")?;
    let (index, seat_id) = seats
        .iter()
        .enumerate()
        .find(|(index, id)| {
            mounted.seats[*index].is_none()
                && store.seats.get(id).is_some_and(|seat| {
                    seat.flags & CAN_ENTER_OR_EXIT != 0 && seat.flags & CAN_CONTROL == 0
                })
        })
        .ok_or("Mount is full")?;
    if !store
        .seat_offsets
        .contains_key(&(mounted.mount_display_id, *seat_id))
    {
        return Err("Mount seat model attachment is unavailable".into());
    }
    let seat = VehiclePassenger {
        driver: driver.to_bits(),
        seat_index: index as u8,
        seat_id: *seat_id,
    };
    let origin = PassengerOrigin {
        map: *world.get::<InMap>(driver).unwrap(),
        display: mounted.mount_display_id,
    };
    world.get_mut::<Mounted>(driver).unwrap().seats[index] = Some(passenger.to_bits());
    world.entity_mut(passenger).insert((seat, origin));
    set_control(world, passenger, true);
    mark_dirty(world, driver);
    update_passenger_position(world, passenger, seat);
    Ok(())
}

fn validate_board(world: &World, passenger: Entity, driver: Entity) -> Result<(), String> {
    if passenger == driver || !alive(world, passenger) || !alive(world, driver) {
        return Err("Both riders must be living players".into());
    }
    if world.get::<VehiclePassenger>(passenger).is_some()
        || world.get::<Mounted>(passenger).is_some()
        || world
            .get::<MovementControl>(passenger)
            .is_none_or(|control| control.controlled)
    {
        return Err("Passenger cannot board while mounted or controlled".into());
    }
    let map = world
        .get::<InMap>(passenger)
        .ok_or("Passenger is transferring maps")?;
    if Some(map) != world.get::<InMap>(driver) {
        return Err("Riders must share a map copy".into());
    }
    if world.get::<Rotation>(driver).is_none() {
        return Err("Driver has no rotation".into());
    }
    let passenger_id = world.get::<ClientId>(passenger).unwrap().0;
    let driver_id = world.get::<ClientId>(driver).unwrap().0;
    if !world
        .resource::<crate::group::GroupRegistry>()
        .is_in_same_group(passenger_id, driver_id)
    {
        return Err("Only party or raid members can board".into());
    }
    let passenger_position = world
        .get::<Position>(passenger)
        .ok_or("Passenger has no position")?;
    let driver_position = world
        .get::<Position>(driver)
        .ok_or("Driver has no position")?;
    if vec(passenger_position).distance(vec(driver_position)) > BOARD_DISTANCE {
        return Err("Mount is out of range".into());
    }
    Ok(())
}

pub(crate) fn exit(world: &mut World, passenger: Entity) {
    let Some(seat) = world.get::<VehiclePassenger>(passenger).copied() else {
        return;
    };
    let driver = Entity::from_bits(seat.driver);
    if let Some(mut mount) = world.get_mut::<Mounted>(driver) {
        let slot = &mut mount.seats[usize::from(seat.seat_index)];
        if *slot == Some(passenger.to_bits()) {
            *slot = None;
        }
    }
    world
        .entity_mut(passenger)
        .remove::<(VehiclePassenger, PassengerOrigin)>();
    isle::release_seat(world, passenger, seat);
    set_control(world, passenger, false);
    mark_dirty(world, driver);
}

fn valid_seat(
    world: &World,
    passenger: Entity,
    seat: VehiclePassenger,
    origin: &PassengerOrigin,
) -> bool {
    let driver = Entity::from_bits(seat.driver);
    if world.get::<isle::NpcVehicle>(driver).is_some() {
        return alive(world, passenger)
            && world.get::<InMap>(passenger) == Some(&origin.map)
            && isle::valid_seat(world, passenger, seat, origin);
    }
    let mounted = world.get::<Mounted>(driver);
    alive(world, passenger)
        && alive(world, driver)
        && world.get::<InMap>(passenger) == Some(&origin.map)
        && world.get::<InMap>(driver) == Some(&origin.map)
        && mounted.is_some_and(|mount| {
            mount.mount_display_id == origin.display
                && mount.seats[usize::from(seat.seat_index)] == Some(passenger.to_bits())
        })
}

fn is_current_passenger(world: &World, passenger: Entity, driver: Entity, index: usize) -> bool {
    world
        .get::<VehiclePassenger>(passenger)
        .is_some_and(|seat| {
            seat.driver == driver.to_bits() && usize::from(seat.seat_index) == index
        })
}

pub(crate) fn reconcile_passengers(world: &mut World) {
    let passengers: Vec<_> = world
        .query::<(Entity, &VehiclePassenger)>()
        .iter(world)
        .map(|(entity, seat)| (entity, *seat))
        .collect();
    for (passenger, seat) in passengers {
        let valid = world
            .get::<PassengerOrigin>(passenger)
            .is_some_and(|origin| valid_seat(world, passenger, seat, origin));
        if valid {
            update_passenger_position(world, passenger, seat)
        } else {
            exit(world, passenger)
        }
    }
    isle::clear_departed_passengers(world);
    // Despawned passengers cannot call Exit; free their driver's occupied slot.
    let drivers: Vec<_> = world
        .query::<(Entity, &Mounted)>()
        .iter(world)
        .filter(|(_, mount)| mount.vehicle_id != 0)
        .map(|(entity, mount)| (entity, mount.seats))
        .collect();
    for (driver, seats) in drivers {
        for (index, occupant) in seats.into_iter().enumerate() {
            if let Some(occupant) = occupant {
                let seated =
                    is_current_passenger(world, Entity::from_bits(occupant), driver, index);
                if !seated {
                    world.get_mut::<Mounted>(driver).unwrap().seats[index] = None;
                    mark_dirty(world, driver);
                }
            }
        }
    }
}

fn update_passenger_position(world: &mut World, passenger: Entity, seat: VehiclePassenger) {
    let driver = Entity::from_bits(seat.driver);
    let position = *world.get::<Position>(driver).unwrap();
    let rotation = *world
        .get::<Rotation>(driver)
        .expect("Driver must have rotation");
    let origin = world
        .get::<PassengerOrigin>(passenger)
        .expect("Seated rider has a vehicle origin");
    let store = world.resource::<VehicleStore>();
    let seat_offset = store.seat_offsets[&(origin.display, seat.seat_id)];
    let point = vec(&position) + Quat::from_rotation_y(rotation.y) * seat_offset;
    world.entity_mut(passenger).insert((
        Position {
            x: point.x,
            y: point.y,
            z: point.z,
        },
        rotation,
    ));
    crate::interest::mark_interest_position_changed(
        world.get_resource::<crate::interest::InterestDirtyQueue>(),
        passenger,
    );
    mark_dirty(world, passenger);
}

fn receive_requests(world: &mut World) {
    let boards:Vec<_>=world.query_filtered::<(&PlayerLink,&mut MessageReceiver<BoardVehicle>),
        (With<crate::auth::Authenticated>,With<crate::auth::InWorld>)>().iter_mut(world)
        .flat_map(|(link,mut receiver)|receiver.receive().map(|request|(link.0,request.driver)).collect::<Vec<_>>()).collect();
    for (passenger, driver) in boards {
        let Some(driver) = Entity::try_from_bits(driver) else {
            warn!("Vehicle boarding rejected: invalid driver identity");
            continue;
        };
        if let Err(error) = board(world, passenger, driver) {
            warn!("Vehicle boarding rejected: {error}");
        }
    }
    let exits:Vec<_>=world.query_filtered::<(&PlayerLink,&mut MessageReceiver<ExitVehicle>),
        (With<crate::auth::Authenticated>,With<crate::auth::InWorld>)>().iter_mut(world)
        .filter_map(|(link,mut receiver)|(receiver.receive().count()>0).then_some(link.0)).collect();
    for passenger in exits {
        exit(world, passenger);
    }
}

pub(crate) fn mount_vehicle_id(world: &World, creature: u32) -> u32 {
    world
        .get_resource::<VehicleStore>()
        .and_then(|store| store.creature_vehicles.get(&creature))
        .copied()
        .unwrap_or(0)
}

fn load_vehicle_store(mut commands: Commands) {
    let root = crate::ground::ground_data_dir();
    let conn = rusqlite::Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("Open world.db for mount vehicles");
    let store = load_store(&root, &conn)
        .expect("Load mount vehicles from world.db; run scripts/import-vehicles.py first");
    commands.insert_resource(store);
}

fn load_vehicle_catalog(conn: &rusqlite::Connection) -> Result<VehicleStore, String> {
    let mut store = VehicleStore::default();
    let mut query = conn
        .prepare("SELECT ID,SeatID_0,SeatID_1,SeatID_2,SeatID_3,SeatID_4,SeatID_5,SeatID_6,SeatID_7 FROM vehicle")
        .map_err(|error| format!("world.db vehicle: {error}"))?;
    let rows = query
        .query_map([], |row| {
            let mut seats = [0; 8];
            for (index, seat) in seats.iter_mut().enumerate() {
                *seat = row.get(index + 1)?;
            }
            Ok((row.get(0)?, seats))
        })
        .map_err(|error| error.to_string())?;
    store.vehicles = rows
        .collect::<Result<_, _>>()
        .map_err(|error| error.to_string())?;
    store.seats = load_seats(conn)?;
    Ok(store)
}

fn load_seats(conn: &rusqlite::Connection) -> Result<HashMap<u32, Seat>, String> {
    let mut query = conn
        .prepare("SELECT ID,Flags,AttachmentOffset_0,AttachmentOffset_1,AttachmentOffset_2 FROM vehicle_seat")
        .map_err(|error| format!("world.db vehicle_seat: {error}"))?;
    let rows = query
        .query_map([], |row| {
            let flags = row.get::<_, i64>(1)? as u32;
            let offset = Vec3::new(row.get(2)?, row.get(4)?, -row.get::<_, f32>(3)?);
            Ok((row.get(0)?, Seat { flags, offset }))
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<_, _>>()
        .map_err(|error| error.to_string())
}

fn load_store(root: &std::path::Path, conn: &rusqlite::Connection) -> Result<VehicleStore, String> {
    let mut store = load_vehicle_catalog(conn)?;
    let mut query = conn
        .prepare("SELECT entry,VehicleId FROM creature_template WHERE VehicleId<>0")
        .map_err(|e| e.to_string())?;
    let rows = query
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (creature, vehicle) = row.map_err(|e| e.to_string())?;
        store.creature_vehicles.insert(creature, vehicle);
    }
    load_mount_poses(root, conn, &mut store)?;
    isle::load_catalog(root, conn, &mut store)?;
    Ok(store)
}

fn load_mount_poses(
    root: &std::path::Path,
    conn: &rusqlite::Connection,
    store: &mut VehicleStore,
) -> Result<(), String> {
    let seat_records: HashMap<u32, i32> =
        load_catalog_ids(conn, "SELECT ID,AttachmentID FROM vehicle_seat")?;
    let mut query = conn
        .prepare(include_str!("../../../scripts/vehicle_models.sql"))
        .map_err(|error| format!("world.db passenger mount models: {error}"))?;
    let rows = query
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, Option<u32>>(2)?,
                row.get::<_, Option<f32>>(3)?,
                row.get::<_, Option<u32>>(4)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    for row in rows {
        let (display_id, vehicle, model, scale, fdid) = row.map_err(|e| e.to_string())?;
        let Some(scale) = scale else {
            warn!("Vehicle display {display_id} absent from local data");
            continue;
        };
        let fdid = fdid.ok_or_else(|| format!("Display {display_id}: missing model {model:?}"))?;
        let path = root.join(format!("models/{fdid}.m2"));
        let data = match std::fs::read(&path) {
            Ok(data) => data,
            Err(error) => {
                warn!(
                    "Vehicle {vehicle} display {display_id}: {}: {error}",
                    path.display()
                );
                continue;
            }
        };
        let seats = store
            .vehicles
            .get(&vehicle)
            .ok_or_else(|| format!("Missing Vehicle {vehicle}"))?;
        for &seat in seats.iter().filter(|&&seat| {
            store.seats.get(&seat).is_some_and(|seat| {
                seat.flags & CAN_ENTER_OR_EXIT != 0 && seat.flags & CAN_CONTROL == 0
            })
        }) {
            let attachment = seat_records
                .get(&seat)
                .ok_or_else(|| format!("Missing VehicleSeat {seat}"))?;
            match read_mount_attachment(&data, *attachment) {
                Ok(position) => {
                    // WorldModels::build_creature_visual aligns NpcVisualRoot by -90deg.
                    let model_alignment = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
                    let authored_offset = store.seats[&seat].offset;
                    let unit_offset = model_alignment * ((position + authored_offset) * scale);
                    store.seat_offsets.insert((display_id, seat), unit_offset);
                }
                Err(error) => warn!("Vehicle {vehicle} seat {seat} display {display_id}: {error}"),
            }
        }
    }
    Ok(())
}

fn load_catalog_ids<T: rusqlite::types::FromSql>(
    conn: &rusqlite::Connection,
    sql: &str,
) -> Result<HashMap<u32, T>, String> {
    let mut query = conn
        .prepare(sql)
        .map_err(|error| format!("world.db {sql}: {error}"))?;
    query
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|error| error.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|error| error.to_string())
}

fn read_mount_attachment(data: &[u8], seat_enum: i32) -> Result<Vec3, String> {
    // SolarityClient native A2D3F0: VehicleSeat enum is not the M2 attachment ID.
    const ATTACHMENTS: [u32; 22] = [
        20, 34, 19, 21, 22, 17, 23, 24, 25, 15, 16, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 0,
    ];
    let attachment = *ATTACHMENTS
        .get(usize::try_from(seat_enum).map_err(|_| "Negative seat attachment")?)
        .ok_or("Invalid seat attachment")?;
    let md20 = if data.starts_with(b"MD20") {
        data
    } else {
        wow_m2::parser::parse_chunks(data)?.md20
    };
    let [x, y, z] = wow_m2::m2_attach::attachment_position(md20, attachment)?;
    Ok(Vec3::new(x, z, -y))
}

#[cfg(test)]
#[path = "vehicle_tests.rs"]
pub(crate) mod tests;
