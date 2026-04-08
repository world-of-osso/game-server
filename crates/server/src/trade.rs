use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Player, Position};
use shared::protocol::{
    AcceptTrade, CancelTrade, ClearTradeItem, ConfirmTrade, DeclineTrade, InitiateTrade,
    SetTradeItem, SetTradeMoney, TradeChannel, TradeItemSnapshot, TradePartySnapshot, TradePhase,
    TradeSnapshot, TradeStateUpdate,
};
use shared::trade::{CompletedTrade, TRADE_SLOT_COUNT, TradeItem, TradeManager, TradeOffer};

use crate::auth::Authenticated;
use crate::networking::PlayerLink;
use crate::persistence::{self, ClientId, GameDatabase, InventoryData, InventoryItemData};

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
            let other_id = trade
                .manager
                .get_session(client_id.0)
                .and_then(|session| session.other_player(client_id.0));
            match trade.manager.decline_trade(client_id.0) {
                Ok(()) => {
                    send_trade_message(
                        conn_entity,
                        &mut senders,
                        None,
                        Some("trade declined".into()),
                    );
                    if let Some(other_id) = other_id {
                        send_trade_message_to_other(
                            client_id.0,
                            other_id,
                            &connections,
                            &players,
                            &mut senders,
                            Some("trade request declined".into()),
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
            match trade.manager.clear_item(client_id.0, usize::from(req.slot)) {
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
            let inventory = persistence::load_inventory(&db.db, client_id.0);
            if inventory.gold < req.copper {
                send_trade_error(conn_entity, &mut senders, "not enough gold for trade");
                continue;
            }
            match trade.manager.set_gold(client_id.0, req.copper) {
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

fn process_confirm_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<ConfirmTrade>),
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
            match trade.manager.confirm_trade(client_id.0) {
                Ok(Some(completed)) => match execute_completed_trade(&db.db, &completed) {
                    Ok(()) => {
                        send_trade_completion(&completed, &connections, &players, &mut senders);
                    }
                    Err(error) => {
                        send_trade_error(conn_entity, &mut senders, &error);
                        if let Some(other_id) = other_character_id(&completed, client_id.0) {
                            send_trade_message_to_other(
                                client_id.0,
                                other_id,
                                &connections,
                                &players,
                                &mut senders,
                                Some(error),
                            );
                        }
                    }
                },
                Ok(None) => {
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

fn broadcast_trade_state(
    db: &redb::Database,
    session: &shared::trade::TradeSession,
    players: &PlayerQuery,
    connections: &ConnectionQuery,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
    message: Option<String>,
) {
    send_trade_snapshot_for_player(
        db,
        session.initiator,
        session,
        players,
        connections,
        senders,
        message.clone(),
    );
    send_trade_snapshot_for_player(
        db,
        session.target,
        session,
        players,
        connections,
        senders,
        message,
    );
}

fn send_trade_snapshot_for_player(
    db: &redb::Database,
    character_id: u64,
    session: &shared::trade::TradeSession,
    players: &PlayerQuery,
    connections: &ConnectionQuery,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
    message: Option<String>,
) {
    let Some(conn_entity) = connection_for_character(players, connections, character_id) else {
        return;
    };
    let snapshot = build_trade_snapshot(db, character_id, session, players);
    send_trade_message(conn_entity, senders, Some(snapshot), message);
}

fn build_trade_snapshot(
    db: &redb::Database,
    viewer_id: u64,
    session: &shared::trade::TradeSession,
    players: &PlayerQuery,
) -> TradeSnapshot {
    let viewer_is_initiator = viewer_id == session.initiator;
    let player_id = if viewer_is_initiator {
        session.initiator
    } else {
        session.target
    };
    let other_id = if viewer_is_initiator {
        session.target
    } else {
        session.initiator
    };
    let player_name_value = player_name(players, player_id);
    let other_name_value = player_name(players, other_id);
    let phase = match session.state {
        shared::trade::TradeState::Pending if viewer_is_initiator => TradePhase::PendingOutgoing,
        shared::trade::TradeState::Pending => TradePhase::PendingIncoming,
        shared::trade::TradeState::Open => TradePhase::Open,
    };
    let player_offer = if viewer_is_initiator {
        &session.initiator_offer
    } else {
        &session.target_offer
    };
    let other_offer = if viewer_is_initiator {
        &session.target_offer
    } else {
        &session.initiator_offer
    };
    TradeSnapshot {
        phase,
        player: build_trade_party_snapshot(
            db,
            player_id,
            player_name_value,
            session.has_accepted(player_id),
            player_offer,
        ),
        other: build_trade_party_snapshot(
            db,
            other_id,
            other_name_value,
            session.has_accepted(other_id),
            other_offer,
        ),
    }
}

fn build_trade_party_snapshot(
    db: &redb::Database,
    character_id: u64,
    name: String,
    accepted: bool,
    offer: &TradeOffer,
) -> TradePartySnapshot {
    let inventory = persistence::load_inventory(db, character_id);
    let slots = offer
        .slots
        .iter()
        .map(|slot| {
            slot.as_ref()
                .map(|item| snapshot_trade_item(&inventory, item))
        })
        .collect();
    TradePartySnapshot {
        name,
        accepted,
        gold: offer.gold,
        slots,
    }
}

fn snapshot_trade_item(inventory: &InventoryData, item: &TradeItem) -> TradeItemSnapshot {
    if let Some(entry) = inventory
        .items
        .iter()
        .find(|entry| entry.item_guid == item.item_guid)
    {
        TradeItemSnapshot {
            item_guid: entry.item_guid,
            item_id: entry.item_id,
            name: entry.name.clone(),
            quality: entry.quality,
            stack_count: u32::from(item.count),
        }
    } else {
        TradeItemSnapshot {
            item_guid: item.item_guid,
            item_id: 0,
            name: format!("Item {}", item.item_guid),
            quality: 1,
            stack_count: u32::from(item.count),
        }
    }
}

fn validate_offered_item(
    db: &redb::Database,
    character_id: u64,
    req: &SetTradeItem,
) -> Result<(), String> {
    if usize::from(req.slot) >= TRADE_SLOT_COUNT {
        return Err("trade slot is out of range".into());
    }
    if req.stack_count == 0 {
        return Err("trade stack count must be greater than zero".into());
    }
    let inventory = persistence::load_inventory(db, character_id);
    let Some(item) = inventory
        .items
        .iter()
        .find(|item| item.item_guid == req.item_guid)
    else {
        return Err("item is not in inventory".into());
    };
    if item.stack_count < u32::from(req.stack_count) {
        return Err("not enough quantity for trade offer".into());
    }
    Ok(())
}

fn execute_completed_trade(db: &redb::Database, trade: &CompletedTrade) -> Result<(), String> {
    let mut initiator_inventory = persistence::load_inventory(db, trade.initiator);
    let mut target_inventory = persistence::load_inventory(db, trade.target);
    if initiator_inventory.gold < trade.initiator_offer.gold {
        return Err("trade failed: initiator no longer has enough gold".into());
    }
    if target_inventory.gold < trade.target_offer.gold {
        return Err("trade failed: target no longer has enough gold".into());
    }

    let initiator_items = take_offered_items(&mut initiator_inventory, &trade.initiator_offer)?;
    let target_items = take_offered_items(&mut target_inventory, &trade.target_offer)?;

    initiator_inventory.gold -= trade.initiator_offer.gold;
    target_inventory.gold -= trade.target_offer.gold;
    initiator_inventory.gold = initiator_inventory
        .gold
        .saturating_add(trade.target_offer.gold);
    target_inventory.gold = target_inventory
        .gold
        .saturating_add(trade.initiator_offer.gold);

    initiator_inventory.items.extend(target_items);
    target_inventory.items.extend(initiator_items);

    persistence::save_inventory(db, trade.initiator, &initiator_inventory);
    persistence::save_inventory(db, trade.target, &target_inventory);
    Ok(())
}

fn take_offered_items(
    inventory: &mut InventoryData,
    offer: &TradeOffer,
) -> Result<Vec<InventoryItemData>, String> {
    let mut moved = Vec::new();
    for item in offer.slots.iter().flatten() {
        let Some(index) = inventory
            .items
            .iter()
            .position(|entry| entry.item_guid == item.item_guid)
        else {
            return Err(format!(
                "trade item {} is no longer in inventory",
                item.item_guid
            ));
        };
        if inventory.items[index].stack_count < u32::from(item.count) {
            return Err(format!(
                "trade item {} no longer has enough quantity",
                item.item_guid
            ));
        }
        let mut moved_item = inventory.items[index].clone();
        moved_item.stack_count = u32::from(item.count);
        inventory.items[index].stack_count -= u32::from(item.count);
        if inventory.items[index].stack_count == 0 {
            inventory.items.remove(index);
        }
        moved.push(moved_item);
    }
    Ok(moved)
}

fn send_trade_completion(
    completed: &CompletedTrade,
    connections: &ConnectionQuery,
    players: &PlayerQuery,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
) {
    send_trade_message_to_other(
        completed.initiator,
        completed.initiator,
        connections,
        players,
        senders,
        Some("trade completed".into()),
    );
    send_trade_message_to_other(
        completed.target,
        completed.target,
        connections,
        players,
        senders,
        Some("trade completed".into()),
    );
}

fn send_trade_message_to_other(
    _from_character_id: u64,
    to_character_id: u64,
    connections: &ConnectionQuery,
    players: &PlayerQuery,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
    message: Option<String>,
) {
    let Some(conn_entity) = connection_for_character(players, connections, to_character_id) else {
        return;
    };
    send_trade_message(conn_entity, senders, None, message);
}

fn send_trade_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
    error: impl Into<String>,
) {
    if let Ok(mut sender) = senders.get_mut(conn_entity) {
        sender.send::<TradeChannel>(TradeStateUpdate {
            trade: None,
            message: None,
            error: Some(error.into()),
        });
    }
}

fn send_trade_message(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
    trade: Option<TradeSnapshot>,
    message: Option<String>,
) {
    if let Ok(mut sender) = senders.get_mut(conn_entity) {
        sender.send::<TradeChannel>(TradeStateUpdate {
            trade,
            message,
            error: None,
        });
    }
}

fn connection_for_character(
    players: &PlayerQuery,
    connections: &ConnectionQuery,
    character_id: u64,
) -> Option<Entity> {
    let (player_entity, _, _, _) = find_player_by_character_id(players, character_id)?;
    connections
        .iter()
        .find(|(_, link)| link.0 == player_entity)
        .map(|(entity, _)| entity)
}

fn find_player_by_name<'w, 's>(
    players: &'w PlayerQuery<'w, 's>,
    name: &str,
) -> Option<(Entity, &'w Player, &'w Position, &'w ClientId)> {
    players
        .iter()
        .find(|(_, player, _, _)| player.name.eq_ignore_ascii_case(name))
}

fn find_player_by_character_id<'w, 's>(
    players: &'w PlayerQuery<'w, 's>,
    character_id: u64,
) -> Option<(Entity, &'w Player, &'w Position, &'w ClientId)> {
    players
        .iter()
        .find(|(_, _, _, client_id)| client_id.0 == character_id)
}

fn player_name(players: &PlayerQuery, character_id: u64) -> String {
    find_player_by_character_id(players, character_id)
        .map(|(_, player, _, _)| player.name.clone())
        .unwrap_or_else(|| format!("Player {character_id}"))
}

fn other_character_id(completed: &CompletedTrade, player_id: u64) -> Option<u64> {
    if completed.initiator == player_id {
        Some(completed.target)
    } else if completed.target == player_id {
        Some(completed.initiator)
    } else {
        None
    }
}

fn position_tuple(position: &Position) -> (f32, f32, f32) {
    (position.x, position.y, position.z)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn trade_error_message(error: shared::trade::TradeError) -> &'static str {
    match error {
        shared::trade::TradeError::OutOfRange => "trade target is out of range",
        shared::trade::TradeError::CannotTradeWithSelf => "cannot trade with yourself",
        shared::trade::TradeError::AlreadyTrading => "one of the players is already trading",
        shared::trade::TradeError::NoPendingRequest => "no pending trade request",
        shared::trade::TradeError::NoActiveSession => "no active trade session",
        shared::trade::TradeError::InvalidTarget => "invalid trade target",
        shared::trade::TradeError::PlayerDead => "cannot trade while dead",
        shared::trade::TradeError::PlayerInCombat => "cannot trade while in combat",
        shared::trade::TradeError::TradeNotOpen => "trade window is not open",
        shared::trade::TradeError::InvalidSlot => "trade slot is out of range",
        shared::trade::TradeError::ItemAlreadyOffered => "item is already offered in trade",
        shared::trade::TradeError::NotConfirmed => "trade is not confirmed",
        shared::trade::TradeError::ItemSoulbound => "item is soulbound and cannot be traded",
    }
}

#[cfg(test)]
#[path = "trade_tests.rs"]
mod tests;
