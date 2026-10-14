//! Fresh durable kill rewards commit on the serial writer before owner publication.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

pub(super) struct ReputationReward {
    pub initial: CharacterReputation,
    pub faction: u32,
    pub amount: i32,
    pub registry: FactionRegistry,
}

#[derive(Clone)]
struct CommittedReward {
    reputation: CharacterReputation,
    message: String,
}

struct PendingReward {
    player: Entity,
    character: u64,
    receipt: CommitReceipt<Option<CommittedReward>>,
}

#[derive(Resource, Default)]
pub(super) struct PendingReputationRewards(VecDeque<PendingReward>);

impl PendingReputationRewards {
    pub(super) fn enqueue(
        &mut self,
        db: &GameDatabase,
        player: Entity,
        character: u64,
        reward: ReputationReward,
    ) {
        let receipt = db.writer.enqueue("NPC kill reputation", move |db| {
            let mut reputation = persistence::load_reputation(db, character, || reward.initial);
            let changes = reputation.gain_rep_with_spillover(
                reward.faction,
                reward.amount,
                RepGainSource::MobKill,
                &reward.registry,
            );
            if changes.is_empty() {
                return Ok(None);
            }
            let message = format_change_message(&changes, &reward.registry);
            persistence::save_reputation(db, character, &reputation);
            Ok(Some(CommittedReward {
                reputation,
                message,
            }))
        });
        self.0.push_back(PendingReward {
            player,
            character,
            receipt,
        });
    }
}

type ReputationOwners<'w, 's> =
    Query<'w, 's, (Entity, &'static PlayerLink, &'static InWorld), With<Authenticated>>;

pub(super) fn publish_reputation_rewards(
    mut pending: ResMut<PendingReputationRewards>,
    owners: ReputationOwners,
    mut players: Query<&mut CharacterReputation>,
    registry: Res<ReputationRegistry>,
    mut senders: Query<&mut MessageSender<ReputationStateUpdate>>,
) {
    while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
        let job = pending.0.pop_front().expect("completed reputation receipt");
        let owner = owners.iter().find(|(_, link, session)| {
            link.0 == job.player && session.character_id == job.character
        });
        let Some((connection, _, _)) = owner else {
            continue;
        };
        publish_reward_result(
            result,
            job.player,
            connection,
            &mut players,
            &registry.0,
            &mut senders,
        );
    }
}

fn publish_reward_result(
    result: Result<Option<CommittedReward>, String>,
    player: Entity,
    connection: Entity,
    players: &mut Query<&mut CharacterReputation>,
    registry: &FactionRegistry,
    senders: &mut Query<&mut MessageSender<ReputationStateUpdate>>,
) {
    match result {
        Ok(Some(reward)) => {
            let Ok(mut reputation) = players.get_mut(player) else {
                return;
            };
            *reputation = reward.reputation;
            send_reputation_update(
                connection,
                &reputation,
                registry,
                Some(reward.message),
                None,
                senders,
            );
        }
        Ok(None) => {}
        Err(error) => send_reputation_error(connection, senders, error),
    }
}
