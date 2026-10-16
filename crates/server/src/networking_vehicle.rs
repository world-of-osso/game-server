//! Source NPC mover authority, separate from the seated player's body.
//! TC a352b1fa Vehicle.cpp:895-899 selects the controlling seat; VehicleDefines.h
//! supplies movement restrictions. Vehicle509 TurnSpeed and NPC template speeds
//! remain authored data, not player defaults.
use super::*;
use crate::vehicle::{PassengerOrigin, VehicleMovement, VehicleStore, isle::NpcVehicle};
use bevy::ecs::system::SystemParam;
use shared::components::{CreatureMotion, Health, ModelDisplay, Npc, VehiclePassenger};

const NO_STRAFE: u32 = 0x00000001;
const NO_JUMPING: u32 = 0x00000002;
const FIXED_POSITION: u32 = 0x00200000;
const CAN_CONTROL: u32 = 0x00000800;

type Rider = (
    &'static VehiclePassenger,
    &'static PassengerOrigin,
    &'static Health,
    &'static InMap,
    Has<crate::networking::PendingDespawn>,
);
type Host = (
    &'static Npc,
    &'static NpcVehicle,
    &'static crate::auras::ActiveAuras,
    &'static ModelDisplay,
    &'static Health,
    &'static InMap,
    &'static crate::creature_ai::SpawnPoint,
    Has<crate::networking::PendingDespawn>,
);
type HostMovement = (
    &'static mut Position,
    &'static mut Rotation,
    &'static mut MovementSpeed,
    &'static mut CreatureMotion,
    Option<&'static AuraStatMods>,
);

#[derive(SystemParam)]
pub(crate) struct DriverInputs<'w, 's> {
    riders: Query<'w, 's, Rider, With<Player>>,
    hosts: Query<'w, 's, Host, (With<Npc>, Without<Player>)>,
    movement: Query<'w, 's, HostMovement, (With<Npc>, Without<Player>)>,
    store: Option<Res<'w, VehicleStore>>,
}

pub(crate) struct NpcMover {
    pub entity: Entity,
    pub map: InMap,
    movement: VehicleMovement,
    walk: f32,
    run: f32,
}

impl DriverInputs<'_, '_> {
    pub(crate) fn authorized_mover(&self, player: Entity) -> Option<NpcMover> {
        let (seat, origin, rider_health, rider_map, rider_departed) =
            self.riders.get(player).ok()?;
        if rider_health.current <= 0.0 || rider_departed || *rider_map != origin.map {
            return None;
        }
        let entity = Entity::try_from_bits(seat.driver)?;
        let (npc, host, auras, display, health, map, spawn, departed) =
            self.hosts.get(entity).ok()?;
        let alive_in_copy = health.current > 0.0 && !departed && *map == origin.map;
        if !alive_in_copy || display.display_id != origin.display {
            return None;
        }
        let store = self.store.as_ref()?;
        let vehicle = store.npc_vehicles.get(&npc.template_id)?;
        let source_seat = store
            .vehicles
            .get(vehicle)?
            .get(usize::from(seat.seat_index))?;
        let controlling = store.seats.get(source_seat)?.flags & CAN_CONTROL != 0;
        if *source_seat != seat.seat_id
            || !controlling
            || !host.owns_control_aura(player, seat.seat_index, auras)
        {
            return None;
        }
        Some(NpcMover {
            entity,
            map: *map,
            movement: *store.movement.get(vehicle)?,
            walk: DEFAULT_WALK_SPEED * spawn.speed_walk,
            run: DEFAULT_RUN_SPEED * spawn.speed_run,
        })
    }

