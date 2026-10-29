//! TC a352b1fa `Vehicle::InstallAllAccessories`: a spawned vehicle creature summons its
//! `vehicle_template_accessory` riders (TDB1210 `tdb_vehicle_template_accessory`) and
//! holds each at its seat. Installed in instance copies only. Seat position uses the
//! vehicle display's M2 seat attachment like player mounts.
//! VEHICLE_DEATH_CREDITS_BOUND_ACCESSORY: when the vehicle dies, a seated accessory
//! bound to an encounter completes it and every accessory is unsummoned. TC exits a
//! non-minion rider alive; this credit rule is ours (Ol' Waxbeard 210153/210149:
//! retail ENCOUNTER_END follows the mount's death, the rider takes no damage).
use super::*;
use shared::components::{ModelDisplay, Npc};

#[derive(Clone, Copy)]
struct AccessoryRow {
    entry: u32,
    /// TC seat -1: the first free seat.
    seat_index: Option<usize>,
}

#[derive(Resource, Default, Clone)]
pub(crate) struct VehicleAccessories {
    by_vehicle: HashMap<u32, Vec<AccessoryRow>>,
    /// Seat offset in the vehicle unit frame by (display, seat index).
    seat_offsets: HashMap<(u32, usize), Vec3>,
}

/// A summoned rider held at its vehicle's seat.
#[derive(Component, Clone, Copy)]
pub(crate) struct Accessory {
    vehicle: Entity,
    offset: Vec3,
}

pub(super) fn load(
    root: &std::path::Path,
    conn: &rusqlite::Connection,
    store: &VehicleStore,
) -> Result<VehicleAccessories, String> {
    let mut accessories = VehicleAccessories::default();
    let mut query = conn
        .prepare("SELECT entry,accessory_entry,seat_id FROM tdb_vehicle_template_accessory")
        .map_err(|error| {
            format!("world.db tdb_vehicle_template_accessory: {error}; run scripts/import-vehicle-accessories.py")
        })?;
    let rows = query
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    for row in rows {
        let (vehicle, entry, seat) = row.map_err(|error| error.to_string())?;
        let seat_index = (seat >= 0).then_some(seat as usize);
        accessories
            .by_vehicle
            .entry(vehicle)
            .or_default()
            .push(AccessoryRow { entry, seat_index });
    }
    load_seat_offsets(root, conn, store, &mut accessories)?;
    Ok(accessories)
}

/// Spawned vehicles only; other TDB rows keep their data without model reads.
fn load_seat_offsets(
    root: &std::path::Path,
    conn: &rusqlite::Connection,
    store: &VehicleStore,
    accessories: &mut VehicleAccessories,
) -> Result<(), String> {
    let attachments: HashMap<u32, i32> =
        load_catalog_ids(conn, "SELECT ID,AttachmentID FROM vehicle_seat")?;
    let mut query = conn
        .prepare(include_str!(
            "../../../../scripts/accessory_vehicle_models.sql"
        ))
        .map_err(|error| format!("accessory vehicle models: {error}"))?;
    let rows = query
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, Option<f32>>(3)?,
                row.get::<_, Option<u32>>(4)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    for row in rows {
        let (display_id, vehicle, scale, fdid) = row.map_err(|error| error.to_string())?;
        let (Some(scale), Some(fdid)) = (scale, fdid) else {
            warn!("Accessory vehicle {vehicle} display_id {display_id}: model metadata absent");
            continue;
        };
        let path = root.join(format!("models/{fdid}.m2"));
        let data = match std::fs::read(&path) {
            Ok(data) => data,
            Err(error) => {
                warn!(
                    "Accessory vehicle {vehicle} display_id {display_id}: {}: {error}",
                    path.display()
                );
                continue;
            }
        };
        let seats = store
            .vehicles
            .get(&vehicle)
            .ok_or_else(|| format!("Missing Vehicle {vehicle}"))?;
        for (index, seat) in seats.iter().enumerate().filter(|(_, seat)| **seat != 0) {
            let definition = store
                .seats
                .get(seat)
                .ok_or_else(|| format!("Missing VehicleSeat {seat}"))?;
            let attachment = attachments
                .get(seat)
                .ok_or_else(|| format!("Missing VehicleSeat {seat}"))?;
            match read_mount_attachment(&data, *attachment) {
                Ok(point) => {
                    let alignment = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
                    let offset = alignment * ((point + definition.offset) * scale);
                    accessories.seat_offsets.insert((display_id, index), offset);
                }
                Err(error) => {
                    warn!(
                        "Accessory vehicle {vehicle} seat {seat} display_id {display_id}: {error}"
                    )
                }
            }
        }
    }
    Ok(())
}

