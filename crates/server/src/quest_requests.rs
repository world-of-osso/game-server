//! Quest giver and quest log requests from the owning client.
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::Message;
use lightyear::prelude::*;
use redb::Database;
use shared::components::{Level, Npc, Player, Position};
use shared::protocol::{
    AbandonQuest, QuestChannel, QuestFailed, QuestFailedReason, QuestGiverAcceptQuest,
    QuestGiverChooseReward, QuestGiverCompleteQuest, QuestGiverHello, QuestGiverOfferReward,
    QuestGiverQueryQuest, QuestGiverQuestComplete, QuestGiverQuestDetails, QuestGiverQuestList,
    QuestGiverRequestItems, QuestGiverStatusMultiple, QuestGiverStatusQuery, QuestObjectiveKind,
    QuestRewardItem, SetQuestWatched,
};

use super::{QueriedQuestGivers, QuestDirty, QuestStore, distance, giver_statuses, quest_player};
use crate::auth::{Authenticated, InWorld};
use crate::combat::DeathState;
use crate::gold::GoldUpdateQueue;
use crate::inventory::free_bag_cells;
use crate::item_templates::ItemTemplates;
use crate::networking::PlayerLink;
use crate::persistence::quest_store::{
    ActiveQuestData, QuestLogData, load_quest_log, save_quest_log,
};
use crate::persistence::{self, InventoryData};
use crate::quest_data::{QuestDb, QuestTemplate};
use crate::quest_rules::{
    QuestPlayer, can_take, ends, giver_quest_list, is_complete_in_log, starts,
};
use crate::quest_view::{
    bag_count, grant_item, quest_details, quest_rewards, remove_items, required_items,
};

/// Quest givers answer within this range (yards). Approximates TrinityCore
/// `INTERACTION_DISTANCE` (5.5) plus both units' combat reach.
const QUEST_GIVER_RANGE: f32 = 10.0;

