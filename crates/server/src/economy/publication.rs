use super::{Config, Economy, cycle, ids, store};
use crate::{
    auction_house::AuctionHouseState, gold::GoldUpdateQueue, persistence::GameDatabase,
    persistence_writer::CommitReceipt,
};
use bevy::log::warn;

pub(super) enum PendingCycle {
    Reservation {
        cycle: u64,
        receipt: CommitReceipt<ids::ReservedIds>,
    },
    Commit {
        cycle: u64,
        plan: cycle::PreparedCycle,
        receipt: CommitReceipt,
    },
}

pub(super) fn advance_cycle(
    db: &GameDatabase,
    config: &Config,
    economy: &mut Economy,
    auctions: &mut AuctionHouseState,
    gold: &mut GoldUpdateQueue,
    now: u64,
) {
    if let Some(pending) = economy.pending.take() {
        poll_cycle(db, config, economy, auctions, gold, pending);
        return;
    }
    if economy
        .last_attempt
        .is_some_and(|last| now.saturating_sub(last) < config.cycle_secs)
    {
        return;
    }
    economy.last_attempt = Some(now);
    match ids::reservation_counts(config, &economy.state) {
        Ok(counts) => {
            let receipt = db
                .writer
                .enqueue_result("economy ID reservation", move |db| ids::reserve(db, counts));
            economy.pending = Some(PendingCycle::Reservation {
                cycle: now,
                receipt,
            });
        }
        Err(reason) => warn!(cycle = now, %reason, "economy cycle aborted"),
    }
}

fn poll_cycle(
    db: &GameDatabase,
    config: &Config,
    economy: &mut Economy,
    auctions: &mut AuctionHouseState,
    gold: &mut GoldUpdateQueue,
    pending: PendingCycle,
) {
    match pending {
        PendingCycle::Reservation { cycle, receipt } => {
            let Some(result) = receipt.result() else {
                economy.pending = Some(PendingCycle::Reservation { cycle, receipt });
                return;
            };
            match result {
                Ok(mut ids) => queue_market_cycle(db, config, economy, auctions, cycle, &mut ids),
                Err(reason) => warn!(cycle, %reason, "economy cycle aborted"),
            }
        }
        commit @ PendingCycle::Commit { .. } => {
            economy.pending = Some(commit);
            publish_ready(db, economy, auctions, gold);
        }
    }
}

pub(super) fn publish_ready(
    db: &GameDatabase,
    economy: &mut Economy,
    auctions: &mut AuctionHouseState,
    gold: &mut GoldUpdateQueue,
) {
    if !economy.market_commit_pending() {
        return;
    }
    let Some(PendingCycle::Commit {
        cycle,
        plan,
        receipt,
    }) = economy.pending.take()
    else {
        unreachable!()
    };
    let Some(result) = receipt.result() else {
        economy.pending = Some(PendingCycle::Commit {
            cycle,
            plan,
            receipt,
        });
        return;
    };
    match result {
        Ok(()) => {
            for id in plan.publish(auctions, &mut economy.state) {
                crate::gold::record_persisted_gold(&db.db, id, gold);
            }
        }
        Err(reason) => warn!(cycle, %reason, "economy cycle aborted"),
    }
}

fn queue_market_cycle(
    db: &GameDatabase,
    config: &Config,
    economy: &mut Economy,
    auctions: &AuctionHouseState,
    cycle: u64,
    ids: &mut ids::ReservedIds,
) {
    let mut storage = cycle::CycleStorage { db: &db.db, ids };
    let plan = cycle::load_and_prepare_cycle(
        &mut storage,
        auctions,
        &economy.state,
        config,
        cycle,
        &mut rand::random::<u64>,
    );
    match plan {
        Ok(plan) => {
            let durable = plan.clone();
            let receipt = db.writer.enqueue("economy market cycle", move |db| {
                store::commit_market(db, Some(&durable.state), &durable.changes)
            });
            economy.pending = Some(PendingCycle::Commit {
                cycle,
                plan,
                receipt,
            });
        }
        Err(reason) => warn!(cycle, %reason, "economy cycle aborted"),
    }
}
