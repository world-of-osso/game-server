use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Health, Player, Position};
use shared::protocol::{
    AcceptTrade, CancelTrade, CancelTradeAccept, ClearTradeItem, ConfirmTrade, DeclineTrade,
    InitiateTrade, SetTradeItem, SetTradeMoney, TradeStateUpdate,
};
use shared::trade::{TradeItem, TradeManager};

use self::trade_support::{
    ERR_NOT_ENOUGH_MONEY, ERR_TRADE_CANCELLED, ERR_TRADE_TOO_FAR, broadcast_trade_state,
    commit_completed_trade, find_player_by_character_id, find_player_by_name, position_tuple,
    send_trade_completion, send_trade_error, send_trade_message, send_trade_message_to_other,
    send_trade_snapshot_for_player, trade_error_message, validate_offered_item,
};
use crate::auras::ActiveAuras;
use crate::auth::Authenticated;
use crate::faction::faction_for_race;
use crate::gold::GoldUpdateQueue;
use crate::item_templates::ItemTemplates;
use crate::networking::PlayerLink;
use crate::persistence::{self, ClientId, GameDatabase};
use crate::taxi::TaxiFlight;

#[path = "trade_support.rs"]
mod trade_support;

#[derive(Resource, Default)]
struct ServerTradeState {
    manager: TradeManager,
    pending: Vec<PendingTrade>,
}

struct PendingTrade {
    completed: shared::trade::CompletedTrade,
    receipt: crate::persistence_writer::CommitReceipt<
        Result<[(u64, u64); 2], trade_support::ExchangeRefusal>,
    >,
}

impl ServerTradeState {
    fn is_committing(&self, character: u64) -> bool {
        self.pending.iter().any(|pending| {
            pending.completed.initiator == character || pending.completed.target == character
        })
    }
}

pub struct TradePlugin;

impl Plugin for TradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ServerTradeState>();
        app.add_systems(Update, process_initiate_requests);
        app.add_systems(Update, process_accept_requests);
        app.add_systems(Update, process_decline_requests);
        app.add_systems(Update, process_cancel_requests);
        app.add_systems(Update, process_set_item_requests);
        app.add_systems(Update, process_clear_item_requests);
        app.add_systems(Update, process_set_money_requests);
        app.add_systems(Update, process_confirm_requests);
        app.add_systems(Update, process_cancel_accept_requests);
        app.add_systems(FixedUpdate, enforce_trade_range);
    }
}

type PlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Player,
        &'static Position,
        &'static ClientId,
    ),
>;
type ConnectionQuery<'w, 's> = Query<'w, 's, (Entity, &'static PlayerLink), With<Authenticated>>;
/// What AzerothCore `HandleInitiateTradeOpcode` checks on both players.
type TraderStates<'w, 's> = Query<
    'w,
    's,
    (
        &'static Player,
        Option<&'static Health>,
        Option<&'static ActiveAuras>,
        Has<TaxiFlight>,
    ),
>;

/// A player as trade initiation sees them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Trader {
    pub alive: bool,
    pub stunned: bool,
    pub in_flight: bool,
    pub faction: u8,
}

fn trader(states: &TraderStates, player: Entity) -> Option<Trader> {
    let (player, health, auras, in_flight) = states.get(player).ok()?;
    Some(Trader {
        alive: health.is_none_or(|health| health.current > 0.0),
        stunned: auras.is_some_and(ActiveAuras::is_stunned),
        in_flight,
        faction: faction_for_race(player.race),
    })
}

/// AzerothCore `HandleInitiateTradeOpcode` refusals in its order, as Retail text;
/// `None` lets the request through to the busy and range checks.
pub(crate) fn initiation_refusal(me: Trader, other: Trader) -> Option<&'static str> {
    if !me.alive {
        return Some("You can't do that when you're dead."); // TRADE_STATUS_YOU_DEAD
    }
    if me.stunned {
        return Some("You can't do that right now."); // TRADE_STATUS_YOU_STUNNED
    }
    if me.in_flight || other.in_flight {
        return Some(ERR_TRADE_TOO_FAR); // TRADE_STATUS_TARGET_TO_FAR
    }
    if !other.alive {
        return Some("You can't trade with dead players."); // ERR_TRADE_TARGET_DEAD
    }
    if other.stunned {
        return Some("Target is stunned"); // ERR_TARGET_STUNNED
    }
    if me.faction != other.faction {
        return Some("Target is unfriendly."); // TRADE_STATUS_WRONG_FACTION
    }
    None
}

