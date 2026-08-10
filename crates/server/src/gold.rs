use std::collections::HashMap;

use bevy::prelude::*;
use lightyear::prelude::{ReplicationDirtyQueue, ReplicationSystems};
use shared::components::{Gold, Player};

use crate::persistence::ClientId;

#[derive(Resource, Default)]
pub(crate) struct GoldUpdateQueue {
    pending: HashMap<u64, u32>,
}

impl GoldUpdateQueue {
    pub(crate) fn record(&mut self, character_id: u64, gold: u32) {
        self.pending.insert(character_id, gold);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    fn take(&mut self) -> HashMap<u64, u32> {
        std::mem::take(&mut self.pending)
    }
}

pub(crate) fn record_persisted_gold(
    db: &redb::Database,
    character_id: u64,
    updates: &mut GoldUpdateQueue,
) {
    let gold = crate::persistence::load_inventory(db, character_id).gold;
    updates.record(character_id, gold);
}

pub(crate) struct GoldPlugin;

impl Plugin for GoldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GoldUpdateQueue>().add_systems(
            PostUpdate,
            apply_gold_updates.before(ReplicationSystems::Send),
        );
    }
}

pub(crate) fn apply_gold_updates(
    mut updates: ResMut<GoldUpdateQueue>,
    mut players: Query<(Entity, &ClientId, &mut Gold), With<Player>>,
    dirty_queue: Option<Res<ReplicationDirtyQueue>>,
) {
    if updates.is_empty() {
        return;
    }

    let mut pending = updates.take();
    for (player_entity, character_id, mut gold) in &mut players {
        let Some(updated_gold) = pending.remove(&character_id.0) else {
            continue;
        };
        if gold.0 == updated_gold {
            continue;
        }

        gold.0 = updated_gold;
        if let Some(dirty_queue) = dirty_queue.as_deref() {
            dirty_queue.mark(player_entity);
        }
    }
}