type Receivers<'w, 's, M> =
    Query<'w, 's, (Entity, &'static mut MessageReceiver<M>), (With<Authenticated>, With<InWorld>)>;
type Senders<'w, 's, M> = Query<'w, 's, &'static mut MessageSender<M>>;

#[derive(SystemParam)]
pub(super) struct QuestReceivers<'w, 's> {
    status: Receivers<'w, 's, QuestGiverStatusQuery>,
    hello: Receivers<'w, 's, QuestGiverHello>,
    query: Receivers<'w, 's, QuestGiverQueryQuest>,
    accept: Receivers<'w, 's, QuestGiverAcceptQuest>,
    complete: Receivers<'w, 's, QuestGiverCompleteQuest>,
    choose: Receivers<'w, 's, QuestGiverChooseReward>,
    abandon: Receivers<'w, 's, AbandonQuest>,
    watch: Receivers<'w, 's, SetQuestWatched>,
}

enum QuestRequest {
    Status(QuestGiverStatusQuery),
    Hello(QuestGiverHello),
    Query(QuestGiverQueryQuest),
    Accept(QuestGiverAcceptQuest),
    Complete(QuestGiverCompleteQuest),
    Choose(QuestGiverChooseReward),
    Abandon(AbandonQuest),
    Watch(SetQuestWatched),
}

fn drain<M: Message>(
    receivers: &mut Receivers<M>,
    wrap: fn(M) -> QuestRequest,
    out: &mut Vec<(Entity, QuestRequest)>,
) {
    for (connection, mut receiver) in receivers.iter_mut() {
        out.extend(
            receiver
                .receive()
                .map(|message| (connection, wrap(message))),
        );
    }
}

impl QuestReceivers<'_, '_> {
    fn drain(&mut self) -> Vec<(Entity, QuestRequest)> {
        let mut out = Vec::new();
        drain(&mut self.status, QuestRequest::Status, &mut out);
        drain(&mut self.hello, QuestRequest::Hello, &mut out);
        drain(&mut self.query, QuestRequest::Query, &mut out);
        drain(&mut self.accept, QuestRequest::Accept, &mut out);
        drain(&mut self.complete, QuestRequest::Complete, &mut out);
        drain(&mut self.choose, QuestRequest::Choose, &mut out);
        drain(&mut self.abandon, QuestRequest::Abandon, &mut out);
        drain(&mut self.watch, QuestRequest::Watch, &mut out);
        out
    }
}

#[derive(SystemParam)]
pub(super) struct QuestSenders<'w, 's> {
    statuses: Senders<'w, 's, QuestGiverStatusMultiple>,
    lists: Senders<'w, 's, QuestGiverQuestList>,
    details: Senders<'w, 's, QuestGiverQuestDetails>,
    request_items: Senders<'w, 's, QuestGiverRequestItems>,
    offer_reward: Senders<'w, 's, QuestGiverOfferReward>,
    completed: Senders<'w, 's, QuestGiverQuestComplete>,
    failed: Senders<'w, 's, QuestFailed>,
}

fn send<M: Message>(senders: &mut Senders<M>, connection: Entity, message: M) {
    if let Ok(mut sender) = senders.get_mut(connection) {
        sender.send::<QuestChannel>(message);
    }
}

/// Live state requests read: who is asking and the NPCs around them.
#[derive(SystemParam)]
pub(super) struct QuestUnits<'w, 's> {
    connections: Query<'w, 's, (&'static InWorld, &'static PlayerLink)>,
    queried: Query<'w, 's, &'static mut QueriedQuestGivers>,
    players: Query<'w, 's, (&'static Player, &'static Level, &'static Position)>,
    npcs: Query<'w, 's, (&'static Npc, &'static Position, Has<DeathState>)>,
}

/// Bag and gold side effects: inventory deltas and gold replication.
#[derive(SystemParam)]
pub(super) struct QuestSideEffects<'w> {
    templates: Option<Res<'w, ItemTemplates>>,
    inventories: ResMut<'w, GoldUpdateQueue>,
    dirty: ResMut<'w, QuestDirty>,
}

/// Everything a request touches besides storage.
#[derive(SystemParam)]
pub(super) struct QuestIo<'w, 's> {
    units: QuestUnits<'w, 's>,
    senders: QuestSenders<'w, 's>,
    effects: QuestSideEffects<'w>,
}

struct Requester {
    connection: Entity,
    character_id: u64,
    player: QuestPlayer,
    position: Position,
}

fn failed(quest_id: u32, reason: QuestFailedReason) -> QuestFailed {
    QuestFailed { quest_id, reason }
}

type QuestResult = Result<(), QuestFailed>;

/// Everything one request reads and writes.
struct QuestSession<'a, 'w, 's> {
    db: &'a Database,
    quests: &'a QuestDb,
    requester: Requester,
    io: &'a mut QuestIo<'w, 's>,
}

pub(super) fn process_quest_requests(
    mut receivers: QuestReceivers,
    store: QuestStore,
    mut io: QuestIo,
) {
    let requests = receivers.drain();
    let Some((db, quests)) = store.get() else {
        return;
    };
    for (connection, request) in requests {
        let Some(requester) = resolve_requester(&io.units, connection) else {
            continue;
        };
        let mut session = QuestSession {
            db,
            quests,
            requester,
            io: &mut io,
        };
        if let Err(error) = session.handle(request) {
            send(&mut session.io.senders.failed, connection, error);
        }
    }
}

fn resolve_requester(units: &QuestUnits, connection: Entity) -> Option<Requester> {
    let (in_world, link) = units.connections.get(connection).ok()?;
    let (player, level, position) = units.players.get(link.0).ok()?;
    Some(Requester {
        connection,
        character_id: in_world.character_id,
        player: quest_player(player, level),
        position: *position,
    })
}