fn process_initiate_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<InitiateTrade>),
        With<Authenticated>,
    >,
    players: PlayerQuery,
    connections: ConnectionQuery,
    states: TraderStates,
    db: Option<Res<GameDatabase>>,
    mut trade: ResMut<ServerTradeState>,
    mut senders: Query<&mut MessageSender<TradeStateUpdate>>,
) {
    let Some(db) = db else { return };
    let now = unix_now();
    for (conn_entity, link, mut receiver) in &mut receivers {
        let Ok((_, _, position, client_id)) = players.get(link.0) else {
            send_trade_error(
                conn_entity,
                &mut senders,
                "trade is unavailable: local player entity missing",
            );
            continue;
        };
        for req in receiver.receive() {
            let Some((target_entity, target_player, target_pos, target_client_id)) =
                find_player_by_name(&players, &req.target_name)
            else {
                send_trade_error(
                    conn_entity,
                    &mut senders,
                    format!(
                        "No player named '{}' is currently playing.",
                        req.target_name
                    ), // ERR_CHAT_PLAYER_NOT_FOUND_S
                );
                continue;
            };
            let refusal = trader(&states, link.0)
                .zip(trader(&states, target_entity))
                .map(|(me, other)| initiation_refusal(me, other));
            match refusal {
                Some(None) => {}
                Some(Some(text)) => {
                    send_trade_error(conn_entity, &mut senders, text);
                    continue;
                }
                None => {
                    send_trade_error(conn_entity, &mut senders, ERR_TRADE_CANCELLED);
                    continue;
                }
            }
            if trade.manager.is_trading(target_client_id.0) {
                let busy = format!("{} is busy right now.", target_player.name); // ERR_PLAYER_BUSY_S
                send_trade_error(conn_entity, &mut senders, busy);
                continue;
            }
            let result = trade.manager.initiate_trade(
                client_id.0,
                target_client_id.0,
                position_tuple(position),
                position_tuple(target_pos),
                now,
            );
            match result {
                Ok(_) => {
                    let Some(session) = trade.manager.get_session(client_id.0) else {
                        send_trade_error(
                            conn_entity,
                            &mut senders,
                            "trade session was not created",
                        );
                        continue;
                    };
                    for character_id in [client_id.0, target_client_id.0] {
                        send_trade_snapshot_for_player(
                            &db.db,
                            character_id,
                            session,
                            &players,
                            &connections,
                            &mut senders,
                            None,
                        );
                    }
                }
                Err(error) => {
                    send_trade_error(conn_entity, &mut senders, trade_error_message(error));
                }
            }
        }
    }
}

fn process_accept_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<AcceptTrade>),
        With<Authenticated>,
    >,
    players: PlayerQuery,
    connections: ConnectionQuery,
    db: Option<Res<GameDatabase>>,
    mut trade: ResMut<ServerTradeState>,
    mut senders: Query<&mut MessageSender<TradeStateUpdate>>,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, mut receiver) in &mut receivers {
        let Ok((_, _, _, client_id)) = players.get(link.0) else {
            send_trade_error(
                conn_entity,
                &mut senders,
                "trade is unavailable: local player entity missing",
            );
            continue;
        };
        for _ in receiver.receive() {
            if trade.is_committing(client_id.0) {
                continue;
            }
            match trade.manager.accept_trade(client_id.0) {
                Ok(_) => {
                    if let Some(session) = trade.manager.get_session(client_id.0) {
                        broadcast_trade_state(
                            &db.db,
                            session,
                            &players,
                            &connections,
                            &mut senders,
                            None,
                        );
                    }
                }
                Err(error) => {
                    send_trade_error(conn_entity, &mut senders, trade_error_message(error));
                }
            }
        }
    }
}

fn process_decline_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<DeclineTrade>),
        With<Authenticated>,
    >,
    players: PlayerQuery,
    connections: ConnectionQuery,
    mut trade: ResMut<ServerTradeState>,
    mut senders: Query<&mut MessageSender<TradeStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        let Ok((_, _, _, client_id)) = players.get(link.0) else {
            send_trade_error(
                conn_entity,
                &mut senders,
                "trade is unavailable: local player entity missing",
            );
            continue;
        };
        for _ in receiver.receive() {
            if trade.is_committing(client_id.0) {
                continue;
            }
            if let Err(error) = handle_decline_request(
                conn_entity,
                client_id.0,
                &players,
                &connections,
                &mut trade.manager,
                &mut senders,
            ) {
                send_trade_error(conn_entity, &mut senders, trade_error_message(error));
            }
        }
    }
}

