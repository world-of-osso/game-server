//! Ignore mutations validate fresh FIFO rows; only durable replies reach the owner.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

pub(super) enum Mutation {
    Add { self_name: String, target: String },
    Remove { target: String },
}

#[derive(Clone)]
struct Publication {
    state: PlayerIgnoreListState,
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
        let receipt = db.writer.enqueue("ignore mutation", move |db| {
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
    let mut state = persistence::load_ignore_list(db, character);
    sanitize_names(&mut state);
    let result = match mutation {
        Mutation::Add { self_name, target } => add_ignore(db, &mut state, &self_name, &target),
        Mutation::Remove { target } => remove_ignore(&mut state, &target),
    };
    let (message, error) = match result {
        Ok(message) => {
            persistence::save_ignore_list(db, character, &state);
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
    sessions: Query<(&PlayerLink, &InWorld), With<Authenticated>>,
    mut ignores: Query<&mut PlayerIgnoreListState>,
    mut senders: Query<&mut MessageSender<IgnoreListStateUpdate>>,
) {
    while let Some(result) = pending
        .0
        .front()
        .and_then(|pending| pending.receipt.result())
    {
        let mutation = pending.0.pop_front().expect("completed ignore receipt");
        let same_player = sessions
            .get(mutation.connection)
            .is_ok_and(|(link, session)| {
                link.0 == mutation.player && session.character_id == mutation.character
            });
        if !same_player {
            continue;
        }
        publish_mutation(mutation, result, &mut ignores, &mut senders);
    }
}

fn publish_mutation(
    mutation: PendingMutation,
    result: Result<Publication, String>,
    ignores: &mut Query<&mut PlayerIgnoreListState>,
    senders: &mut Query<&mut MessageSender<IgnoreListStateUpdate>>,
) {
    let publication = match result {
        Ok(publication) => publication,
        Err(error) => {
            send_error(mutation.connection, senders, &error);
            return;
        }
    };
    let snapshot = snapshot_from_state(&publication.state);
    if let Ok(mut live) = ignores.get_mut(mutation.player) {
        *live = publication.state;
    }
    send_update(
        mutation.connection,
        snapshot,
        publication.message,
        publication.error,
        senders,
    );
}
