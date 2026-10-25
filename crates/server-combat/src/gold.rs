use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use lightyear::prelude::{ReplicationDirtyQueue, ReplicationSystems};
use shared::components::{Gold, Player};

use crate::persistence::ClientId;

/// Retail gold cap, 9,999,999g 99s 99c (TrinityCore `MAX_MONEY_AMOUNT`): the most copper
/// a player or a Warband bank holds.
pub(crate) const MAX_MONEY: u64 = 99_999_999_999;

/// Render a copper amount without narrowing the server's money range.
pub(crate) fn format_money(copper: u64) -> String {
    let gold = copper / 10_000;
    let silver = (copper % 10_000) / 100;
    let copper = copper % 100;
    format!("{gold}g {silver}s {copper}c")
}

/// Persisted `InventoryData` changes: gold values to replicate, plus the characters
/// whose bag contents need an `InventoryDelta`.
#[derive(Resource, Default)]
pub(crate) struct GoldUpdateQueue {
    pending: HashMap<u64, u64>,
    changed_inventories: HashSet<u64>,
    acquired_items: Vec<(u64, u32, u32)>,
    news_characters: HashSet<u64>,
    looted_items: Vec<(u64, u32, u32, u8)>,
    looted_money: Vec<(u64, u64)>,
    criteria: Vec<(u64, crate::achievement::CriteriaEvent)>,
}

impl GoldUpdateQueue {
    pub(crate) fn record(&mut self, character_id: u64, gold: u64) {
        self.pending.insert(character_id, gold);
        self.changed_inventories.insert(character_id);
        // TC Player::SetMoney: highest actual wallet on non-loading money changes.
        self.record_criteria(
            character_id,
            crate::achievement::CriteriaEvent::MostMoneyOwned(gold),
        );
    }

    /// Items changed without a gold change.
    pub(crate) fn record_items(&mut self, character_id: u64) {
        self.changed_inventories.insert(character_id);
    }

    pub(crate) fn record_news(&mut self, character: u64) {
        self.news_characters.insert(character);
    }

    pub(crate) fn take_news_characters(&mut self) -> HashSet<u64> {
        std::mem::take(&mut self.news_characters)
    }

    /// A committed Retail acquisition, not an inventory move or a requested grant.
    pub(crate) fn record_acquisition(&mut self, character: u64, item: u32, count: u32) {
        self.acquired_items.push((character, item, count));
    }

    /// A committed loot-slot award, including group rolls, never a vendor/craft/move.
    pub(crate) fn record_looted_item(&mut self, character: u64, item: u32, count: u32, kind: u8) {
        self.looted_items.push((character, item, count, kind));
    }

    /// Actual committed coin awards, including each member's validated split share.
    pub(crate) fn record_looted_money(&mut self, character: u64, copper: u64) {
        if copper != 0 {
            self.looted_money.push((character, copper));
        }
    }

    /// Achievement credit of a committed inventory/money transaction by its character.
    pub(crate) fn record_criteria(
        &mut self,
        character: u64,
        event: crate::achievement::CriteriaEvent,
    ) {
        self.criteria.push((character, event));
    }

    pub(crate) fn take_criteria(&mut self) -> Vec<(u64, crate::achievement::CriteriaEvent)> {
        std::mem::take(&mut self.criteria)
    }

    pub(crate) fn take_looted_money(&mut self) -> Vec<(u64, u64)> {
        std::mem::take(&mut self.looted_money)
    }

    pub(crate) fn take_looted_items(&mut self) -> Vec<(u64, u32, u32, u8)> {
        std::mem::take(&mut self.looted_items)
    }

    pub(crate) fn take_acquisitions(&mut self) -> Vec<(u64, u32, u32)> {
        std::mem::take(&mut self.acquired_items)
    }

    /// Characters whose bags changed this frame, before `send_inventory_deltas` takes them.
    pub(crate) fn changed_inventories(&self) -> &HashSet<u64> {
        &self.changed_inventories
    }

    pub(crate) fn take_changed_inventories(&mut self) -> HashSet<u64> {
        std::mem::take(&mut self.changed_inventories)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    fn take(&mut self) -> HashMap<u64, u64> {
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
