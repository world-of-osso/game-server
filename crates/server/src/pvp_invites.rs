use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{PvpStateUpdate, RespondToBattlegroundInvite};

use super::pvp_support::{send_pvp_error, send_pvp_update};
use super::{PlayerPvpState, PvpServerState, respond_to_battleground_invitation};
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;

pub(super) fn process_battleground_invitation_responses(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<RespondToBattlegroundInvite>,
        ),
        With<Authenticated>,
    >,
    mut players: Query<&mut PlayerPvpState>,
    mut state: ResMut<PvpServerState>,
    mut senders: Query<&mut MessageSender<PvpStateUpdate>>,
) {
    for (connection, link, in_world, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            let Ok(mut player_state) = players.get_mut(link.0) else {
                send_pvp_error(connection, &mut senders, "pvp unavailable for player");
                continue;
            };
            let result = apply_invitation_response(
                &mut player_state,
                &mut state,
                in_world.character_id,
                request,
            );
            let (message, error) = match result {
                Ok(message) => (Some(message), None),
                Err(error) => (None, Some(error)),
            };
            send_pvp_update(connection, &player_state, message, error, &mut senders);
        }
    }
}

fn apply_invitation_response(
    player: &mut PlayerPvpState,
    state: &mut PvpServerState,
    character_id: u64,
    request: RespondToBattlegroundInvite,
) -> Result<String, String> {
    let invitation = player
        .battleground
        .as_mut()
        .ok_or("player has no battleground invitation")?;
    if invitation.match_id != request.match_id {
        return Err("battleground invitation is no longer available".into());
    }
    respond_to_battleground_invitation(state, request.match_id, character_id, request.accepted)?;
    if request.accepted {
        invitation.accepted = true;
        Ok(format!(
            "{} invitation accepted; map entry pending",
            invitation.name
        ))
    } else {
        let name = invitation.name.clone();
        player.battleground = None;
        Ok(format!("declined {} invitation", name))
    }
}
