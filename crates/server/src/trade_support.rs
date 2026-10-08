use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Player, Position};
use shared::protocol::{
    SetTradeItem, TradeChannel, TradeItemSnapshot, TradePartySnapshot, TradePhase, TradeSnapshot,
    TradeStateUpdate,
};
use shared::trade::{
    CompletedTrade, TRADE_SLOT_COUNT, TRADE_SLOT_NONTRADED, TradeItem, TradeOffer,
};

use crate::gold::{GoldUpdateQueue, MAX_MONEY};
use crate::inventory::{inventory_model::InventoryModel, load_model};
use crate::item_templates::ItemTemplates;
use crate::persistence::inventory_store::write_inventory_record;
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

pub(super) fn build_trade_snapshot(
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
                .map(|item| snapshot_trade_item(db, &inventory, item))
        })
        .collect();

    TradePartySnapshot {
        name,
        accepted,
        gold: offer.gold,
        slots,
    }
}

fn snapshot_trade_item(
    db: &redb::Database,
    inventory: &InventoryData,
    item: &TradeItem,
) -> TradeItemSnapshot {
    if let Some(entry) = inventory
        .items
        .iter()
        .find(|entry| entry.item_guid == item.item_guid)
    {
        TradeItemSnapshot {
            item_guid: entry.item_guid,
            item_id: entry.item_id,
            definition_source: persistence::inventory_store::load_item_definition_source(
                db,
                entry.item_guid,
            ),
            name: entry.name.clone(),
            quality: entry.quality,
            stack_count: u32::from(item.count),
        }
    } else {
        TradeItemSnapshot {
            item_guid: item.item_guid,
            item_id: 0,
            definition_source: persistence::inventory_store::load_item_definition_source(
                db,
                item.item_guid,
            ),
            name: format!("Item {}", item.item_guid),
            quality: 1,
            stack_count: u32::from(item.count),
        }
    }
}

/// AzerothCore `HandleSetTradeItemOpcode` / `Item::CanBeTraded`: a bag item with
/// enough in the stack, not soulbound unless it sits in the "Will not be traded" slot.
pub(super) fn validate_offered_item(
    db: &redb::Database,
    templates: &ItemTemplates,
    character_id: u64,
    req: &SetTradeItem,
) -> Result<(), String> {
    let slot = usize::from(req.slot);
    if slot >= TRADE_SLOT_COUNT || req.stack_count == 0 {
        return Err(ERR_ITEM_NOT_FOUND.into());
    }
    let model = load_model(db, templates, character_id).map_err(|_| ERR_ITEM_NOT_FOUND)?;
    let Some(item) = model.bag_item(req.item_guid) else {
        return Err(ERR_ITEM_NOT_FOUND.into());
    };
    if item.stack_count < u32::from(req.stack_count) {
        return Err(ERR_ITEM_NOT_FOUND.into());
    }
    if slot != TRADE_SLOT_NONTRADED && model.is_soulbound(req.item_guid) {
        return Err(ERR_TRADE_BOUND_ITEM.into());
    }
    Ok(())
}

/// Why an accepted exchange was refused; each side sees its own Retail text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExchangeRefusal {
    /// `character_id` no longer has an offered item.
    OfferGone(u64),
    /// An offered item became soulbound before settlement.
    BoundItem,
    /// `character_id` no longer has the offered money (TrinityCore
    /// `HandleAcceptTradeOpcode` `EQUIP_ERR_NOT_ENOUGH_MONEY`).
    NotEnoughMoney(u64),
    /// `character_id` has no room for what they receive.
    BagFull(u64),
    /// `character_id` would pass the gold cap.
    TooMuchGold(u64),
}

impl ExchangeRefusal {
    /// The text `viewer` sees.
    pub(super) fn message_for(self, viewer: u64) -> &'static str {
        match self {
            Self::OfferGone(_) => ERR_ITEM_NOT_FOUND,
            Self::BoundItem => ERR_TRADE_BOUND_ITEM,
            Self::NotEnoughMoney(who) if who == viewer => ERR_NOT_ENOUGH_MONEY,
            Self::NotEnoughMoney(_) => "Trade failed.",
            Self::BagFull(who) if who == viewer => "Trade failed, you don't have enough space.", // ERR_TRADE_BAG_FULL
            Self::BagFull(_) => "Trade failed, target doesn't have enough space.", // ERR_TRADE_TARGET_BAG_FULL
            Self::TooMuchGold(who) if who == viewer => "At gold limit", // ERR_TOO_MUCH_GOLD
            Self::TooMuchGold(_) => "Trade failed.",
        }
    }
}

