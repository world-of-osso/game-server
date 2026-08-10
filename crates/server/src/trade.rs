use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Player, Position};
use shared::protocol::{
    AcceptTrade, CancelTrade, ClearTradeItem, ConfirmTrade, DeclineTrade, InitiateTrade,
    SetTradeItem, SetTradeMoney, TradeStateUpdate,
};
use shared::trade::{TradeItem, TradeManager};

use self::trade_support::{
    broadcast_trade_state, execute_completed_trade, find_player_by_character_id,
    find_player_by_name, other_character_id, position_tuple, send_trade_completion,
    send_trade_error, send_trade_message, send_trade_message_to_other,
    send_trade_snapshot_for_player, trade_error_message, validate_offered_item,
};
use crate::auth::Authenticated;
use crate::gold::GoldUpdateQueue;
use crate::networking::PlayerLink;
use crate::persistence::{self, ClientId, GameDatabase};

#[path = "trade_support.rs"]
mod trade_support;

#[derive(Resource, Default)]
struct ServerTradeState {
    manager: TradeManager,
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

fn process_initiate_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<InitiateTrade>),
        With<Authenticated>,
    >,
    players: PlayerQuery,
    connections: ConnectionQuery,
    db: Option<Res<GameDatabase>>,
    mut trade: ResMut<ServerTradeState>,
    mut senders: Query<&mut MessageSender<TradeStateUpdate>>,
) {
    let Some(db) = db else { return };
    let now = unix_now();
    for (conn_entity, link, mut receiver) in &mut receivers {
        let Ok((_, player, position, client_id)) = players.get(link.0) else {
            send_trade_error(
                conn_entity,
                &mut senders,
                "trade is unavailable: local player entity missing",
            );
            continue;
        };
        for req in receiver.receive() {
            let Some((_, target_player, target_pos, target_client_id)) =
                find_player_by_name(&players, &req.target_name)
            else {
                send_trade_error(conn_entity, &mut senders, "trade target is not online");
                continue;
            };
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
                    send_trade_snapshot_for_player(
                        &db.db,
                        client_id.0,
                        session,
                        &players,
                        &connections,
                        &mut senders,
                        Some(format!("trade request sent to {}", target_player.name)),
                    );
                    send_trade_snapshot_for_player(
                        &db.db,
                        target_client_id.0,
                        session,
                        &players,
                        &connections,
                        &mut senders,
                        Some(format!("{} wants to trade", player.name)),
                    );
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
            match trade.manager.accept_trade(client_id.0) {
                Ok(_) => {
                    if let Some(session) = trade.manager.get_session(client_id.0) {
                        broadcast_trade_state(
                            &db.db,
                            session,
                            &players,
                            &connections,
                            &mut senders,
                            Some("trade opened".into()),
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
    send_trade_message(conn_entity, senders, None, Some("trade declined".into()));
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
        Some("trade request declined".into()),
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
            match trade.manager.cancel_trade(client_id.0) {
                Ok(other_id) => {
                    send_trade_message(
                        conn_entity,
                        &mut senders,
                        None,
                        Some("trade cancelled".into()),
                    );
                    send_trade_message_to_other(
                        client_id.0,
                        other_id,
                        &connections,
                        &players,
                        &mut senders,
                        Some("trade cancelled".into()),
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
        for req in receiver.receive() {
            if let Err(error) = validate_offered_item(&db.db, client_id.0, &req) {
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
        &mut trade.manager,
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
        &mut trade.manager,
        &mut senders,
        |db, client_id, req, manager| {
            let inventory = persistence::load_inventory(db, client_id);
            if inventory.gold < req.copper {
                return Err("not enough gold for trade".into());
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
    manager: &mut TradeManager,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
    mut apply: F,
) where
    R: lightyear::prelude::Message,
    F: FnMut(&redb::Database, u64, &R, &mut TradeManager) -> Result<(), String>,
{
    for (conn_entity, link, mut receiver) in receivers.iter_mut() {
        let Ok((_, _, _, client_id)) = players.get(link.0) else {
            send_trade_error(
                conn_entity,
                senders,
                "trade is unavailable: local player entity missing",
            );
            continue;
        };
        for req in receiver.receive() {
            match apply(&db.db, client_id.0, &req, manager) {
                Ok(()) => {
                    if let Some(session) = manager.get_session(client_id.0) {
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
    mut trade: ResMut<ServerTradeState>,
    mut gold_updates: ResMut<GoldUpdateQueue>,
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
            handle_confirm_trade_request(
                conn_entity,
                client_id.0,
                &mut trade.manager,
                &db,
                &mut gold_updates,
                &connections,
                &players,
                &mut senders,
            );
        }
    }
}

fn handle_confirm_trade_request(
    conn_entity: Entity,
    character_id: u64,
    manager: &mut TradeManager,
    db: &GameDatabase,
    gold_updates: &mut GoldUpdateQueue,
    connections: &ConnectionQuery,
    players: &PlayerQuery,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
) {
    match manager.confirm_trade(character_id) {
        Ok(Some(completed)) => {
            let result = execute_and_send_completed_trade(
                db,
                &completed,
                gold_updates,
                connections,
                players,
                senders,
            );
            if let Err(error) = result {
                send_completed_trade_error(
                    conn_entity,
                    character_id,
                    &completed,
                    error,
                    connections,
                    players,
                    senders,
                );
            }
        }
        Ok(None) => {
            if let Some(session) = manager.get_session(character_id) {
                broadcast_trade_state(&db.db, session, players, connections, senders, None);
            }
        }
        Err(error) => send_trade_error(conn_entity, senders, trade_error_message(error)),
    }
}

fn execute_and_send_completed_trade(
    db: &GameDatabase,
    completed: &shared::trade::CompletedTrade,
    gold_updates: &mut GoldUpdateQueue,
    connections: &ConnectionQuery,
    players: &PlayerQuery,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
) -> Result<(), String> {
    execute_completed_trade(&db.db, completed, gold_updates)?;
    send_trade_completion(completed, connections, players, senders);
    Ok(())
}

fn send_completed_trade_error(
    conn_entity: Entity,
    character_id: u64,
    completed: &shared::trade::CompletedTrade,
    error: String,
    connections: &ConnectionQuery,
    players: &PlayerQuery,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
) {
    send_trade_error(conn_entity, senders, &error);
    let Some(other_id) = other_character_id(completed, character_id) else {
        return;
    };
    send_trade_message_to_other(
        character_id,
        other_id,
        connections,
        players,
        senders,
        Some(error),
    );
}

fn enforce_trade_range(
    players: PlayerQuery,
    connections: ConnectionQuery,
    mut trade: ResMut<ServerTradeState>,
    mut senders: Query<&mut MessageSender<TradeStateUpdate>>,
) {
    let cancelled = trade.manager.enforce_range(|character_id| {
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
            Some("trade cancelled: out of range".into()),
        );
        send_trade_message_to_other(
            target,
            target,
            &connections,
            &players,
            &mut senders,
            Some("trade cancelled: out of range".into()),
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
#[path = "trade_tests.rs"]
mod tests;
