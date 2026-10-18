//! Quest giver and quest log requests from the owning client.
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::Message;
use lightyear::prelude::*;
use redb::Database;
use shared::components::{Level, Npc, Player, Position};
use shared::protocol::{
    AbandonQuest, PushQuestToParty, QuestChannel, QuestFailed, QuestFailedReason,
    QuestGiverAcceptQuest, QuestGiverChooseReward, QuestGiverCompleteQuest, QuestGiverHello,
    QuestGiverOfferReward, QuestGiverQueryQuest, QuestGiverQuestComplete, QuestGiverQuestDetails,
    QuestGiverQuestList, QuestGiverRequestItems, QuestGiverStatusMultiple, QuestGiverStatusQuery,
    QuestPushReason, QuestPushResponse, QuestPushResult, QuestRewardItem, QuestShareOffer,
    SetQuestWatched,
};

use super::{QueriedQuestGivers, QuestDirty, QuestStore, distance, giver_statuses, quest_player};
use crate::auth::{Authenticated, InWorld};
use crate::combat::DeathState;
use crate::experience::{MAX_LEVEL, XpAward, XpAwards, quest_xp};
use crate::gold::GoldUpdateQueue;
use crate::inventory::free_bag_cells;
use crate::item_templates::ItemTemplates;
use crate::networking::PlayerLink;
use crate::persistence::quest_store::{
    ActiveQuestData, QuestLogData, load_quest_log, save_quest_log,
};
use crate::persistence::{self, InventoryData};
use crate::quest_data::{ObjectiveKind, QuestDb, QuestTemplate};
use crate::quest_rules::{
    QuestPlayer, can_take, ends, giver_quest_list, is_complete_in_log, quest_money, starts,
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
    push: Receivers<'w, 's, PushQuestToParty>,
    push_response: Receivers<'w, 's, QuestPushResponse>,
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
    Push(PushQuestToParty),
    PushResponse(QuestPushResponse),
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
        drain(&mut self.push, QuestRequest::Push, &mut out);
        drain(
            &mut self.push_response,
            QuestRequest::PushResponse,
            &mut out,
        );
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
    inventory_errors: Senders<'w, 's, shared::protocol::InventoryError>,
    share_offers: Senders<'w, 's, QuestShareOffer>,
    push_results: Senders<'w, 's, QuestPushResult>,
}

fn send<M: Message>(senders: &mut Senders<M>, connection: Entity, message: M) {
    if let Ok(mut sender) = senders.get_mut(connection) {
        sender.send::<QuestChannel>(message);
    }
}

/// Live state requests read: who is asking and the NPCs around them.
#[derive(SystemParam)]
pub(super) struct QuestUnits<'w, 's> {
    connections: Query<'w, 's, (Entity, &'static InWorld, &'static PlayerLink)>,
    queried: Query<'w, 's, &'static mut QueriedQuestGivers>,
    players: Query<'w, 's, (&'static Player, &'static Level, &'static Position)>,
    npcs: Query<'w, 's, (&'static Npc, &'static Position, Has<DeathState>)>,
    lives: Query<'w, 's, &'static crate::death::PlayerDeathState>,
    profession_states: Query<
        'w,
        's,
        (
            &'static crate::profession::PlayerProfessions,
            &'static crate::currency::PlayerCurrencies,
        ),
    >,
}

/// Bag and gold side effects: inventory deltas and gold replication.
#[derive(SystemParam)]
pub(super) struct QuestSideEffects<'w> {
    templates: Option<Res<'w, ItemTemplates>>,
    inventories: ResMut<'w, GoldUpdateQueue>,
    dirty: ResMut<'w, QuestDirty>,
    xp: ResMut<'w, XpAwards>,
    specializations: Option<Res<'w, crate::profession::specializations::SpecializationData>>,
}

