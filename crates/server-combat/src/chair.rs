//! Chairs (`GAMEOBJECT_TYPE_CHAIR`, TrinityCore a352b1fa `GameObject::Use`,
//! GameObject.cpp:2780-2852): using one moves the player onto its nearest free seat,
//! facing the chair's way, and seats it at the chair's height.

use bevy::prelude::*;
use shared::components::{PlayerStandState, Position, Rotation, StandState};
use shared::protocol::GameObjectInfo;

use crate::game_objects::wow_position;

/// `DEFAULT_VISIBILITY_DISTANCE`: a seat farther than this from the user is never
/// the nearest.
const DEFAULT_VISIBILITY_DISTANCE: f32 = 90.0;
/// A seat's last user still sits on it when within this of the seat (2D).
const SEAT_RADIUS: f32 = 0.1;

/// A chair's seats: `chairslots` (Data0) on a line across its facing, `chairheight`
/// (Data1) picking the low, medium or high sitting pose, and each seat's last user.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Chair {
    pub height: u32,
    /// Last user of each seat; a template without slots has one seat.
    users: Vec<Option<Entity>>,
    /// `chairslots` as the template has it, for the seat spacing.
    slots: u32,
}

impl Chair {
    pub fn new(slots: u32, height: u32) -> Self {
        Self {
            height,
            users: vec![None; slots.max(1) as usize],
            slots,
        }
    }

    /// WoW (x, y) of `seat` on a chair of template `size` at `origin` facing
    /// `orientation`: seats sit `size` apart, centred on the chair, along its facing
    /// + π/2.
    fn seat_position(&self, seat: usize, origin: Vec3, orientation: f32, size: f32) -> Vec2 {
        let relative = size * seat as f32 - size * (self.slots as f32 - 1.0) / 2.0;
        let across = orientation + std::f32::consts::FRAC_PI_2;
        Vec2::new(
            origin.x + relative * across.cos(),
            origin.y + relative * across.sin(),
        )
    }
}

/// `StandState` a chair of `height` seats its user in (`UNIT_STAND_STATE_SIT_LOW_CHAIR`
/// + height); `None` for a height without one (ObjectMgr.cpp:7516 rejects above 2).
fn seated_state(height: u32) -> Option<StandState> {
    match height {
        0 => Some(StandState::SitLowChair),
        1 => Some(StandState::SitMediumChair),
        2 => Some(StandState::SitHighChair),
        _ => None,
    }
}

/// Whether `user` still occupies the seat at `seat` (sitting on a chair, not on the
/// ground, within `SEAT_RADIUS`).
fn occupies(world: &World, user: Entity, seat: Vec2) -> bool {
    let state = world
        .get::<PlayerStandState>(user)
        .map_or(StandState::Stand, |state| state.0);
    let Some(position) = world.get::<Position>(user) else {
        return false;
    };
    state.is_sit()
        && state != StandState::Sit
        && wow_position(position).truncate().distance(seat) < SEAT_RADIUS
}

/// Seat `user` on `chair`'s nearest free seat: it moves there at the chair's height
/// and facing (`NearTeleportTo`) and takes the chair's sitting state. A chair with
/// every seat taken does nothing.
pub(crate) fn sit(world: &mut World, user: Entity, chair: Entity) {
    let (Some(info), Some(position), Some(mut seats)) = (
        world.get::<GameObjectInfo>(chair).cloned(),
        world.get::<Position>(chair).copied(),
        world.get::<Chair>(chair).cloned(),
    ) else {
        return;
    };
    let Some(state) = seated_state(seats.height) else {
        warn!(
            "Chair {} has height {} without a seat",
            info.entry, seats.height
        );
        return;
    };
    let Some(user_position) = world.get::<Position>(user).copied() else {
        return;
    };
    let origin = wow_position(&position);
    let orientation = world
        .get::<crate::game_objects::GameObjectBounds>(chair)
        .map_or(0.0, |bounds| bounds.orientation);
    let user_at = wow_position(&user_position).truncate();
    let mut nearest: Option<(usize, Vec2, f32)> = None;
    for seat in 0..seats.users.len() {
        let at = seats.seat_position(seat, origin, orientation, info.scale);
        if let Some(previous) = seats.users[seat] {
            if world.get_entity(previous).is_ok() && occupies(world, previous, at) {
                continue;
            }
            seats.users[seat] = None;
        }
        let distance = user_at.distance(at);
        if distance <= nearest.map_or(DEFAULT_VISIBILITY_DISTANCE, |(.., lowest)| lowest) {
            nearest = Some((seat, at, distance));
        }
    }
    let Some((seat, at, _)) = nearest else {
        world.entity_mut(chair).insert(seats);
        return;
    };
    seats.users[seat] = Some(user);
    world.entity_mut(chair).insert(seats);
    move_onto_seat(world, user, Vec3::new(at.x, at.y, origin.z), orientation);
    crate::stand_state::set_stand_state(world, user, state);
}

/// `Unit::NearTeleportTo`: the player stands at `wow` facing `orientation`; its client
/// adopts the position on the new movement epoch.
fn move_onto_seat(world: &mut World, user: Entity, wow: Vec3, orientation: f32) {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(wow.x, wow.y, wow.z);
    world.entity_mut(user).insert((
        Position { x, y, z },
        // Same facing convention as spawned units (`game_objects::game_object_bundle`).
        Rotation {
            x: 0.0,
            y: orientation + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    crate::teleport::teleported_in(world, user);
    if let Some(replication) = world.get_resource::<lightyear::prelude::ReplicationDirtyQueue>() {
        replication.mark(user);
    }
}

#[cfg(test)]
#[path = "chair_tests.rs"]
mod tests;