    pub(crate) fn apply_inputs(
        &mut self,
        mover: &NpcMover,
        player: Entity,
        inputs: &[PlayerInput],
        clocks: &mut MovementClocks,
        mut ground: Option<&mut GroundMap>,
    ) -> MovementChanges {
        let Ok((mut position, mut rotation, mut speed, mut motion, auras)) =
            self.movement.get_mut(mover.entity)
        else {
            return MovementChanges::default();
        };
        let mut changes = MovementChanges::default();
        for input in inputs {
            let terrain = ground.as_deref_mut().map(|ground| (ground, mover.map));
            let step = move_vehicle(
                input, mover, player, clocks, terrain, *position, rotation.y, auras,
            );
            changes = changes.merge(MovementChanges {
                position_changed: *position != step.position,
                replicated_state_changed: *position != step.position
                    || rotation.y != step.yaw
                    || speed.0 != step.speed
                    || *motion != step.motion,
                behind: step.behind,
            });
            position.set_if_neq(step.position);
            rotation.y = step.yaw;
            speed.set_if_neq(MovementSpeed(step.speed));
            motion.set_if_neq(step.motion);
        }
        changes
    }
}

struct VehicleStep {
    position: Position,
    yaw: f32,
    speed: f32,
    motion: CreatureMotion,
    behind: bool,
}

fn move_vehicle(
    input: &PlayerInput,
    mover: &NpcMover,
    player: Entity,
    clocks: &mut MovementClocks,
    mut terrain: Option<(&mut GroundMap, InMap)>,
    position: Position,
    yaw: f32,
    auras: Option<&AuraStatMods>,
) -> VehicleStep {
    let current = Vec3::new(position.x, position.y, position.z);
    let base = if input.running { mover.run } else { mover.walk };
    let speed = base
        * auras.map_or(1.0, |mods| mods.movement_speed)
        * movement_speed_multiplier(yaw, &input.direction, direction_length(&input.direction));
    let reported = Vec3::from(input.position);
    if !reported.is_finite() || !input.facing_yaw.is_finite() || !speed.is_finite() || speed <= 0.0
    {
        return VehicleStep {
            position,
            yaw,
            speed: 0.0,
            motion: CreatureMotion::Still,
            behind: false,
        };
    }
    // Turn time comes from the same bank as travel: a packet burst buys neither
    // extra distance nor an instantaneous vehicle turn. Stationary reports can turn.
    let yaw_delta = (input.facing_yaw - yaw + std::f32::consts::PI)
        .rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    let turn_secs = if mover.movement.turn_speed > 0.0 {
        yaw_delta.abs() / mover.movement.turn_speed
    } else {
        0.0
    };
    let travel_secs = current.xz().distance(reported.xz()) / speed;
    let granted = clocks.grant(player, travel_secs.max(turn_secs));
    let next_yaw = yaw
        + yaw_delta.clamp(
            -mover.movement.turn_speed * granted,
            mover.movement.turn_speed * granted,
        );
    let mut target = reported;
    if mover.movement.flags & FIXED_POSITION != 0 {
        target = current;
    } else if mover.movement.flags & NO_STRAFE != 0 {
        let forward = Vec3::new(next_yaw.sin(), 0.0, next_yaw.cos());
        let distance = (reported - current).dot(forward);
        target = current + forward * distance;
    }
    let (mut next, behind) =
        move_toward_reported(current, target, speed, Travel::Ground, |_| granted);
    if let Some((ground, map)) = terrain.as_mut() {
        next = ground.clamp_live_movement(*map, current, next);
        let envelope = LandEnvelope {
            airborne: None,
            jumping: input.jumping && mover.movement.flags & NO_JUMPING == 0,
            feather_fall: false,
        };
        next.y = land_height(ground, *map, current, next, envelope);
    } else {
        next.y = current.y;
    }
    let moved = current.xz().distance(next.xz()) > 0.001;
    let motion = if !moved {
        CreatureMotion::Still
    } else if input.running {
        CreatureMotion::Run
    } else {
        CreatureMotion::Walk
    };
    VehicleStep {
        position: Position {
            x: next.x,
            y: next.y,
            z: next.z,
        },
        yaw: next_yaw,
        speed: if moved { speed } else { 0.0 },
        motion,
        behind: behind || yaw_delta.abs() > mover.movement.turn_speed * granted + 0.001,
    }
}