/// Everything a request touches besides storage.
#[derive(SystemParam)]
pub(super) struct QuestIo<'w, 's> {
    commands: Commands<'w, 's>,
    units: QuestUnits<'w, 's>,
    senders: QuestSenders<'w, 's>,
    effects: QuestSideEffects<'w>,
    groups: Option<Res<'w, crate::group::GroupRegistry>>,
    shares: ResMut<'w, PendingShares>,
}

#[derive(Resource, Default)]
pub(super) struct PendingShares {
    next_id: u64,
    offers: std::collections::HashMap<u64, SharedOffer>,
}
struct SharedOffer {
    id: u64,
    sender: u64,
    quest: u32,
}

struct Requester {
    connection: Entity,
    character_id: u64,
    player: QuestPlayer,
    position: Position,
    name: String,
}

fn push_reason(reason: QuestFailedReason) -> QuestPushReason {
    match reason {
        QuestFailedReason::AlreadyOn => QuestPushReason::OnQuest,
        QuestFailedReason::AlreadyDone => QuestPushReason::AlreadyDone,
        QuestFailedReason::QuestLogFull => QuestPushReason::LogFull,
        QuestFailedReason::LowLevel => QuestPushReason::LowLevel,
        QuestFailedReason::HighLevel => QuestPushReason::HighLevel,
        QuestFailedReason::WrongClass => QuestPushReason::Class,
        QuestFailedReason::WrongRace => QuestPushReason::Race,
        QuestFailedReason::DontHaveRequirement => QuestPushReason::Prerequisite,
        QuestFailedReason::TimedOut => QuestPushReason::TimerExpired,
        _ => QuestPushReason::Invalid,
    }
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
    item_error_reported: bool,
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
            item_error_reported: false,
        };
        if let Err(error) = session.handle(request)
            && !session.item_error_reported
        {
            send(&mut session.io.senders.failed, connection, error);
        }
    }
}

pub(super) fn process_world_quests(
    store: QuestStore,
    mut io: QuestIo,
    maps: Query<&crate::maps::InMap>,
) {
    let Some((db, quests)) = store.get() else {
        return;
    };
    let connections: Vec<_> = io
        .units
        .connections
        .iter()
        .map(|(connection, _, link)| (connection, link.0))
        .collect();
    for (connection, player_entity) in connections {
        let Some(requester) = resolve_requester(&io.units, connection) else {
            continue;
        };
        let Ok(map) = maps.get(player_entity) else {
            continue;
        };
        let mut session = QuestSession {
            db,
            quests,
            requester,
            io: &mut io,
            item_error_reported: false,
        };
        session.visit_world_quests(map.map_id);
    }
}

fn resolve_requester(units: &QuestUnits, connection: Entity) -> Option<Requester> {
    let (_, in_world, link) = units.connections.get(connection).ok()?;
    let (player, level, position) = units.players.get(link.0).ok()?;
    Some(Requester {
        connection,
        character_id: in_world.character_id,
        player: quest_player(player, level),
        position: *position,
        name: player.name.clone(),
    })
}

impl<'a> QuestSession<'a, '_, '_> {
    fn visit_world_quests(&mut self, map_id: u32) {
        let position = self.requester.position;
        let (world_x, world_y, _) =
            crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
        let windows: Vec<_> = self.quests.world_quest_windows.values().copied().collect();
        for window in windows {
            let quest_id = window.quest_id;
            let inside = self
                .quests
                .world_quest_areas
                .get(&quest_id)
                .is_some_and(|areas| {
                    areas
                        .iter()
                        .any(|area| area.contains(map_id, world_x, world_y))
                });
            let (mut log, mut inventory) = self.load();
            if log.active(quest_id).is_none() {
                if !inside {
                    continue;
                }
                if self
                    .accept_log(quest_id, &mut log, &mut inventory, Some(window.expires_at))
                    .is_err()
                {
                    continue;
                }
            }
            let watched = log.watched.contains(&quest_id);
            if watched != inside {
                if inside {
                    log.watched.push(quest_id);
                } else {
                    log.watched.retain(|&id| id != quest_id);
                }
                self.save_log(&log);
            }
            if is_complete_in_log(self.quests, &log, quest_id, |id| {
                bag_count(self.db, &inventory, id)
            }) {
                if let Err(error) = self.choose_reward(0, quest_id, None) {
                    send(
                        &mut self.io.senders.failed,
                        self.requester.connection,
                        error,
                    );
                }
            }
        }
    }

