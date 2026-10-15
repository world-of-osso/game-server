//! Persisted Retail guild invitation delivery and decline.
use crate::auth::{Authenticated, InWorld};
use crate::persistence::{self, GameDatabase};
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::protocol::{AuthChannel, GuildDeclineInviteRequest, GuildInvitation};

#[cfg(test)]
#[path = "guild_decline_fifo_tests.rs"]
mod fifo_tests;

pub(crate) fn send_pending_invites(
    db: Res<GameDatabase>,
    mut connections: Query<
        (&InWorld, &mut MessageSender<GuildInvitation>),
        (With<Authenticated>, Added<InWorld>),
    >,
) {
    for (world, mut sender) in &mut connections {
        if let Some(invite) = persistence::load_pending_guild_invite(&db.db, world.character_id) {
            sender.send::<AuthChannel>(GuildInvitation {
                guild_id: invite.guild_id,
                guild_name: invite.guild_name,
                inviter_name: invite.inviter_name,
            });
        }
    }
}

pub(crate) fn decline_invites(
    db: Res<GameDatabase>,
    mut connections: Query<
        (&InWorld, &mut MessageReceiver<GuildDeclineInviteRequest>),
        With<Authenticated>,
    >,
) {
    for (world, mut receiver) in &mut connections {
        for _ in receiver.receive() {
            let character = world.character_id;
            // This packet has no acknowledgement; the writer logs/counts errors and drains on shutdown.
            db.writer.enqueue("guild invite decline", move |db| {
                persistence::take_pending_guild_invite(db, character);
                Ok(())
            });
        }
    }
}
