//! IoC Demolisher spellclick66245 / aura236. TC a352b1fa SpellAuraEffects.cpp
//! HandleAuraControlVehicle and Vehicle.cpp AddVehiclePassenger select a source seat.
use super::*;
use crate::auras::{ActiveAuras, AuraInstance, RemoveReason};
use shared::components::{ModelDisplay, Npc};

const MAP_ID: u16 = 628;
const DEMOLISHER: u32 = 34775;
const CONTROL_VEHICLE: u32 = 236;

#[derive(Clone, Copy)]
struct Occupant {
    passenger: Entity,
    aura_id: u32,
}

#[derive(Component, Default)]
pub(super) struct NpcVehicle {
    seats: [Option<Occupant>; 8],
}

pub(super) fn load_catalog(
    root: &std::path::Path,
    conn: &rusqlite::Connection,
    store: &mut VehicleStore,
) -> Result<(), String> {
    let attachments: HashMap<u32, i32> =
        load_catalog_ids(conn, "SELECT ID,AttachmentID FROM vehicle_seat")?;
    let mut query = conn
        .prepare(include_str!("../../../../scripts/isle_vehicle_models.sql"))
        .map_err(|error| format!("IoC Demolisher source models: {error}"))?;
    let rows = query
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, Option<u32>>(4)?,
                row.get::<_, Option<f32>>(3)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    for row in rows {
        let (display, vehicle, fdid, scale) = row.map_err(|error| error.to_string())?;
        let fdid =
            fdid.ok_or_else(|| format!("Demolisher display {display} lacks source model"))?;
        let scale =
            scale.ok_or_else(|| format!("Demolisher display {display} lacks source scale"))?;
        let path = root.join(format!("models/{fdid}.m2"));
        let data = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let seats = *store
            .vehicles
            .get(&vehicle)
            .ok_or_else(|| format!("Demolisher display {display} lacks Vehicle {vehicle}"))?;
        for seat in seats {
            if seat == 0 {
                continue;
            }
            let definition = store
                .seats
                .get(&seat)
                .ok_or_else(|| format!("Demolisher lacks VehicleSeat {seat}"))?;
            if definition.flags & CAN_ENTER_OR_EXIT == 0 {
                continue;
            }
            let attachment = attachments
                .get(&seat)
                .ok_or_else(|| format!("Demolisher lacks VehicleSeat {seat} attachment"))?;
            let point = read_mount_attachment(&data, *attachment)
                .map_err(|error| format!("Demolisher display {display} seat {seat}: {error}"))?;
            let alignment = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
            let offset = alignment * ((point + definition.offset) * scale);
            store.seat_offsets.insert((display, seat), offset);
        }
        store.npc_vehicles.insert(DEMOLISHER, vehicle);
    }
    Ok(())
}

fn control_effect(aura: &AuraInstance) -> Option<&crate::auras::AuraEffectState> {
    aura.effects
        .iter()
        .find(|effect| effect.aura_type == CONTROL_VEHICLE)
}

fn is_demolisher(world: &World, vehicle: Entity) -> bool {
    world
        .get::<Npc>(vehicle)
        .is_some_and(|npc| npc.template_id == DEMOLISHER)
        && world
            .get::<InMap>(vehicle)
            .is_some_and(|map| map.map_id == MAP_ID)
}

pub(crate) fn on_applied(world: &mut World, vehicle: Entity, aura: &AuraInstance) {
    let Some(effect) = control_effect(aura) else {
        return;
    };
    if !is_demolisher(world, vehicle) {
        return;
    }
    let seat_index = effect.amount as i32 - 1;
    if let Err(reason) = board(world, aura.caster, vehicle, aura.instance_id, seat_index) {
        warn!(?vehicle, rider=?aura.caster, reason, "IoC vehicle boarding rejected");
        let aura_id = aura.instance_id;
        // Finish AuraApplied before its failed application is removed and announced.
        world.commands().queue(move |world: &mut World| {
            crate::auras::remove_auras(world, vehicle, RemoveReason::Cancelled, |aura| {
                aura.instance_id == aura_id
            });
        });
    }
}

fn board(
    world: &mut World,
    passenger: Entity,
    vehicle: Entity,
    aura_id: u32,
    requested_seat: i32,
) -> Result<(), String> {
    if passenger == vehicle || !alive(world, passenger) {
        return Err("Rider must be a living player".into());
    }
    let vehicle_alive = world
        .get::<Health>(vehicle)
        .is_some_and(|health| health.current > 0.0)
        && world
            .get::<crate::networking::PendingDespawn>(vehicle)
            .is_none();
    if !vehicle_alive || world.get::<InMap>(passenger) != world.get::<InMap>(vehicle) {
        return Err("Vehicle must be alive in the rider's map copy".into());
    }
    if world.get::<VehiclePassenger>(passenger).is_some()
        || world.get::<Mounted>(passenger).is_some()
    {
        return Err("Rider already occupies a mount or vehicle".into());
    }
    let display = world
        .get::<ModelDisplay>(vehicle)
        .ok_or("Vehicle model absent")?
        .display_id;
    let (index, seat_id) = select_seat(world, vehicle, display, requested_seat)?;
    let origin = PassengerOrigin {
        map: *world.get::<InMap>(vehicle).unwrap(),
        display,
    };
    let seat = VehiclePassenger {
        driver: vehicle.to_bits(),
        seat_index: index as u8,
        seat_id,
    };
    if world.get::<NpcVehicle>(vehicle).is_none() {
        world.entity_mut(vehicle).insert(NpcVehicle::default());
    }
    world.get_mut::<NpcVehicle>(vehicle).unwrap().seats[index] =
        Some(Occupant { passenger, aura_id });
    world.entity_mut(passenger).insert((seat, origin));
    set_control(world, passenger, true);
    update_passenger_position(world, passenger, seat);
    Ok(())
}

