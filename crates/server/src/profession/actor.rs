//! Owned profession snapshots commit on FIFO before owner publication.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

struct PendingSnapshot {
    player: Entity,
    character: u64,
    receipt: CommitReceipt<ProfessionSnapshot>,
}

#[derive(Resource, Default)]
pub(super) struct PendingProfessionSnapshots(VecDeque<PendingSnapshot>);

impl PendingProfessionSnapshots {
    pub(super) fn enqueue(
        &mut self,
        db: &GameDatabase,
        player: Entity,
        character: u64,
        professions: PlayerProfessions,
    ) {
        let receipt = db.writer.enqueue("profession snapshot", move |db| {
            persistence::save_professions(db, character, &professions);
            Ok(professions.snapshot())
        });
        self.0.push_back(PendingSnapshot {
            player,
            character,
            receipt,
        });
    }
}

pub(crate) struct ProfessionPersistencePlugin;

impl ProfessionPersistencePlugin {
    pub(crate) fn install(app: &mut App) {
        app.init_resource::<PendingProfessionSnapshots>()
            .add_systems(
                PostUpdate,
                (enqueue_profession_snapshots, publish_profession_snapshots).chain(),
            );
    }
}

impl Plugin for ProfessionPersistencePlugin {
    fn build(&self, app: &mut App) {
        Self::install(app);
    }
}

fn publish_profession_snapshots(
    mut pending: ResMut<PendingProfessionSnapshots>,
    mut connections: Query<
        (
            &PlayerLink,
            &InWorld,
            &mut MessageSender<ProfessionSnapshot>,
        ),
        With<Authenticated>,
    >,
) {
    while let Some(result) = pending
        .0
        .front()
        .and_then(|pending| pending.receipt.result())
    {
        let pending = pending.0.pop_front().expect("completed profession receipt");
        let snapshot = match result {
            Ok(snapshot) => snapshot,
            Err(error) => {
                error!(character = pending.character, %error, "Profession snapshot commit failed");
                continue;
            }
        };
        let owner = connections.iter_mut().find(|(link, session, _)| {
            link.0 == pending.player && session.character_id == pending.character
        });
        if let Some((_, _, mut sender)) = owner {
            sender.send::<ProfessionChannel>(snapshot);
        }
    }
}
