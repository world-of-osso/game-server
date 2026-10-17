//! Persisted Retail guild invitation delivery and decline.
use crate::auth::{Authenticated, InWorld};
use crate::persistence::{self, GameDatabase};
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::protocol::{AuthChannel, GuildDeclineInviteRequest, GuildInvitation};

pub(crate) fn send_pending_invites(
    db: Res<GameDatabase>,
    mut connections: Query<
        (&InWorld, &mut MessageSender<GuildInvitation>),
        (With<Authenticated>, Added<InWorld>),
    >,
) {
    for (world, mut sender) in &mut connections {
        if let Some(invite) = load_pending_invitation(&db.db, world.character_id) {
            sender.send::<AuthChannel>(invite);
        }
    }
}

/// Guild.cpp1727–1738 sends the inviting guild's current registered emblem.
/// Pending invite storage remains unchanged; resolve its real design at delivery.
pub(crate) fn load_pending_invitation(
    db: &redb::Database,
    character: u64,
) -> Option<GuildInvitation> {
    let invite = persistence::load_pending_guild_invite(db, character)?;
    Some(GuildInvitation {
        guild_id: invite.guild_id,
        guild_name: invite.guild_name,
        inviter_name: invite.inviter_name,
        tabard: persistence::guild_tabards::load_design(db, invite.guild_id),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{components::GuildMembership, guild_creation::GuildTabardDesign, protocol::*};

    fn install_delivery(h: &mut crate::bank_test_harness::Harness) {
        // GuildPlugin already installs persisted delivery in the real harness.
        h.app
            .add_systems(Update, crate::messaging::process_guild_invites);
    }

    #[test]
    fn guild_invitation_delivers_registered_optional_tabard_immediately_and_after_reopen() {
        for registered in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("invitations.redb");
            let mut h = crate::bank_test_harness::harness_at(&path, "Tabardmaster").unwrap();
            install_delivery(&mut h);
            let owner = h.character_id;
            let guest = h.create_alt("Tabardguest");
            persistence::set_character_guild_by_id(h.db(), owner, 7, "Raid Team").unwrap();
            persistence::banks::save_guild_member_rank(h.db(), owner, 0);
            h.app
                .world_mut()
                .entity_mut(h.player)
                .insert(GuildMembership {
                    guild_id: 7,
                    guild_name: "Raid Team".into(),
                });
            let design = GuildTabardDesign {
                icon: 1,
                icon_color: 2,
                border: 3,
                border_color: 4,
                background_color: 5,
            };
            if registered {
                h.set_gold(u64::from(shared::guild_creation::TABARD_COST));
                persistence::guild_tabards::register(h.db(), owner, design).unwrap();
                assert_eq!(h.gold(), 0);
            }
            let expected = serde_json::to_value(registered.then_some(design)).unwrap();
            let connection = h.connect_character(guest, "Tabardguest");
            h.send::<_, AuthChannel>(GuildInviteRequest {
                target_character_name: "Tabardguest".into(),
            });
            assert!(h.take::<GuildInviteResponse>()[0].success);
            let immediate = h.take_for::<GuildInvitation>(connection);
            assert_eq!(immediate.len(), 1);
            assert_eq!(
                serde_json::to_value(&immediate[0]).unwrap()["tabard"],
                expected
            );
            assert_eq!(immediate[0].guild_name, "Raid Team");
            assert_eq!(immediate[0].inviter_name, "Tabardmaster");
            drop(h);
            let mut reopened = crate::bank_test_harness::harness_at(&path, "Tabardreader").unwrap();
            install_delivery(&mut reopened);
            let connection = reopened.connect_character(guest, "Tabardguest");
            for _ in 0..3 {
                reopened.app.update();
            }
            let pending = reopened.take_for::<GuildInvitation>(connection);
            assert_eq!(pending.len(), 1);
            assert_eq!(pending[0], immediate[0]);
            assert_eq!(
                serde_json::to_value(&pending[0]).unwrap()["tabard"],
                expected
            );
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
            persistence::take_pending_guild_invite(&db.db, world.character_id);
        }
    }
}