    fn handle(&mut self, request: QuestRequest) -> QuestResult {
        match request {
            QuestRequest::Status(query) => {
                self.send_statuses(&query.npcs);
                Ok(())
            }
            QuestRequest::Hello(hello) => self.hello(hello.npc),
            QuestRequest::Query(query) => self.query(query.npc, query.quest_id),
            QuestRequest::Accept(accept) => self.accept(accept.npc, accept.quest_id),
            QuestRequest::Complete(complete) => {
                self.require_giver_quest(complete.quest_id)?;
                self.complete(complete.npc, complete.quest_id)
            }
            QuestRequest::Choose(choose) => {
                self.require_giver_quest(choose.quest_id)?;
                self.choose_reward(choose.npc, choose.quest_id, choose.choice_index)
            }
            QuestRequest::Abandon(abandon) => self.abandon(abandon.quest_id),
            QuestRequest::Watch(watch) => self.watch(watch.quest_id, watch.watched),
            QuestRequest::Push(push) => {
                self.push(push.quest_id);
                Ok(())
            }
            QuestRequest::PushResponse(response) => {
                self.respond_share(response);
                Ok(())
            }
        }
    }

    fn require_giver_quest(&self, quest_id: u32) -> QuestResult {
        if self.quest(quest_id)?.quest_info_modifiers & 0x100 != 0 {
            return Err(failed(quest_id, QuestFailedReason::NotAvailable));
        }
        Ok(())
    }

    fn push_result(
        &mut self,
        connection: Entity,
        quest_id: u32,
        player_name: String,
        reason: QuestPushReason,
    ) {
        send(
            &mut self.io.senders.push_results,
            connection,
            QuestPushResult {
                quest_id,
                quest_title: self
                    .quests
                    .quests
                    .get(&quest_id)
                    .map_or_else(String::new, |q| q.title.clone()),
                recipient: false,
                player_name,
                reason,
            },
        );
    }

    fn recipient_result(
        &mut self,
        connection: Entity,
        quest_id: u32,
        sender_name: String,
        reason: QuestPushReason,
    ) {
        send(
            &mut self.io.senders.push_results,
            connection,
            QuestPushResult {
                quest_id,
                quest_title: self.quests.quests[&quest_id].title.clone(),
                recipient: true,
                player_name: sender_name,
                reason,
            },
        );
    }

    fn same_party(&self, sender: u64, recipient: u64) -> bool {
        self.io
            .groups
            .as_deref()
            .is_some_and(|g| g.group_members(sender).contains(&recipient))
    }

    fn share_eligibility(&self, target: &Requester, quest_id: u32) -> Result<(), QuestPushReason> {
        let (_, _, link) = self
            .io
            .units
            .connections
            .get(target.connection)
            .map_err(|_| QuestPushReason::Invalid)?;
        if self
            .io
            .units
            .lives
            .get(link.0)
            .is_ok_and(|life| life.state != shared::death::DeathState::Alive)
        {
            return Err(QuestPushReason::Dead);
        }
        let quest = self
            .quests
            .quests
            .get(&quest_id)
            .ok_or(QuestPushReason::Invalid)?;
        can_take(
            self.quests,
            &load_quest_log(self.db, target.character_id),
            target.player,
            quest,
        )
        .map_err(push_reason)
    }