/// AzerothCore `HandleAcceptTradeOpcode`: the money is checked first, then both
/// sides' items leave their bags, money moves, each side stores what it receives,
/// and both inventories are written in one transaction. Nothing changes when
/// anything is missing or does not fit.
pub(super) fn execute_completed_trade(
    db: &redb::Database,
    templates: &ItemTemplates,
    trade: &CompletedTrade,
    gold_updates: &mut GoldUpdateQueue,
) -> Result<(), ExchangeRefusal> {
    let load = |id| load_model(db, templates, id).map_err(|_| ExchangeRefusal::OfferGone(id));
    let mut initiator = load(trade.initiator)?;
    let mut target = load(trade.target)?;
    let initiator_gold = exchanged_gold(
        &initiator,
        trade.initiator,
        trade.initiator_offer.gold,
        trade.target_offer.gold,
    )?;
    let target_gold = exchanged_gold(
        &target,
        trade.target,
        trade.target_offer.gold,
        trade.initiator_offer.gold,
    )?;
    let mut mint = || persistence::next_id(db, "item_instance");
    let to_target = take_offered_items(
        &mut initiator,
        &trade.initiator_offer,
        trade.initiator,
        &mut mint,
    )?;
    let to_initiator =
        take_offered_items(&mut target, &trade.target_offer, trade.target, &mut mint)?;
    store_received(&mut initiator, to_initiator, templates, &mut mint)
        .ok_or(ExchangeRefusal::BagFull(trade.initiator))?;
    store_received(&mut target, to_target, templates, &mut mint)
        .ok_or(ExchangeRefusal::BagFull(trade.target))?;
    initiator.set_gold(initiator_gold);
    target.set_gold(target_gold);
    let txn = db.begin_write().expect("Failed to begin trade write");
    write_inventory_record(&txn, trade.initiator, &initiator.into_record());
    write_inventory_record(&txn, trade.target, &target.into_record());
    txn.commit().expect("Failed to commit trade");
    gold_updates.record(trade.initiator, initiator_gold);
    gold_updates.record(trade.target, target_gold);
    Ok(())
}

/// Gold after giving `given` and receiving `received`.
fn exchanged_gold(
    model: &InventoryModel,
    character_id: u64,
    given: u64,
    received: u64,
) -> Result<u64, ExchangeRefusal> {
    let left = model
        .gold()
        .checked_sub(given)
        .ok_or(ExchangeRefusal::NotEnoughMoney(character_id))?;
    left.checked_add(received)
        .filter(|gold| *gold <= MAX_MONEY)
        .ok_or(ExchangeRefusal::TooMuchGold(character_id))
}

fn take_offered_items(
    model: &mut InventoryModel,
    offer: &TradeOffer,
    character_id: u64,
    mint: &mut impl FnMut() -> u64,
) -> Result<Vec<(InventoryItemData, shared::item_data::ItemDefinitionSource)>, ExchangeRefusal> {
    offer
        .traded_items()
        .map(|item| {
            let owned = model
                .bag_item(item.item_guid)
                .ok_or(ExchangeRefusal::OfferGone(character_id))?;
            let source = model.definition_source(owned.item_guid);
            if model.is_soulbound(item.item_guid) {
                return Err(ExchangeRefusal::BoundItem);
            }
            model
                .take_bag_item(item.item_guid, u32::from(item.count), mint)
                .map(|taken| (taken, source))
                .map_err(|_| ExchangeRefusal::OfferGone(character_id))
        })
        .collect()
}

fn store_received(
    model: &mut InventoryModel,
    items: Vec<(InventoryItemData, shared::item_data::ItemDefinitionSource)>,
    templates: &ItemTemplates,
    mint: &mut impl FnMut() -> u64,
) -> Option<()> {
    for (item, source) in items {
        let template = *templates.get(source, item.item_id)?;
        model.store_item(item, source, template, mint).ok()?;
    }
    Some(())
}

pub(super) fn send_trade_completion(
    completed: &CompletedTrade,
    connections: &ConnectionQuery,
    players: &PlayerQuery,
    senders: &mut Query<&mut MessageSender<TradeStateUpdate>>,
) {
    for character_id in [completed.initiator, completed.target] {
        send_trade_message_to_other(
            character_id,
            character_id,
            connections,
            players,
            senders,
            Some(ERR_TRADE_COMPLETE.into()),
        );
    }
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

pub(super) fn position_tuple(position: &Position) -> (f32, f32, f32) {
    (position.x, position.y, position.z)
}

pub(super) const ERR_ITEM_NOT_FOUND: &str = "The item was not found.";
pub(super) const ERR_TRADE_BOUND_ITEM: &str = "You can't trade a soulbound item.";
pub(super) const ERR_TRADE_COMPLETE: &str = "Trade complete.";
pub(super) const ERR_TRADE_CANCELLED: &str = "Trade canceled.";
pub(super) const ERR_TRADE_TOO_FAR: &str = "Trade target is too far away.";
pub(super) const ERR_NOT_ENOUGH_MONEY: &str = "You don't have enough money.";

/// Retail GlobalStrings text for a trade refusal.
pub(super) fn trade_error_message(error: shared::trade::TradeError) -> &'static str {
    use shared::trade::TradeError;
    match error {
        TradeError::OutOfRange => ERR_TRADE_TOO_FAR,
        TradeError::CannotTradeWithSelf => "You can't trade with yourself.", // ERR_TRADE_SELF
        TradeError::AlreadyTrading => "You are already trading",             // ERR_ALREADY_TRADING
        TradeError::InvalidTarget => "No target or name specified.", // ERR_NO_TARGET_OR_NAME
        TradeError::PlayerDead => "You can't do that when you're dead.", // ERR_PLAYER_DEAD
        TradeError::PlayerInCombat => "You can't do that while in combat", // ERR_NOT_IN_COMBAT
        TradeError::ItemSoulbound => ERR_TRADE_BOUND_ITEM,
        TradeError::ItemAlreadyOffered | TradeError::InvalidSlot => ERR_ITEM_NOT_FOUND,
        TradeError::NoPendingRequest
        | TradeError::NoActiveSession
        | TradeError::TradeNotOpen
        | TradeError::NotConfirmed => ERR_TRADE_CANCELLED,
    }
}