fn select_seat(
    world: &World,
    vehicle: Entity,
    display: u32,
    requested: i32,
) -> Result<(usize, u32), String> {
    let store = world.resource::<VehicleStore>();
    let vehicle_id = store
        .npc_vehicles
        .get(&DEMOLISHER)
        .ok_or("Demolisher source VehicleId absent")?;
    let seats = store
        .vehicles
        .get(vehicle_id)
        .ok_or("Demolisher source seats absent")?;
    let occupied = world.get::<NpcVehicle>(vehicle);
    seats
        .iter()
        .copied()
        .enumerate()
        .find(|(index, seat)| {
            let available = occupied.is_none_or(|state| state.seats[*index].is_none());
            let usable = store
                .seats
                .get(seat)
                .is_some_and(|seat| seat.flags & CAN_ENTER_OR_EXIT != 0);
            let selected = requested < 0 || *index == requested as usize;
            available && usable && selected && store.seat_offsets.contains_key(&(display, *seat))
        })
        .ok_or_else(|| "No available authored Demolisher seat".into())
}

pub(super) fn valid_seat(
    world: &World,
    passenger: Entity,
    seat: VehiclePassenger,
    origin: &PassengerOrigin,
) -> bool {
    let vehicle = Entity::from_bits(seat.driver);
    let occupant = world
        .get::<NpcVehicle>(vehicle)
        .and_then(|state| state.seats[usize::from(seat.seat_index)]);
    world
        .get::<Health>(vehicle)
        .is_some_and(|health| health.current > 0.0)
        && world
            .get::<crate::networking::PendingDespawn>(vehicle)
            .is_none()
        && world.get::<InMap>(vehicle) == Some(&origin.map)
        && world
            .get::<ModelDisplay>(vehicle)
            .is_some_and(|model| model.display_id == origin.display)
        && occupant.is_some_and(|occupant| {
            occupant.passenger == passenger
                && world.get::<ActiveAuras>(vehicle).is_some_and(|auras| {
                    auras.iter().any(|aura| {
                        aura.instance_id == occupant.aura_id && aura.caster == passenger
                    })
                })
        })
}

pub(crate) fn on_removed(world: &mut World, vehicle: Entity, aura: &AuraInstance) {
    if control_effect(aura).is_none() {
        return;
    }
    let seated = world.get::<NpcVehicle>(vehicle).is_some_and(|state| {
        state.seats.iter().flatten().any(|occupant| {
            occupant.passenger == aura.caster && occupant.aura_id == aura.instance_id
        })
    });
    if seated {
        super::exit(world, aura.caster);
    }
}

pub(super) fn release_seat(world: &mut World, passenger: Entity, seat: VehiclePassenger) {
    let vehicle = Entity::from_bits(seat.driver);
    let aura_id = world.get_mut::<NpcVehicle>(vehicle).and_then(|mut state| {
        let slot = &mut state.seats[usize::from(seat.seat_index)];
        if slot.is_some_and(|occupant| occupant.passenger == passenger) {
            slot.take().map(|occupant| occupant.aura_id)
        } else {
            None
        }
    });
    if let Some(aura_id) = aura_id {
        crate::auras::remove_auras(world, vehicle, RemoveReason::Cancelled, |aura| {
            aura.instance_id == aura_id
        });
    }
}

pub(super) fn clear_departed_passengers(world: &mut World) {
    let departed: Vec<_> = world
        .query::<(Entity, &NpcVehicle)>()
        .iter(world)
        .flat_map(|(vehicle, state)| {
            state
                .seats
                .iter()
                .enumerate()
                .filter_map(|(index, occupant)| {
                    let occupant = occupant.as_ref()?;
                    let seated = world
                        .get::<VehiclePassenger>(occupant.passenger)
                        .is_some_and(|seat| {
                            seat.driver == vehicle.to_bits()
                                && usize::from(seat.seat_index) == index
                        });
                    (!seated).then_some((vehicle, index, occupant.aura_id))
                })
                .collect::<Vec<_>>()
        })
        .collect();
    for (vehicle, index, aura_id) in departed {
        world.get_mut::<NpcVehicle>(vehicle).unwrap().seats[index] = None;
        crate::auras::remove_auras(world, vehicle, RemoveReason::CasterGone, |aura| {
            aura.instance_id == aura_id
        });
    }
}