    fn push(&mut self, quest_id: u32) {
        let allowed = self
            .quests
            .quests
            .get(&quest_id)
            .is_some_and(|q| q.flags & 0x8 != 0)
            && load_quest_log(self.db, self.character())
                .active(quest_id)
                .is_some_and(|q| !q.failed);
        if !allowed {
            self.push_result(
                self.requester.connection,
                quest_id,
                String::new(),
                QuestPushReason::NotAllowed,
            );
            return;
        }
        let members = self
            .io
            .groups
            .as_deref()
            .map_or_else(Vec::new, |g| g.group_members(self.character()));
        if members.is_empty() {
            self.push_result(
                self.requester.connection,
                quest_id,
                String::new(),
                QuestPushReason::NotInParty,
            );
            return;
        }
        let online: std::collections::HashSet<_> = self
            .io
            .units
            .connections
            .iter()
            .map(|(_, c, _)| c.character_id)
            .collect();
        self.io
            .shares
            .offers
            .retain(|to, offer| online.contains(to) && online.contains(&offer.sender));
        for character in members
            .into_iter()
            .filter(|&id| id != self.character())
            .collect::<Vec<_>>()
        {
            let Some(connection) = self.connection_for(character) else {
                continue;
            };
            let Some(target) = resolve_requester(&self.io.units, connection) else {
                continue;
            };
            let result = if crate::faction::faction_group_for_race(target.player.race)
                != crate::faction::faction_group_for_race(self.requester.player.race)
            {
                Err(QuestPushReason::CrossFactionRestricted)
            } else if self.io.shares.offers.contains_key(&character) {
                Err(QuestPushReason::Busy)
            } else {
                self.share_eligibility(&target, quest_id)
            };
            let reason = match result {
                Ok(()) => QuestPushReason::Success,
                Err(reason) => reason,
            };
            self.push_result(
                self.requester.connection,
                quest_id,
                target.name.clone(),
                reason,
            );
            if reason != QuestPushReason::Success {
                if reason != QuestPushReason::Busy
                    && reason != QuestPushReason::CrossFactionRestricted
                {
                    self.recipient_result(
                        connection,
                        quest_id,
                        self.requester.name.clone(),
                        reason,
                    );
                }
                continue;
            }
            self.io.shares.next_id += 1;
            let id = self.io.shares.next_id;
            let sender = self.character();
            self.io.shares.offers.insert(
                character,
                SharedOffer {
                    id,
                    sender,
                    quest: quest_id,
                },
            );
            send(
                &mut self.io.senders.share_offers,
                connection,
                QuestShareOffer {
                    offer_id: id,
                    sender_name: self.requester.name.clone(),
                    quest_id,
                    title: self.quests.quests[&quest_id].title.clone(),
                },
            );
        }
    }

    fn respond_share(&mut self, response: QuestPushResponse) {
        let character = self.character();
        if !self
            .io
            .shares
            .offers
            .get(&character)
            .is_some_and(|o| o.id == response.offer_id)
        {
            return;
        }
        let offer = self
            .io
            .shares
            .offers
            .remove(&character)
            .expect("matched offer");
        let Some(sender_connection) = self.connection_for(offer.sender) else {
            return;
        };
        let sender_log = load_quest_log(self.db, offer.sender);
        let active = sender_log.active(offer.quest);
        let still_allowed = self.same_party(offer.sender, character)
            && active.is_some_and(|q| !q.failed)
            && self
                .quests
                .quests
                .get(&offer.quest)
                .is_some_and(|q| q.flags & 0x8 != 0);
        let reason = if !still_allowed {
            QuestPushReason::NotAllowed
        } else if !response.accepted {
            QuestPushReason::Declined
        } else if let Err(reason) = self.share_eligibility(&self.requester, offer.quest) {
            reason
        } else {
            let deadline = active.and_then(|q| q.expires_at);
            if deadline.is_some_and(|end| {
                end <= self
                    .quests
                    .now
                    .unwrap_or_else(crate::persistence::quest_store::unix_now)
            }) {
                QuestPushReason::TimerExpired
            } else {
                let (mut log, mut inventory) = self.load();
                match self.accept_log(offer.quest, &mut log, &mut inventory, deadline) {
                    Ok(()) => QuestPushReason::Accepted,
                    Err(error) => push_reason(error.reason),
                }
            }
        };
        self.push_result(
            sender_connection,
            offer.quest,
            self.requester.name.clone(),
            reason,
        );
        if response.accepted && reason != QuestPushReason::Accepted {
            if let Some(sender) = resolve_requester(&self.io.units, sender_connection) {
                self.recipient_result(self.requester.connection, offer.quest, sender.name, reason);
            }
        }
    }

