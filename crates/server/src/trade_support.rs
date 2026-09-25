use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Player, Position};
use shared::protocol::{
    SetTradeItem, TradeChannel, TradeItemSnapshot, TradePartySnapshot, TradePhase, TradeSnapshot,
    TradeStateUpdate,
};
use shared::trade::{CompletedTrade, TRADE_SLOT_COUNT, TradeItem, TradeOffer};

use crate::gold::GoldUpdateQueue;
use crate::persistence::{self, ClientId, InventoryData, InventoryItemData};
use crate::trade::{ConnectionQuery, PlayerQuery};

pub(super) fn broadcast_trade_state(
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

pub(super) fn send_trade_snapshot_for_player(
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
    let (player_id, other_id) = trade_party_ids(session, viewer_is_initiator);
    let (player_offer, other_offer) = trade_offers_for_viewer(session, viewer_is_initiator);

    TradeSnapshot {
        phase: trade_phase_for_viewer(session, viewer_is_initiator),
        player: build_trade_party_snapshot(
            db,
            player_id,
            player_name(players, player_id),
            session.has_accepted(player_id),
            player_offer,
        ),
        other: build_trade_party_snapshot(
            db,
            other_id,
            player_name(players, other_id),
            session.has_accepted(other_id),
            other_offer,
        ),
    }
}

fn trade_party_ids(session: &shared::trade::TradeSession, viewer_is_initiator: bool) -> (u64, u64) {
    if viewer_is_initiator {
        (session.initiator, session.target)
    } else {
        (session.target, session.initiator)
    }
}

fn trade_phase_for_viewer(
    session: &shared::trade::TradeSession,
    viewer_is_initiator: bool,
) -> TradePhase {
    match session.state {
        shared::trade::TradeState::Pending if viewer_is_initiator => TradePhase::PendingOutgoing,
        shared::trade::TradeState::Pending => TradePhase::PendingIncoming,
        shared::trade::TradeState::Open => TradePhase::Open,
    }
}

fn trade_offers_for_viewer(
    session: &shared::trade::TradeSession,
    viewer_is_initiator: bool,
) -> (&TradeOffer, &TradeOffer) {
    if viewer_is_initiator {
        (&session.initiator_offer, &session.target_offer)
    } else {
        (&session.target_offer, &session.initiator_offer)
    }
}

pub(super) fn build_trade_party_snapshot(
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

pub(super) fn validate_offered_item(
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

pub(super) fn execute_completed_trade(
    db: &redb::Database,
    trade: &CompletedTrade,
    gold_updates: &mut GoldUpdateQueue,
) -> Result<(), String> {
    let mut initiator_inventory = persistence::load_inventory(db, trade.initiator);
    let mut target_inventory = persistence::load_inventory(db, trade.target);
    if initiator_inventory.gold < u64::from(trade.initiator_offer.gold) {
        return Err("trade failed: initiator no longer has enough gold".into());
    }
    if target_inventory.gold < u64::from(trade.target_offer.gold) {
        return Err("trade failed: target no longer has enough gold".into());
    }

    let initiator_items = take_offered_items(&mut initiator_inventory, &trade.initiator_offer)?;
    let target_items = take_offered_items(&mut target_inventory, &trade.target_offer)?;

    initiator_inventory.gold -= u64::from(trade.initiator_offer.gold);
    target_inventory.gold -= u64::from(trade.target_offer.gold);
    initiator_inventory.gold = initiator_inventory
        .gold
        .saturating_add(u64::from(trade.target_offer.gold));
    target_inventory.gold = target_inventory
        .gold
        .saturating_add(u64::from(trade.initiator_offer.gold));

    initiator_inventory.items.extend(target_items);
    target_inventory.items.extend(initiator_items);

    persistence::save_inventory(db, trade.initiator, &initiator_inventory);
    persistence::save_inventory(db, trade.target, &target_inventory);
    gold_updates.record(trade.initiator, initiator_inventory.gold);
    gold_updates.record(trade.target, target_inventory.gold);
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

pub(super) fn send_trade_completion(
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

pub(super) fn send_trade_message_to_other(
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

pub(super) fn send_trade_error(
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

pub(super) fn send_trade_message(
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

pub(super) fn connection_for_character(
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

pub(super) fn find_player_by_name<'w, 's>(
    players: &'w PlayerQuery<'w, 's>,
    name: &str,
) -> Option<(Entity, &'w Player, &'w Position, &'w ClientId)> {
    players
        .iter()
        .find(|(_, player, _, _)| player.name.eq_ignore_ascii_case(name))
}

pub(super) fn find_player_by_character_id<'w, 's>(
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

pub(super) fn other_character_id(completed: &CompletedTrade, player_id: u64) -> Option<u64> {
    if completed.initiator == player_id {
        Some(completed.target)
    } else if completed.target == player_id {
        Some(completed.initiator)
    } else {
        None
    }
}

pub(super) fn position_tuple(position: &Position) -> (f32, f32, f32) {
    (position.x, position.y, position.z)
}

pub(super) fn trade_error_message(error: shared::trade::TradeError) -> &'static str {
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