fn handle_decline_request(
    conn_entity: Entity,
    client_id: u64,
    players: &PlayerQuery,
    connections: &ConnectionQuery,
    manager: &mut TradeManager,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
) -> Result<(), shared::trade::TradeError> {
    let other_id = decline_target(manager, client_id);
    manager.decline_trade(client_id)?;
    send_trade_message(conn_entity, senders, None, Some(ERR_TRADE_CANCELLED.into()));
    notify_other_declined_trade(client_id, other_id, players, connections, senders);
    Ok(())
}

fn decline_target(manager: &TradeManager, client_id: u64) -> Option<u64> {
    manager
        .get_session(client_id)
        .and_then(|session| session.other_player(client_id))
}

fn notify_other_declined_trade(
    client_id: u64,
    other_id: Option<u64>,
    players: &PlayerQuery,
    connections: &ConnectionQuery,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
) {
    let Some(other_id) = other_id else {
        return;
    };
    send_trade_message_to_other(
        client_id,
        other_id,
        connections,
        players,
        senders,
        Some(ERR_TRADE_CANCELLED.into()),
    );
}

fn process_cancel_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<CancelTrade>),
        With<Authenticated>,
    >,
    players: PlayerQuery,
    connections: ConnectionQuery,
    mut trade: ResMut<ServerTradeState>,
    mut senders: Query<&mut MessageSender<TradeStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        let Ok((_, _, _, client_id)) = players.get(link.0) else {
            send_trade_error(
                conn_entity,
                &mut senders,
                "trade is unavailable: local player entity missing",
            );
            continue;
        };
        for _ in receiver.receive() {
            if trade.is_committing(client_id.0) {
                continue;
            }
            match trade.manager.cancel_trade(client_id.0) {
                Ok(other_id) => {
                    send_trade_message(
                        conn_entity,
                        &mut senders,
                        None,
                        Some(ERR_TRADE_CANCELLED.into()),
                    );
                    send_trade_message_to_other(
                        client_id.0,
                        other_id,
                        &connections,
                        &players,
                        &mut senders,
                        Some(ERR_TRADE_CANCELLED.into()),
                    );
                }
                Err(error) => {
                    send_trade_error(conn_entity, &mut senders, trade_error_message(error));
                }
            }
        }
    }
}

fn process_set_item_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<SetTradeItem>),
        With<Authenticated>,
    >,
    players: PlayerQuery,
    connections: ConnectionQuery,
    db: Option<Res<GameDatabase>>,
    templates: Option<Res<ItemTemplates>>,
    mut trade: ResMut<ServerTradeState>,
    mut senders: Query<&mut MessageSender<TradeStateUpdate>>,
) {
    let (Some(db), Some(templates)) = (db, templates) else {
        return;
    };
    for (conn_entity, link, mut receiver) in &mut receivers {
        let Ok((_, _, _, client_id)) = players.get(link.0) else {
            send_trade_error(
                conn_entity,
                &mut senders,
                "trade is unavailable: local player entity missing",
            );
            continue;
        };
        for req in receiver.receive() {
            if trade.is_committing(client_id.0) {
                continue;
            }
            if let Err(error) = validate_offered_item(&db.db, &templates, client_id.0, &req) {
                send_trade_error(conn_entity, &mut senders, &error);
                continue;
            }
            match trade.manager.set_item(
                client_id.0,
                usize::from(req.slot),
                TradeItem {
                    item_guid: req.item_guid,
                    count: req.stack_count,
                },
            ) {
                Ok(()) => {
                    if let Some(session) = trade.manager.get_session(client_id.0) {
                        broadcast_trade_state(
                            &db.db,
                            session,
                            &players,
                            &connections,
                            &mut senders,
                            None,
                        );
                    }
                }
                Err(error) => {
                    send_trade_error(conn_entity, &mut senders, trade_error_message(error));
                }
            }
        }
    }
}

fn process_clear_item_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<ClearTradeItem>),
        With<Authenticated>,
    >,
    players: PlayerQuery,
    connections: ConnectionQuery,
    db: Option<Res<GameDatabase>>,
    mut trade: ResMut<ServerTradeState>,
    mut senders: Query<&mut MessageSender<TradeStateUpdate>>,
) {
    let Some(db) = db else { return };
    process_trade_offer_updates(
        &mut receivers,
        &players,
        &connections,
        db.as_ref(),
        &mut trade,
        &mut senders,
        |_, client_id, req, manager| {
            manager
                .clear_item(client_id, usize::from(req.slot))
                .map_err(|error| trade_error_message(error).to_string())
        },
    );
}