    fn connection_for(&self, character: u64) -> Option<Entity> {
        self.io
            .units
            .connections
            .iter()
            .find_map(|(connection, owner, _)| {
                (owner.character_id == character).then_some(connection)
            })
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

    fn create_quest_items(
        &self,
        inventory: &InventoryData,
        items: &[(u32, u32)],
    ) -> Result<InventoryData, shared::protocol::InventoryError> {
        use crate::inventory::inventory_model::error;
        use shared::protocol::InventoryErrorReason;
        if items.is_empty() {
            return Ok(inventory.clone());
        }
        let templates = self.io.effects.templates.as_deref().ok_or_else(|| {
            error(
                InventoryErrorReason::InternalBagError,
                "quest item templates unavailable",
            )
        })?;
        let mut record =
            persistence::inventory_store::load_inventory_record(self.db, self.character())
                .ok_or_else(|| {
                    error(
                        InventoryErrorReason::InternalBagError,
                        "quest inventory unavailable",
                    )
                })?;
        record.inventory = inventory.clone();
        let mut model =
            crate::inventory::load_model_from_record(self.db, templates, self.character(), record)?;
        for &(item, count) in items {
            grant_item(&mut model, self.db, self.quests, templates, item, count)?;
        }
        Ok(model.into_record().inventory)
    }

    fn grant_items(
        &mut self,
        quest_id: u32,
        inventory: &mut InventoryData,
        items: &[(u32, u32)],
    ) -> QuestResult {
        match self.create_quest_items(inventory, items) {
            Ok(created) => {
                *inventory = created;
                Ok(())
            }
            Err(error) => {
                if error.reason == shared::protocol::InventoryErrorReason::ItemMaxCount {
                    self.report_item_error(quest_id, error);
                    return Err(failed(quest_id, QuestFailedReason::InventoryFull));
                }
                let reason = match error.reason {
                    shared::protocol::InventoryErrorReason::InvFull
                    | shared::protocol::InventoryErrorReason::BagFull
                    | shared::protocol::InventoryErrorReason::CantStack => {
                        QuestFailedReason::InventoryFull
                    }
                    _ => {
                        warn!("Quest {quest_id} item grant rejected: {error:?}");
                        QuestFailedReason::NotAvailable
                    }
                };
                Err(failed(quest_id, reason))
            }
        }
    }

    fn report_item_error(&mut self, quest_id: u32, error: shared::protocol::InventoryError) {
        self.item_error_reported = true;
        match self
            .io
            .senders
            .inventory_errors
            .get_mut(self.requester.connection)
        {
            Ok(mut sender) => sender.send::<shared::protocol::InventoryChannel>(error),
            Err(connection) => {
                warn!("Quest {quest_id} inventory rejection has no sender: {connection:?}")
            }
        }
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
        let statuses = giver_statuses(
            self.db,
            self.quests,
            &log,
            &inventory,
            self.requester.player,
            givers,
        );
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
            bag_count(self.db, &inventory, id)
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
        let details = quest_details(
            self.quests,
            self.quest(quest_id)?,
            npc,
            self.requester.player,
        );
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
        self.accept_log(quest_id, &mut log, &mut inventory, None)?;
        self.queue_script_quest_hook(npc, quest_id, false);
        Ok(())
    }

    fn queue_script_quest_hook(&mut self, npc: u64, quest_id: u32, rewarded: bool) {
        let connection = self.requester.connection;
        self.io.commands.queue(move |world: &mut World| {
            let Some(player) = world.get::<PlayerLink>(connection).map(|link| link.0) else {
                return;
            };
            let Some(creature) = Entity::try_from_bits(npc) else {
                return;
            };
            let hook = if rewarded {
                crate::scripted_ai::Hook::QuestRewarded { player, quest_id }
            } else {
                crate::scripted_ai::Hook::QuestAccepted { player, quest_id }
            };
            crate::scripted_ai::dispatch(world, creature, hook);
        });
    }

    fn accept_log(
        &mut self,
        quest_id: u32,
        log: &mut QuestLogData,
        inventory: &mut InventoryData,
        shared_deadline: Option<i64>,
    ) -> QuestResult {
        let quest = self.quest(quest_id)?;
        can_take(self.quests, log, self.requester.player, quest)
            .map_err(|reason| failed(quest_id, reason))?;
        if quest.start_item != 0 {
            if self.free_cells() == 0 {
                return Err(failed(quest_id, QuestFailedReason::InventoryFull));
            }
            let count = quest.provided_item_count.max(1);
            self.grant_items(quest_id, inventory, &[(quest.start_item, count)])?;
            self.save_inventory(&inventory, false);
            let character = self.character();
            self.io
                .effects
                .inventories
                .record_acquisition(character, quest.start_item, count);
        }
        log.active.push(ActiveQuestData {
            quest_id,
            counts: vec![0; quest.objectives.len()],
            expires_at: shared_deadline.or_else(|| {
                (quest.time_allowed > 0).then(|| {
                    self.quests
                        .now
                        .unwrap_or_else(crate::persistence::quest_store::unix_now)
                        + i64::from(quest.time_allowed)
                })
            }),
            failed: false,
        });
        // Retail auto quest watch (`autoQuestWatch` default on).
        log.watched.push(quest_id);
        self.save_log(&log);
        Ok(())
    }

    /// The quest is in the log and this giver ends it.
    fn turn_in_target(&self, npc: u64, quest_id: u32, log: &QuestLogData) -> QuestResult {
        let quest = self.quest(quest_id)?;
        if quest.quest_info_modifiers & 0x100 != 0 {
            let now = self.quests.now.expect("world quest clock initialized");
            if !self.quests.world_quest_windows.contains_key(&quest_id)
                || log.active(quest_id).is_none_or(|active| {
                    active.failed || active.expires_at.is_none_or(|end| now >= end)
                })
            {
                return Err(failed(quest_id, QuestFailedReason::NotAvailable));
            }
            return Ok(());
        }
        let entry = self.giver(npc, quest_id)?;
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
        let done = is_complete_in_log(self.quests, &log, quest_id, |id| {
            bag_count(self.db, &inventory, id)
        });
        let connection = self.requester.connection;
        if done {
            let offer = QuestGiverOfferReward {
                npc,
                quest_id,
                title: quest.title.clone(),
                reward_text: quest.offer_reward_text.clone(),
                rewards: quest_rewards(self.quests, quest, self.requester.player),
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
        if !is_complete_in_log(self.quests, &log, quest_id, |id| {
            bag_count(self.db, &inventory, id)
        }) {
            return Err(failed(quest_id, QuestFailedReason::ObjectivesIncomplete));
        }
        let items = self.granted_items(quest, choice_index)?;
        let mut numeric = crate::world_quest_rewards::load_settlement(
            self.db,
            self.character(),
            self.requester.player.race,
            self.quests.world_quest_rewards.get(&quest_id),
        )
        .map_err(|error| {
            warn!("Quest {quest_id} reward rejected: {error}");
            failed(quest_id, QuestFailedReason::NotAvailable)
        })?;
        let money = quest_money(self.quests, quest, self.requester.player);
        let gold = inventory.gold.checked_add_signed(i64::from(money));
        let Some(gold) = gold else {
            return Err(failed(quest_id, QuestFailedReason::NotEnoughMoney));
        };
        if !items.is_empty() && self.free_cells() < items.len() {
            return Err(failed(quest_id, QuestFailedReason::InventoryFull));
        }
        let knowledge = self.load_reward_knowledge(quest, numeric.as_ref())?;
        if let (Some(settlement), Some(wallet)) = (&mut numeric, &knowledge) {
            settlement.wallet = wallet.clone();
        }
        take_quest_items(self.db, &mut inventory, quest);
        self.grant_items(quest_id, &mut inventory, &items)?;
        inventory.gold = gold;
        log.remove(quest_id);
        log.mark_rewarded(quest_id);
        log.completed_at.insert(
            quest_id,
            self.quests
                .now
                .unwrap_or_else(crate::persistence::quest_store::unix_now),
        );
        self.persist_reward(
            &inventory,
            &log,
            self.reward_appearances(quest, &items),
            numeric,
            knowledge.as_ref(),
        );
        let character = self.character();
        for &(item, count) in &items {
            self.io
                .effects
                .inventories
                .record_acquisition(character, item, count);
        }
        self.queue_script_quest_hook(npc, quest_id, true);
        let has_knowledge_reward = knowledge.is_some();
        let daily = quest.repeatability() == shared::protocol::QuestRepeatability::Daily;
        self.io.commands.queue(move |world: &mut World| {
            crate::achievement::record_quest_reward(world, character, quest_id, daily, money);
            if has_knowledge_reward {
                crate::profession::specializations::reload_knowledge_wallet(world, character);
            }
        });
        // `RewardQuest`: no XP at the cap.
        let level = self.requester.player.level;
        let xp = if quest.quest_info_modifiers & 0x100 != 0 {
            crate::world_quest_rewards::xp(self.quests, quest, level)
        } else if level >= MAX_LEVEL {
            0
        } else {
            match quest.captured_reward_xp {
                Some(xp) => xp,
                None => quest_xp(quest.level, level, quest.reward_xp_difficulty),
            }
        };
        if xp > 0 {
            let character_id = self.character();
            self.io.effects.xp.0.push(XpAward::quest(character_id, xp));
        }
        let complete = QuestGiverQuestComplete {
            quest_id,
            money: money.max(0) as u32,
            items: items
                .iter()
                .map(|&(item_id, count)| QuestRewardItem {
                    item_id,
                    name: self.quests.item_name(item_id),
                    count,
                })
                .collect(),
            xp,
        };
        send(
            &mut self.io.senders.completed,
            self.requester.connection,
            complete,
        );
        self.offer_next_quest(npc, quest.reward_next_quest, &log);
        Ok(())
    }

    fn load_reward_knowledge(
        &self,
        quest: &QuestTemplate,
        numeric: Option<&crate::world_quest_rewards::Settlement>,
    ) -> Result<Option<shared::currency::CurrencyWallet>, QuestFailed> {
        if quest.profession_knowledge_rewards.is_empty() {
            return Ok(None);
        }
        let catalog = self
            .io
            .effects
            .specializations
            .as_deref()
            .ok_or(failed(quest.id, QuestFailedReason::DontHaveRequirement))?;
        let (_, _, link) = self
            .io
            .units
            .connections
            .get(self.requester.connection)
            .map_err(|_| failed(quest.id, QuestFailedReason::DontHaveRequirement))?;
        let (professions, currencies) = self
            .io
            .units
            .profession_states
            .get(link.0)
            .map_err(|_| failed(quest.id, QuestFailedReason::DontHaveRequirement))?;
        let wallet = match numeric {
            Some(settlement) => &settlement.wallet,
            None => &currencies.0,
        };
        crate::profession::specializations::award_knowledge(catalog, professions, wallet, quest.id)
            .map(Some)
            .map_err(|error| {
                warn!(
                    "Quest {} knowledge reward refused for character {}: {error}",
                    quest.id,
                    self.character()
                );
                failed(quest.id, QuestFailedReason::DontHaveRequirement)
            })
    }

    fn persist_reward(
        &mut self,
        inventory: &InventoryData,
        log: &QuestLogData,
        appearances: Vec<u32>,
        numeric: Option<crate::world_quest_rewards::Settlement>,
        knowledge: Option<&shared::currency::CurrencyWallet>,
    ) {
        let character = self.character();
        let account = persistence::find_account_id_for_character(self.db, character)
            .expect("quest reward character has an account");
        let txn = self.db.begin_write().expect("begin quest reward write");
        persistence::appearance_store::learn_account_appearances_in(&txn, account, appearances);
        txn.open_table(persistence::INVENTORIES)
            .expect("open inventories")
            .insert(character, bitcode::encode(inventory).as_slice())
            .expect("write reward inventory");
        txn.open_table(crate::persistence::quest_store::QUEST_LOGS)
            .expect("open quest logs")
            .insert(character, bitcode::encode(log).as_slice())
            .expect("write rewarded quest log");
        if let Some(settlement) = &numeric {
            settlement.persist_in(&txn, character);
        }
        if let Some(wallet) = knowledge {
            crate::profession::specializations::write_currency_wallet(&txn, character, wallet);
        }
        txn.commit().expect("commit quest reward");
        if let Some(settlement) = numeric {
            let connection = self.requester.connection;
            self.io
                .commands
                .queue(move |world: &mut World| settlement.publish(world, connection));
        }
        self.io
            .effects
            .inventories
            .record(character, inventory.gold);
        self.io.effects.dirty.0.insert(character);
    }

    /// Retail 11.0.5 grants non-cosmetic appearances from every reward option,
    /// not just the item selected or an armor type the character can equip.
    fn reward_appearances(&self, quest: &QuestTemplate, selected: &[(u32, u32)]) -> Vec<u32> {
        if quest.reward_items.is_empty() && quest.choice_items.is_empty() {
            return Vec::new();
        }
        let templates = self
            .io
            .effects
            .templates
            .as_ref()
            .expect("quest rewards need item templates");
        quest
            .reward_items
            .iter()
            .chain(&quest.choice_items)
            .filter_map(|&(id, _)| {
                let template =
                    templates.get(shared::item_data::ItemDefinitionSource::Retail, id)?;
                let picked = selected.iter().any(|&(item_id, _)| item_id == id);
                let collect = !template.is_cosmetic() || (picked && template.binds_on_pickup());
                collect.then_some(template.appearance_id).flatten()
            })
            .collect()
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
        let details = quest_details(self.quests, quest, npc, self.requester.player);
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
        if provided.0 != 0 && bag_count(self.db, &inventory, provided.0) > 0 {
            remove_items(self.db, &mut inventory, provided.0, provided.1);
            self.save_inventory(&inventory, false);
        }
        // TC QuestHandler.cpp:463 after a successful log removal.
        let character = self.character();
        self.io.commands.queue(move |world: &mut World| {
            crate::achievement::record_character_event(
                world,
                character,
                crate::achievement::CriteriaEvent::AbandonQuest,
            );
        });
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
fn take_quest_items(db: &Database, inventory: &mut InventoryData, quest: &QuestTemplate) {
    for (index, objective) in quest.objectives.iter().enumerate() {
        let bound = crate::world_quest_objectives::rule(quest, index).quest_bound_item();
        if objective.kind == ObjectiveKind::Item && !bound {
            remove_items(db, inventory, objective.object_id, objective.required);
        }
    }
    if quest.start_item != 0 {
        remove_items(
            db,
            inventory,
            quest.start_item,
            quest.provided_item_count.max(1),
        );
    }
}