/// `Vehicle::InstallAllAccessories` for vehicles that entered an instance copy.
pub(crate) fn install_accessories(
    added: Query<(Entity, &Npc, &InMap, &ModelDisplay), Added<Npc>>,
    accessories: Option<Res<VehicleAccessories>>,
    mut commands: Commands,
) {
    let Some(accessories) = accessories else {
        return;
    };
    for (vehicle, npc, map, model) in &added {
        let Some(rows) = accessories.by_vehicle.get(&npc.template_id) else {
            continue;
        };
        if map.instance_id == 0 {
            continue;
        }
        let mut taken: Vec<usize> = rows.iter().filter_map(|row| row.seat_index).collect();
        for row in rows.clone() {
            let seat = row.seat_index.or_else(|| {
                let free = (0..8).find(|index| {
                    !taken.contains(index)
                        && accessories
                            .seat_offsets
                            .contains_key(&(model.display_id, *index))
                })?;
                taken.push(free);
                Some(free)
            });
            let Some(&offset) =
                seat.and_then(|seat| accessories.seat_offsets.get(&(model.display_id, seat)))
            else {
                error!(
                    "Vehicle {} display {} lacks seat {:?} for accessory {}",
                    npc.template_id, model.display_id, row.seat_index, row.entry
                );
                continue;
            };
            commands.queue(move |world: &mut World| {
                install(world, vehicle, row.entry, offset);
            });
        }
    }
}

fn install(world: &mut World, vehicle: Entity, entry: u32, offset: Vec3) {
    let (Some(position), Some(rotation)) = (
        world.get::<Position>(vehicle).copied(),
        world.get::<Rotation>(vehicle).copied(),
    ) else {
        return;
    };
    let point = seat_point(&position, &rotation, offset);
    // summon_creature takes WoW coordinates (x, -z, y); spawn facing adds a quarter turn.
    let facing = rotation.y - std::f32::consts::FRAC_PI_2;
    let wow = [point.x, -point.z, point.y, facing];
    if let Some(rider) = crate::zones::summon_creature(world, vehicle, entry, wow) {
        world
            .entity_mut(rider)
            .insert(Accessory { vehicle, offset });
    }
}

fn seat_point(position: &Position, rotation: &Rotation, offset: Vec3) -> Vec3 {
    vec(position) + Quat::from_rotation_y(rotation.y) * offset
}

/// Riders follow their seat after their own movement for the tick.
pub(crate) fn seat_accessories(world: &mut World) {
    let riders: Vec<_> = world
        .query::<(Entity, &Accessory)>()
        .iter(world)
        .map(|(rider, accessory)| (rider, *accessory))
        .collect();
    for (rider, accessory) in riders {
        let (Some(position), Some(rotation)) = (
            world.get::<Position>(accessory.vehicle).copied(),
            world.get::<Rotation>(accessory.vehicle).copied(),
        ) else {
            // Vehicle::Uninstall: a removed vehicle takes its riders along.
            crate::zones::unsummon(world, rider);
            continue;
        };
        let point = seat_point(&position, &rotation, accessory.offset);
        if world
            .get::<Position>(rider)
            .is_some_and(|current| vec(current) == point)
        {
            continue;
        }
        world.entity_mut(rider).insert((
            Position {
                x: point.x,
                y: point.y,
                z: point.z,
            },
            rotation,
        ));
        crate::interest::mark_interest_position_changed(
            world.get_resource::<crate::interest::InterestDirtyQueue>(),
            rider,
        );
        mark_dirty(world, rider);
    }
}

/// VEHICLE_DEATH_CREDITS_BOUND_ACCESSORY (ours; see module docs).
pub(crate) fn on_vehicle_death(world: &mut World, vehicle: Entity) {
    let riders: Vec<_> = world
        .query::<(Entity, &Accessory, &Npc)>()
        .iter(world)
        .filter(|(_, accessory, _)| accessory.vehicle == vehicle)
        .map(|(rider, _, npc)| (rider, npc.template_id))
        .collect();
    let Some(&map) = world.get::<InMap>(vehicle) else {
        return;
    };
    for (rider, entry) in riders {
        crate::scripted_ai::instance_script::complete_bound_entry(world, map, entry, rider);
        crate::zones::unsummon(world, rider);
    }
}