fn process_set_money_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<SetTradeMoney>),
        With<Authenticated>,
    >,
    players: PlayerQuery,
    connections: ConnectionQuery,
    db: Option<Res<GameDatabase>>,
    mut trade: ResMut<ServerTradeState>,
    mut senders: Query<&mut MessageSender<TradeStateUpdate>>,
) {
    let Some(db) = db else { return };
    process_trade_offer_updates(
        &mut receivers,
        &players,
        &connections,
        db.as_ref(),
        &mut trade,
        &mut senders,
        |db, client_id, req, manager| {
            let inventory = persistence::load_inventory(db, client_id);
            if inventory.gold < req.copper {
                return Err(ERR_NOT_ENOUGH_MONEY.into());
            }
            manager
                .set_gold(client_id, req.copper)
                .map_err(|error| trade_error_message(error).to_string())
        },
    );
}

fn process_trade_offer_updates<R, F>(
    receivers: &mut Query<(Entity, &PlayerLink, &mut MessageReceiver<R>), With<Authenticated>>,
    players: &PlayerQuery,
    connections: &ConnectionQuery,
    db: &GameDatabase,
    trade: &mut ServerTradeState,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
    mut apply: F,
) where
    R: lightyear::prelude::Message,
    F: FnMut(&redb::Database, u64, &R, &mut TradeManager) -> Result<(), String>,
{
    for (conn_entity, link, mut receiver) in receivers.iter_mut() {
        if !receiver.has_messages() {
            continue;
        }

        let Ok((_, _, _, client_id)) = players.get(link.0) else {
            send_trade_error(
                conn_entity,
                senders,
                "trade is unavailable: local player entity missing",
            );
            continue;
        };
        for req in receiver.receive() {
            if trade.is_committing(client_id.0) {
                continue;
            }
            match apply(&db.db, client_id.0, &req, &mut trade.manager) {
                Ok(()) => {
                    if let Some(session) = trade.manager.get_session(client_id.0) {
                        broadcast_trade_state(&db.db, session, players, connections, senders, None);
                    }
                }
                Err(error) => send_trade_error(conn_entity, senders, &error),
            }
        }
    }
}

fn process_confirm_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<ConfirmTrade>),
        With<Authenticated>,
    >,
    players: PlayerQuery,
    connections: ConnectionQuery,
    db: Option<Res<GameDatabase>>,
    templates: Option<Res<ItemTemplates>>,
    mut trade: ResMut<ServerTradeState>,
    mut gold_updates: ResMut<GoldUpdateQueue>,
    mut senders: Query<&mut MessageSender<TradeStateUpdate>>,
) {
    let (Some(db), Some(templates)) = (db, templates) else {
        return;
    };
    let ServerTradeState { manager, pending } = &mut *trade;
    let mut exchange = Exchange {
        db: &db,
        templates: &templates,
        gold_updates: &mut gold_updates,
        connections: &connections,
        players: &players,
        pending,
    };
    exchange.publish_completed(manager, &mut senders);
    for (conn_entity, link, mut receiver) in &mut receivers {
        let Ok((_, _, _, client_id)) = players.get(link.0) else {
            send_trade_error(
                conn_entity,
                &mut senders,
                "trade is unavailable: local player entity missing",
            );
            continue;
        };
        for _ in receiver.receive() {
            exchange.confirm(conn_entity, client_id.0, manager, &mut senders);
        }
    }
}

/// Everything an accepted trade needs to execute and report.
struct Exchange<'a, 'w, 's> {
    db: &'a GameDatabase,
    templates: &'a ItemTemplates,
    gold_updates: &'a mut GoldUpdateQueue,
    connections: &'a ConnectionQuery<'w, 's>,
    players: &'a PlayerQuery<'w, 's>,
    pending: &'a mut Vec<PendingTrade>,
}

