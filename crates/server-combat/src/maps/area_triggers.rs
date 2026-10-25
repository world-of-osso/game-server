//! Area trigger teleports (TrinityCore `WorldSession::HandleAreaTriggerOpcode`,
//! MiscHandler.cpp:479-640). The Retail client reports entering a trigger; here the
//! server sees the authoritative position cross into one. A player's first position
//! on a map only records the triggers it stands in (it arrived there, it did not
//! walk in).

use bevy::prelude::*;
use shared::components::{MovementControl, Player, Position};
use shared::death::DeathState;

use super::{InMap, MapStore, TeleportOptions, teleport_to};
use crate::death::PlayerDeathState;

/// The teleport trigger the player stands in on its current map.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AreaTriggerPresence {
    map: InMap,
    inside: Option<u32>,
}

type TriggerPlayers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Position,
        &'static InMap,
        &'static MovementControl,
        Option<&'static PlayerDeathState>,
        Option<&'static AreaTriggerPresence>,
    ),
    (With<Player>, Changed<Position>),
>;

pub(super) fn detect_area_trigger_teleports(
    players: TriggerPlayers,
    store: Option<Res<MapStore>>,
    mut commands: Commands,
) {
    let Some(store) = store else { return };
    for (player, position, &map, control, death, presence) in &players {
        let inside = trigger_at(&store, map.map_id, position, death);
        let entered = match presence {
            Some(presence) if presence.map == map => {
                inside.filter(|&trigger| presence.inside != Some(trigger))
            }
            _ => None,
        };
        if presence.is_none_or(|presence| presence.map != map || presence.inside != inside) {
            commands
                .entity(player)
                .insert(AreaTriggerPresence { map, inside });
        }
        // IsInFlight: a taxi passenger triggers nothing.
        let Some(trigger) = entered.filter(|_| !control.controlled) else {
            continue;
        };
        let Some(target) = store
            .area_triggers_on(map.map_id)
            .iter()
            .find(|at| at.id == trigger)
            .map(|at| at.target)
        else {
            continue;
        };
        info!(
            "{player} entered area trigger {trigger} to map {}",
            target.map_id
        );
        commands.queue(move |world: &mut World| {
            teleport_to(world, player, target, TeleportOptions::default());
        });
    }
}

/// The first teleport trigger of `map_id` containing the player (`Player::IsInAreaTrigger`:
/// a ghost only uses triggers whose action set allows ghosts; a corpse none).
fn trigger_at(
    store: &MapStore,
    map_id: u32,
    position: &Position,
    death: Option<&PlayerDeathState>,
) -> Option<u32> {
    let state = death.map_or(DeathState::Alive, |death| death.state);
    if state == DeathState::Dead {
        return None;
    }
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let point = Vec3::new(x, y, z);
    store
        .area_triggers_on(map_id)
        .iter()
        .find(|at| (state != DeathState::Ghost || at.allow_while_ghost) && at.contains(point))
        .map(|at| at.id)
}