impl<'a> QuestSession<'a, '_, '_> {
    fn handle(&mut self, request: QuestRequest) -> QuestResult {
        match request {
            QuestRequest::Status(query) => {
                self.send_statuses(&query.npcs);
                Ok(())
            }
            QuestRequest::Hello(hello) => self.hello(hello.npc),
            QuestRequest::Query(query) => self.query(query.npc, query.quest_id),
            QuestRequest::Accept(accept) => self.accept(accept.npc, accept.quest_id),
            QuestRequest::Complete(complete) => self.complete(complete.npc, complete.quest_id),
            QuestRequest::Choose(choose) => {
                self.choose_reward(choose.npc, choose.quest_id, choose.choice_index)
            }
            QuestRequest::Abandon(abandon) => self.abandon(abandon.quest_id),
            QuestRequest::Watch(watch) => self.watch(watch.quest_id, watch.watched),
        }
    }

    fn character(&self) -> u64 {
        self.requester.character_id
    }

    fn load(&self) -> (QuestLogData, InventoryData) {
        (
            load_quest_log(self.db, self.character()),
            persistence::load_inventory(self.db, self.character()),
        )
    }

    fn save_log(&mut self, log: &QuestLogData) {
        let character = self.character();
        save_quest_log(self.db, character, log);
        self.io.effects.dirty.0.insert(character);
    }

    fn save_inventory(&mut self, inventory: &InventoryData, gold_changed: bool) {
        let character = self.character();
        persistence::save_inventory(self.db, character, inventory);
        if gold_changed {
            self.io
                .effects
                .inventories
                .record(character, inventory.gold);
        } else {
            self.io.effects.inventories.record_items(character);
        }
    }

