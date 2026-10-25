//! A player's stand state (`UnitData::StandState`, TrinityCore a352b1fa
//! `Unit::SetStandState`, Unit.cpp:10802-10814): what sets it, what stands the player
//! back up and the auras that need it seated.

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Player, PlayerMotion, PlayerStandState, StandState};
use shared::protocol::StandStateIntent;

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;

/// `SpellAuraInterruptFlags::Standing` (SpellDefines.h:98): food and drink.
pub(crate) const AURA_INTERRUPT_STANDING: u32 = 0x0004_0000;

/// `SPELL_ATTR0_ALLOW_WHILE_SITTING` (SharedDefines.h:463).
const SPELL_ATTR0_ALLOW_WHILE_SITTING: u32 = 0x0800_0000;
/// `SPELL_ATTR3_DO_NOT_TRIGGER_TARGET_STAND` (SharedDefines.h:569).
const SPELL_ATTR3_DO_NOT_TRIGGER_TARGET_STAND: u32 = 0x0040_0000;

/// `MOVEMENTFLAG_MASK_MOVING` bits a `PlayerMotion` carries.
const MOVING: u64 = PlayerMotion::FORWARD
    | PlayerMotion::BACKWARD
    | PlayerMotion::STRAFE_LEFT
    | PlayerMotion::STRAFE_RIGHT
    | PlayerMotion::FALLING;

pub struct StandStatePlugin;

impl Plugin for StandStatePlugin {
    fn build(&self, app: &mut App) {
        // Unread messages do not outlive their frame, so requests are read every frame.
        app.add_systems(Update, receive_stand_state_requests);
        app.add_systems(
            FixedUpdate,
            stand_up_moving_players.after(crate::networking::process_player_inputs),
        );
    }
}

/// `Unit::SetStandState`: a player takes `state`; standing up removes its auras with
/// the Standing interrupt flag (food and drink). A unit without `PlayerStandState`
/// (a creature) is left alone.
pub(crate) fn set_stand_state(world: &mut World, unit: Entity, state: StandState) {
    let Some(mut current) = world.get_mut::<PlayerStandState>(unit) else {
        return;
    };
    if current.0 == state {
        return;
    }
    current.0 = state;
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(unit);
    }
    if state == StandState::Stand {
        crate::auras::interrupt_auras(world, unit, [AURA_INTERRUPT_STANDING, 0], None);
    }
}

/// `unit`'s stand state; a creature or a unit without one stands.
pub(crate) fn stand_state(world: &World, unit: Entity) -> StandState {
    world
        .get::<PlayerStandState>(unit)
        .map_or(StandState::Stand, |state| state.0)
}

/// `Unit::DealDamage` (Unit.cpp:1185-1186): any damage but a periodic tick stands a
/// player victim up.
pub(crate) fn stand_on_damage(world: &mut World, victim: Entity) {
    set_stand_state(world, victim, StandState::Stand);
}

/// `Spell::DoProcessTargetContainer` (Spell.cpp:3004-3005): a harmful spell of a
/// hostile caster stands its target up unless `SPELL_ATTR3_DO_NOT_TRIGGER_TARGET_STAND`.
pub(crate) fn stand_on_hostile_spell(world: &mut World, target: Entity, attributes3: u32) {
    if attributes3 & SPELL_ATTR3_DO_NOT_TRIGGER_TARGET_STAND == 0
        && !stand_state(world, target).is_stand()
    {
        set_stand_state(world, target, StandState::Stand);
    }
}

/// The Retail client stands up before it casts a spell without
/// `SPELL_ATTR0_ALLOW_WHILE_SITTING` (TrinityCore defines the attribute and leaves the
/// check to the client, which sends `CMSG_STAND_STATE_CHANGE`).
pub(crate) fn stand_to_cast(world: &mut World, caster: Entity, attributes0: u32) {
    if attributes0 & SPELL_ATTR0_ALLOW_WHILE_SITTING == 0 {
        set_stand_state(world, caster, StandState::Stand);
    }
}

/// `Unit::_ApplyAura` (Unit.cpp:3584-3586): an aura that needs its target seated sits
/// a standing one down; `SetStunned(true)` (Unit.cpp:11715-11716) stands a player up.
pub(crate) fn on_aura_applied(
    world: &mut World,
    target: Entity,
    interrupt_flags: [u32; 2],
    stuns: bool,
) {
    if stuns {
        set_stand_state(world, target, StandState::Stand);
    } else if interrupt_flags[0] & AURA_INTERRUPT_STANDING != 0
        && stand_state(world, target) == StandState::Stand
    {
        set_stand_state(world, target, StandState::Sit);
    }
}

/// `WorldSession::HandleStandStateChangeOpcode` (MiscHandler.cpp:401-415): a player may
/// ask to stand, sit, sleep or kneel; any other state is dropped.
pub(crate) fn receive_stand_state_requests(world: &mut World) {
    let mut receivers = world.query::<(
        &mut MessageReceiver<StandStateIntent>,
        &PlayerLink,
        Has<Authenticated>,
        Has<InWorld>,
    )>();
    let requests: Vec<(Entity, StandState)> = receivers
        .iter_mut(world)
        .filter(|(_, _, auth, in_world)| *auth && *in_world)
        .flat_map(|(mut receiver, link, ..)| {
            let player = link.0;
            receiver
                .receive()
                .map(move |intent| (player, intent.state))
                .collect::<Vec<_>>()
        })
        .collect();
    for (player, state) in requests {
        if matches!(
            state,
            StandState::Stand | StandState::Sit | StandState::Sleep | StandState::Kneel
        ) {
            set_stand_state(world, player, state);
        }
    }
}

/// A moving player stands up. TrinityCore stands up sitting players here
/// (`HandleMovementOpcode`, MovementHandler.cpp:589-590) and leaves sleeping and kneeling
/// ones to the client's own `CMSG_STAND_STATE_CHANGE`; this server owns movement, so it
/// stands up every state that is not standing. `PlayerInput` carries no turning flag.
pub(crate) fn stand_up_moving_players(world: &mut World) {
    let mut players =
        world.query_filtered::<(Entity, &PlayerMotion, &PlayerStandState), With<Player>>();
    let moving: Vec<Entity> = players
        .iter(world)
        .filter(|(_, motion, state)| motion.0 & MOVING != 0 && !state.0.is_stand())
        .map(|(entity, ..)| entity)
        .collect();
    for player in moving {
        set_stand_state(world, player, StandState::Stand);
    }
}

#[cfg(test)]
#[path = "stand_state_tests.rs"]
mod tests;
