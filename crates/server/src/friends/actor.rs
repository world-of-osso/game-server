//! Friend edits and owner publication share FIFO durability ordering.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

pub(super) enum Mutation {
    Add { self_name: String, target: String },
    Remove { target: String },
}

#[derive(Clone)]
struct Publication {
    state: PlayerFriendsState,
    message: Option<String>,
    error: Option<String>,
}

struct PendingMutation {
    connection: Entity,
    player: Entity,
    character: u64,
    receipt: CommitReceipt<Publication>,
}

#[derive(Resource, Default)]
pub(super) struct PendingMutations(VecDeque<PendingMutation>);

impl PendingMutations {
    pub(super) fn enqueue(
        &mut self,
        db: &GameDatabase,
        connection: Entity,
        player: Entity,
        character: u64,
        mutation: Mutation,
    ) {
        let receipt = db.writer.enqueue("friend mutation", move |db| {
            execute(db, character, mutation)
        });
        self.0.push_back(PendingMutation {
            connection,
            player,
            character,
            receipt,
        });
    }
}

fn execute(db: &redb::Database, character: u64, mutation: Mutation) -> Result<Publication, String> {
    let mut state = persistence::load_friends(db, character);
    sanitize_duplicates(&mut state);
    let result = match mutation {
        Mutation::Add { self_name, target } => add_friend(db, &mut state, &self_name, &target),
        Mutation::Remove { target } => remove_friend(&mut state, &target),
    };
    let (message, error) = match result {
        Ok(message) => {
            persistence::save_friends(db, character, &state);
            (Some(message), None)
        }
        Err(error) => (None, Some(error)),
    };
    Ok(Publication {
        state,
        message,
        error,
    })
}

pub(super) fn publish_mutations(
    mut pending: ResMut<PendingMutations>,
    db: Option<Res<GameDatabase>>,
    sessions: Query<(&PlayerLink, &InWorld), With<Authenticated>>,
    mut friends: Query<(&mut PlayerFriendsState, Option<&mut FriendsSnapshotTracker>)>,
    mut index: ResMut<FriendSnapshotIndex>,
    mut senders: Query<&mut MessageSender<FriendsStateUpdate>>,
) {
    let Some(db) = db else { return };
    while let Some(result) = pending
        .0
        .front()
        .and_then(|pending| pending.receipt.result())
    {
        let mutation = pending.0.pop_front().expect("completed friend receipt");
        let same_player = sessions
            .get(mutation.connection)
            .is_ok_and(|(link, session)| {
                link.0 == mutation.player && session.character_id == mutation.character
            });
        if !same_player {
            continue;
        }
        publish_mutation(
            &db.db,
            mutation,
            result,
            &mut friends,
            &mut index,
            &mut senders,
        );
    }
}

fn publish_mutation(
    db: &redb::Database,
    mutation: PendingMutation,
    result: Result<Publication, String>,
    friends: &mut Query<(&mut PlayerFriendsState, Option<&mut FriendsSnapshotTracker>)>,
    index: &mut FriendSnapshotIndex,
    senders: &mut Query<&mut MessageSender<FriendsStateUpdate>>,
) {
    let publication = match result {
        Ok(publication) => publication,
        Err(error) => {
            send_friends_error(mutation.connection, senders, &error);
            return;
        }
    };
    index.register_owner(mutation.player, mutation.connection, &publication.state);
    let snapshot = build_friends_snapshot(db, &publication.state, &index.online);
    if let Ok((mut live, tracker)) = friends.get_mut(mutation.player) {
        *live = publication.state;
        if let Some(mut tracker) = tracker {
            tracker.last_sent = Some(snapshot.clone());
        }
    }
    send_friends_update(
        mutation.connection,
        snapshot,
        publication.message,
        publication.error,
        senders,
    );
}
