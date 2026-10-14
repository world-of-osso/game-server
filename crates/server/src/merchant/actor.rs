//! Merchant requests retain session order; only committed receipts publish effects.
use super::transactions::{Inputs, Operation, Publication};
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

pub(super) struct Request {
    pub connection: Entity,
    pub character: u64,
    pub player: Entity,
    pub vendor: Entity,
    pub operation: Operation,
}
struct PendingCommit {
    request: Request,
    receipt: CommitReceipt<Publication>,
}
#[derive(Resource, Default)]
pub(super) struct PendingMerchant {
    pub requests: VecDeque<Request>,
    commit: Option<PendingCommit>,
}

pub(super) fn drive_merchant_commits(
    mut pending: ResMut<PendingMerchant>,
    mut shop: Shop,
    mut sessions: Query<(&InWorld, &mut BuybackSlots)>,
) {
    if let Some(result) = pending
        .commit
        .as_ref()
        .and_then(|commit| commit.receipt.result())
    {
        let commit = pending.commit.take().expect("completed merchant commit");
        match result {
            Ok(publication) => publish(&mut shop, &mut sessions, &commit.request, publication),
            Err(error) => {
                error!(
                    "merchant commit character={}: {error}",
                    commit.request.character
                );
                shop.fail(
                    commit.request.connection,
                    commit.request.vendor.to_bits(),
                    MerchantError::ItemNotFound,
                );
            }
        }
    }
    if pending.commit.is_some() || !shop.ready() {
        return;
    }
    while let Some(request) = pending.requests.pop_front() {
        let Ok((session, buyback)) = sessions.get_mut(request.connection) else {
            continue;
        };
        if session.character_id != request.character {
            continue;
        }
        let inputs = match snapshot_inputs(&shop, &request) {
            Ok(inputs) => inputs,
            Err(error) => {
                shop.fail(request.connection, request.vendor.to_bits(), error);
                continue;
            }
        };
        let slots = buyback.clone();
        let (connection, character, vendor, operation) = (
            request.connection,
            request.character,
            request.vendor,
            request.operation.clone(),
        );
        let receipt =
            shop.db
                .as_ref()
                .expect("ready")
                .writer
                .enqueue("merchant transaction", move |db| {
                    transactions::execute(
                        db, inputs, slots, connection, character, vendor, operation,
                    )
                });
        pending.commit = Some(PendingCommit { request, receipt });
        break;
    }
}

fn snapshot_inputs(shop: &Shop, request: &Request) -> Result<Inputs, MerchantError> {
    let entry = shop.vendor_entry(request.vendor)?;
    let buyer = shop.buyer(request.player);
    Ok(Inputs {
        templates: shop.templates.as_ref().expect("ready").as_ref().clone(),
        catalog: shop.catalog.clone(),
        quests: shop.quests.as_deref().cloned(),
        business: shop
            .economy_config
            .as_ref()
            .and_then(|config| config.merchant(entry))
            .cloned(),
        stock: shop.stock.clone(),
        buyer,
        entry,
        can_repair: shop
            .npcs
            .get(request.vendor)
            .is_ok_and(|(_, flags)| flags.contains(NpcFlags::REPAIR)),
        reputation: shop.reputations.get(request.player).ok().cloned(),
        durability_rules: shop.durability_rules.clone(),
        durability: shop.durability.get(request.player).ok().cloned(),
        wallet: shop
            .currencies
            .get(request.player)
            .ok()
            .map(|wallet| wallet.0.clone()),
        reset: *shop.reset,
        now: shop.now(),
    })
}

fn publish(
    shop: &mut Shop,
    sessions: &mut Query<(&InWorld, &mut BuybackSlots)>,
    request: &Request,
    publication: Publication,
) {
    if let Some(value) = publication.gold {
        shop.gold.record(request.character, value);
    }
    for event in publication.criteria {
        shop.gold.record_criteria(request.character, event);
    }
    for (item, count) in publication.acquisitions {
        shop.gold.record_acquisition(request.character, item, count);
    }
    if let Some(stock) = publication.stock {
        *shop.stock = stock;
    }
    if let Some(state) = publication.economy {
        if let Some(economy) = shop.economy.as_deref_mut() {
            *economy = crate::economy::Economy::from_state(state);
        }
    }
    if let Some(guild) = publication.guild {
        shop.changed_guilds.0.push(guild);
    }
    let Ok((session, mut slots)) = sessions.get_mut(request.connection) else {
        return;
    };
    if session.character_id != request.character {
        return;
    }
    if let Some(durability) = publication.durability {
        if let Ok(mut live) = shop.durability.get_mut(request.player) {
            *live = durability;
        }
    }
    if let Some(wallet) = publication.wallet {
        if let Ok(mut live) = shop.currencies.get_mut(request.player) {
            live.0 = wallet.clone();
        }
        crate::currency::send_currency_update(
            request.connection,
            &wallet,
            None,
            None,
            &mut shop.currency_updates,
        );
    }
    if let Some(buyback) = publication.buyback {
        *slots = buyback;
    }
    if publication.refresh_buyback {
        shop.send_buyback(request.connection, &slots);
    }
    if publication.refresh_inventory {
        shop.send_inventory(
            request.connection,
            request.vendor,
            shop.buyer(request.player),
            request.character,
        );
    }
    if let Some(error) = publication.refusal {
        shop.fail(request.connection, request.vendor.to_bits(), error);
    }
}