    fn quest(&self, quest_id: u32) -> Result<&'a QuestTemplate, QuestFailed> {
        self.quests
            .quests
            .get(&quest_id)
            .ok_or(failed(quest_id, QuestFailedReason::NotAvailable))
    }

    /// Live NPC's creature entry, if the requester can talk to it.
    fn giver(&self, npc: u64, quest_id: u32) -> Result<u32, QuestFailed> {
        let entity =
            Entity::try_from_bits(npc).ok_or(failed(quest_id, QuestFailedReason::NotAvailable))?;
        let (npc, position, dead) = self
            .io
            .units
            .npcs
            .get(entity)
            .map_err(|_| failed(quest_id, QuestFailedReason::NotAvailable))?;
        if dead {
            return Err(failed(quest_id, QuestFailedReason::NotAvailable));
        }
        if distance(position, &self.requester.position) > QUEST_GIVER_RANGE {
            return Err(failed(quest_id, QuestFailedReason::TooFar));
        }
        Ok(npc.template_id)
    }

    fn send_statuses(&mut self, npcs: &[u64]) {
        let givers: Vec<(Entity, u32)> = npcs
            .iter()
            .filter_map(|&bits| Entity::try_from_bits(bits))
            .filter_map(|entity| Some((entity, self.io.units.npcs.get(entity).ok()?.0.template_id)))
            .collect();
        if let Ok(mut queried) = self.io.units.queried.get_mut(self.requester.connection) {
            queried.0.extend(givers.iter().map(|&(entity, _)| entity));
        }
        let (log, inventory) = self.load();
        let statuses = giver_statuses(self.quests, &log, &inventory, self.requester.player, givers);
        send(
            &mut self.io.senders.statuses,
            self.requester.connection,
            QuestGiverStatusMultiple { statuses },
        );
    }

    fn hello(&mut self, npc: u64) -> QuestResult {
        let entry = self.giver(npc, 0)?;
        let (log, inventory) = self.load();
        let quests = giver_quest_list(self.quests, &log, self.requester.player, entry, |id| {
            bag_count(&inventory, id)
        });
        send(
            &mut self.io.senders.lists,
            self.requester.connection,
            QuestGiverQuestList { npc, quests },
        );
        Ok(())
    }

    /// Details of a quest the giver starts and the player can take.
    fn offerable(&self, npc: u64, quest_id: u32, log: &QuestLogData) -> QuestResult {
        let entry = self.giver(npc, quest_id)?;
        let quest = self.quest(quest_id)?;
        if !starts(self.quests, entry, quest_id) {
            return Err(failed(quest_id, QuestFailedReason::NotAvailable));
        }
        can_take(self.quests, log, self.requester.player, quest)
            .map_err(|reason| failed(quest_id, reason))
    }

    fn query(&mut self, npc: u64, quest_id: u32) -> QuestResult {
        self.offerable(npc, quest_id, &load_quest_log(self.db, self.character()))?;
        let details = quest_details(self.quests, self.quest(quest_id)?, npc);
        send(
            &mut self.io.senders.details,
            self.requester.connection,
            details,
        );
        Ok(())
    }

    fn free_cells(&self) -> usize {
        self.io.effects.templates.as_deref().map_or(0, |templates| {
            free_bag_cells(self.db, templates, self.character())
        })
    }

    fn accept(&mut self, npc: u64, quest_id: u32) -> QuestResult {
        let (mut log, mut inventory) = self.load();
        self.offerable(npc, quest_id, &log)?;
        let quest = self.quest(quest_id)?;
        if quest.start_item != 0 {
            if self.free_cells() == 0 {
                return Err(failed(quest_id, QuestFailedReason::InventoryFull));
            }
            let count = quest.provided_item_count.max(1);
            grant_item(
                &mut inventory,
                self.db,
                self.quests,
                quest.start_item,
                count,
            );
            self.save_inventory(&inventory, false);
        }
        log.active.push(ActiveQuestData {
            quest_id,
            counts: vec![0; quest.objectives.len()],
        });
        // Retail auto quest watch (`autoQuestWatch` default on).
        log.watched.push(quest_id);
        self.save_log(&log);
        Ok(())
    }

    /// The quest is in the log and this giver ends it.
    fn turn_in_target(&self, npc: u64, quest_id: u32, log: &QuestLogData) -> QuestResult {
        let entry = self.giver(npc, quest_id)?;
        self.quest(quest_id)?;
        if !ends(self.quests, entry, quest_id) {
            return Err(failed(quest_id, QuestFailedReason::NotAvailable));
        }
        if log.active(quest_id).is_none() {
            return Err(failed(quest_id, QuestFailedReason::NotInLog));
        }
        Ok(())
    }

    fn complete(&mut self, npc: u64, quest_id: u32) -> QuestResult {
        let (log, inventory) = self.load();
        self.turn_in_target(npc, quest_id, &log)?;
        let quest = self.quest(quest_id)?;
        let done = is_complete_in_log(self.quests, &log, quest_id, |id| bag_count(&inventory, id));
        let connection = self.requester.connection;
        if done {
            let offer = QuestGiverOfferReward {
                npc,
                quest_id,
                title: quest.title.clone(),
                reward_text: quest.offer_reward_text.clone(),
                rewards: quest_rewards(self.quests, quest),
            };
            send(&mut self.io.senders.offer_reward, connection, offer);
        } else {
            let request = QuestGiverRequestItems {
                npc,
                quest_id,
                title: quest.title.clone(),
                completion_text: quest.request_items_text.clone(),
                required_items: required_items(self.quests, quest),
                can_complete: false,
            };
            send(&mut self.io.senders.request_items, connection, request);
        }
        Ok(())
    }

    /// Items granted on turn-in: fixed rewards plus the chosen one.
    fn granted_items(
        &self,
        quest: &QuestTemplate,
        choice_index: Option<u8>,
    ) -> Result<Vec<(u32, u32)>, QuestFailed> {
        let mut items = quest.reward_items.clone();
        if !quest.choice_items.is_empty() {
            let choice = choice_index
                .and_then(|index| quest.choice_items.get(usize::from(index)))
                .ok_or(failed(quest.id, QuestFailedReason::InvalidRewardChoice))?;
            items.push(*choice);
        }
        Ok(items)
    }

    fn choose_reward(&mut self, npc: u64, quest_id: u32, choice_index: Option<u8>) -> QuestResult {
        let (mut log, mut inventory) = self.load();
        self.turn_in_target(npc, quest_id, &log)?;
        let quest = self.quest(quest_id)?;
        if !is_complete_in_log(self.quests, &log, quest_id, |id| bag_count(&inventory, id)) {
            return Err(failed(quest_id, QuestFailedReason::ObjectivesIncomplete));
        }
        let items = self.granted_items(quest, choice_index)?;
        let gold = inventory.gold.checked_add_signed(quest.reward_money);
        let Some(gold) = gold else {
            return Err(failed(quest_id, QuestFailedReason::NotEnoughMoney));
        };
        if self.free_cells() < items.len() {
            return Err(failed(quest_id, QuestFailedReason::InventoryFull));
        }
        take_quest_items(&mut inventory, quest);
        for &(item_id, count) in &items {
            grant_item(&mut inventory, self.db, self.quests, item_id, count);
        }
        inventory.gold = gold;
        self.save_inventory(&inventory, quest.reward_money != 0);
        log.remove(quest_id);
        log.mark_rewarded(quest_id);
        self.save_log(&log);
        let complete = QuestGiverQuestComplete {
            quest_id,
            money: quest.reward_money.max(0) as u32,
            items: items
                .iter()
                .map(|&(item_id, count)| QuestRewardItem {
                    item_id,
                    name: self.quests.item_name(item_id),
                    count,
                })
                .collect(),
        };
        send(
            &mut self.io.senders.completed,
            self.requester.connection,
            complete,
        );
        self.offer_next_quest(npc, quest.reward_next_quest, &log);
        Ok(())
    }

    /// Retail follows a turn-in with the chain's next quest when the same giver
    /// starts it and the player can take it.
    fn offer_next_quest(&mut self, npc: u64, next_quest: u32, log: &QuestLogData) {
        if next_quest == 0 || self.offerable(npc, next_quest, log).is_err() {
            return;
        }
        let Ok(quest) = self.quest(next_quest) else {
            return;
        };
        let details = quest_details(self.quests, quest, npc);
        send(
            &mut self.io.senders.details,
            self.requester.connection,
            details,
        );
    }

    fn abandon(&mut self, quest_id: u32) -> QuestResult {
        let (mut log, mut inventory) = self.load();
        if !log.remove(quest_id) {
            return Err(failed(quest_id, QuestFailedReason::NotInLog));
        }
        self.save_log(&log);
        // `TakeQuestSourceItem`: the item the quest provided goes with it.
        let provided = self.quests.quests.get(&quest_id).map_or((0, 0), |quest| {
            (quest.start_item, quest.provided_item_count.max(1))
        });
        if provided.0 != 0 && bag_count(&inventory, provided.0) > 0 {
            remove_items(&mut inventory, provided.0, provided.1);
            self.save_inventory(&inventory, false);
        }
        Ok(())
    }

    fn watch(&mut self, quest_id: u32, watched: bool) -> QuestResult {
        let mut log = load_quest_log(self.db, self.character());
        if log.active(quest_id).is_none() {
            return Err(failed(quest_id, QuestFailedReason::NotInLog));
        }
        let listed = log.watched.contains(&quest_id);
        if watched && !listed {
            log.watched.push(quest_id);
        } else if !watched && listed {
            log.watched.retain(|&id| id != quest_id);
        } else {
            return Ok(());
        }
        self.save_log(&log);
        Ok(())
    }
}

/// `RewardQuest` destroys the required items and the provided start item.
fn take_quest_items(inventory: &mut InventoryData, quest: &QuestTemplate) {
    for objective in &quest.objectives {
        if objective.kind == QuestObjectiveKind::Item {
            remove_items(inventory, objective.object_id, objective.required);
        }
    }
    if quest.start_item != 0 {
        remove_items(
            inventory,
            quest.start_item,
            quest.provided_item_count.max(1),
        );
    }
}