impl Exchange<'_, '_, '_> {
    /// AzerothCore `HandleAcceptTradeOpcode`: once both accepted, execute; a refusal
    /// keeps the window open with both accepts cleared and each side's reason.
    fn confirm(
        &mut self,
        connection: Entity,
        character_id: u64,
        manager: &mut TradeManager,
        senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
    ) {
        if self.pending.iter().any(|pending| {
            pending.completed.initiator == character_id || pending.completed.target == character_id
        }) {
            return;
        }
        let completed = match manager.confirm_trade(character_id) {
            Ok(Some(completed)) => completed,
            Ok(None) => {
                if let Some(session) = manager.get_session(character_id) {
                    broadcast_trade_state(
                        &self.db.db,
                        session,
                        self.players,
                        self.connections,
                        senders,
                        None,
                    );
                }
                return;
            }
            Err(error) => {
                send_trade_error(connection, senders, trade_error_message(error));
                return;
            }
        };
        let snapshot = completed.clone();
        let templates = self.templates.clone();
        let receipt = self.db.writer.enqueue("completed player trade", move |db| {
            Ok(commit_completed_trade(db, &templates, &snapshot))
        });
        self.pending.push(PendingTrade { completed, receipt });
    }

    fn publish_completed(
        &mut self,
        manager: &mut TradeManager,
        senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
    ) {
        for pending in std::mem::take(self.pending) {
            let Some(result) = pending.receipt.result() else {
                self.pending.push(pending);
                continue;
            };
            let completed = pending.completed;
            match result {
                Ok(Ok(gold)) => {
                    for (character, value) in gold {
                        self.gold_updates.record(character, value);
                    }
                    manager.finish_trade(completed.initiator);
                    send_trade_completion(&completed, self.connections, self.players, senders);
                }
                failure => {
                    manager.refuse_accepts(completed.initiator);
                    let Some(session) = manager.get_session(completed.initiator) else {
                        continue;
                    };
                    for viewer in [completed.initiator, completed.target] {
                        let Some(connection) = trade_support::connection_for_character(
                            self.players,
                            self.connections,
                            viewer,
                        ) else {
                            continue;
                        };
                        let error = match &failure {
                            Ok(Err(refusal)) => refusal.message_for(viewer),
                            _ => "Trade failed.",
                        };
                        let snapshot = trade_support::build_trade_snapshot(
                            &self.db.db,
                            viewer,
                            session,
                            self.players,
                        );
                        if let Ok(mut sender) = senders.get_mut(connection) {
                            sender.send::<shared::protocol::TradeChannel>(TradeStateUpdate {
                                trade: Some(snapshot),
                                message: None,
                                error: Some(error.into()),
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Retail `CancelTradeAccept`: withdraw the player's own accept.
fn process_cancel_accept_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<CancelTradeAccept>),
        With<Authenticated>,
    >,
    players: PlayerQuery,
    connections: ConnectionQuery,
    db: Option<Res<GameDatabase>>,
    mut trade: ResMut<ServerTradeState>,
    mut senders: Query<&mut MessageSender<TradeStateUpdate>>,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, mut receiver) in &mut receivers {
        let Ok((_, _, _, client_id)) = players.get(link.0) else {
            continue;
        };
        for _ in receiver.receive() {
            if trade.is_committing(client_id.0) {
                continue;
            }
            match trade.manager.unconfirm_trade(client_id.0) {
                Ok(()) => {
                    if let Some(session) = trade.manager.get_session(client_id.0) {
                        broadcast_trade_state(
                            &db.db,
                            session,
                            &players,
                            &connections,
                            &mut senders,
                            None,
                        );
                    }
                }
                Err(error) => {
                    send_trade_error(conn_entity, &mut senders, trade_error_message(error));
                }
            }
        }
    }
}

fn enforce_trade_range(
    players: PlayerQuery,
    connections: ConnectionQuery,
    mut trade: ResMut<ServerTradeState>,
    mut senders: Query<&mut MessageSender<TradeStateUpdate>>,
) {
    let committing: std::collections::HashSet<_> = trade
        .pending
        .iter()
        .flat_map(|pending| [pending.completed.initiator, pending.completed.target])
        .collect();
    let cancelled = trade.manager.enforce_range(|character_id| {
        if committing.contains(&character_id) {
            return Some((0.0, 0.0, 0.0));
        }
        find_player_by_character_id(&players, character_id)
            .map(|(_, _, position, _)| position_tuple(position))
    });
    for (initiator, target) in cancelled {
        send_trade_message_to_other(
            initiator,
            initiator,
            &connections,
            &players,
            &mut senders,
            Some(ERR_TRADE_TOO_FAR.into()),
        );
        send_trade_message_to_other(
            target,
            target,
            &connections,
            &players,
            &mut senders,
            Some(ERR_TRADE_TOO_FAR.into()),
        );
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "inventory_source_tests.rs"]
mod source_tests;

#[cfg(test)]
#[path = "trade_tests.rs"]
mod tests;
