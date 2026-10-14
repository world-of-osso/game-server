use bevy::prelude::*;
use lightyear::prelude::*;
use shared::currency::{CurrencyWallet, currency_by_id};
use shared::protocol::{
    CurrencyChannel, CurrencyEntrySnapshot, CurrencySnapshot, CurrencyStateUpdate, EarnCurrency,
    SpendCurrency,
};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase};

#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct PlayerCurrencies(pub CurrencyWallet);

mod actor;
use actor::{PendingCurrencyCommits, publish_currency_commits};

pub struct CurrencyPlugin;

impl Plugin for CurrencyPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<crate::achievement::CriteriaCredit>();
        app.init_resource::<PendingCurrencyCommits>();
        app.add_systems(Update, publish_currency_commits);
        app.add_systems(Update, send_initial_currency_snapshot);
        app.add_systems(Update, process_currency_requests::<EarnCurrency>);
        app.add_systems(Update, process_currency_requests::<SpendCurrency>);
    }
}

type NewInWorldLink<'w, 's> =
    Query<'w, 's, (Entity, &'static PlayerLink), (With<Authenticated>, Added<InWorld>)>;

fn send_initial_currency_snapshot(
    connections: NewInWorldLink,
    currencies: Query<&PlayerCurrencies>,
    mut senders: Query<&mut MessageSender<CurrencyStateUpdate>>,
) {
    for (conn_entity, link) in &connections {
        let Ok(wallet) = currencies.get(link.0) else {
            send_currency_error(
                conn_entity,
                &mut senders,
                "currencies unavailable for player",
            );
            continue;
        };
        send_currency_update(
            conn_entity,
            &wallet.0,
            Some("currencies loaded".into()),
            None,
            &mut senders,
        );
    }
}

trait CurrencyRequest: lightyear::prelude::Message {
    fn gained_currency(&self) -> Option<u32>;
    fn apply(wallet: &mut CurrencyWallet, request: &Self) -> Result<String, String>;
}

impl CurrencyRequest for EarnCurrency {
    fn gained_currency(&self) -> Option<u32> {
        Some(self.currency_id)
    }

    fn apply(wallet: &mut CurrencyWallet, request: &Self) -> Result<String, String> {
        if matches!(
            request.currency_id,
            shared::currency::HONOR_CURRENCY_ID | shared::currency::CONQUEST_CURRENCY_ID
        ) {
            return Err(
                "PvP currency awards require a server match result and authored reward policy"
                    .into(),
            );
        }
        earn_currency(wallet, request.currency_id, request.amount)
    }
}

impl CurrencyRequest for SpendCurrency {
    fn gained_currency(&self) -> Option<u32> {
        None
    }

    fn apply(wallet: &mut CurrencyWallet, request: &Self) -> Result<String, String> {
        spend_currency(wallet, request.currency_id, request.amount)
    }
}

fn process_currency_requests<R: CurrencyRequest>(
    mut receivers: Query<
        (Entity, &PlayerLink, &InWorld, &mut MessageReceiver<R>),
        With<Authenticated>,
    >,
    currencies: Query<&PlayerCurrencies>,
    pvp_states: Query<&crate::pvp::PlayerPvpState>,
    db: Option<Res<GameDatabase>>,
    mut pending: ResMut<PendingCurrencyCommits>,
    mut senders: Query<&mut MessageSender<CurrencyStateUpdate>>,
) {
    let Some(db) = db else {
        return;
    };
    for (connection, link, session, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            if currencies.get(link.0).is_err() {
                send_currency_error(
                    connection,
                    &mut senders,
                    "currencies unavailable for player",
                );
                continue;
            }
            let character = session.character_id;
            let maintain_pvp = pvp_states.contains(link.0);
            let receipt = db.writer.enqueue("currency transaction", move |db| {
                actor::execute(db, character, maintain_pvp, request)
            });
            pending.push(connection, link.0, character, receipt);
        }
    }
}

pub(crate) fn earn_currency(
    wallet: &mut CurrencyWallet,
    currency_id: u32,
    amount: u32,
) -> Result<String, String> {
    if amount == 0 {
        return Err("currency amount must be greater than zero".into());
    }
    let Some(def) = currency_by_id(currency_id) else {
        return Err(format!("unknown currency id {}", currency_id));
    };
    let added = wallet
        .add(currency_id, amount, def)
        .map_err(|_| format!("unknown currency id {}", currency_id))?;
    Ok(format!("earned {} {}", added, def.name))
}

pub(crate) fn spend_currency(
    wallet: &mut CurrencyWallet,
    currency_id: u32,
    amount: u32,
) -> Result<String, String> {
    if amount == 0 {
        return Err("currency amount must be greater than zero".into());
    }
    let Some(def) = currency_by_id(currency_id) else {
        return Err(format!("unknown currency id {}", currency_id));
    };
    wallet
        .remove(currency_id, amount)
        .map_err(|_| format!("insufficient {}", def.name))?;
    Ok(format!("spent {} {}", amount, def.name))
}

pub(crate) fn build_currency_snapshot(wallet: &CurrencyWallet) -> CurrencySnapshot {
    let mut entries = wallet
        .entries
        .iter()
        .map(|entry| CurrencyEntrySnapshot {
            id: entry.id,
            name: currency_by_id(entry.id)
                .map(|def| def.name.to_string())
                .unwrap_or_else(|| format!("Currency {}", entry.id)),
            amount: u64::from(entry.amount),
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.id);
    CurrencySnapshot { entries }
}

pub(crate) fn send_currency_update(
    conn_entity: Entity,
    wallet: &CurrencyWallet,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<CurrencyStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<CurrencyChannel>(CurrencyStateUpdate {
        snapshot: Some(build_currency_snapshot(wallet)),
        message,
        error,
    });
}

fn send_currency_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<CurrencyStateUpdate>>,
    error: impl Into<String>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<CurrencyChannel>(CurrencyStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

#[cfg(test)]
#[path = "currency_tests.rs"]
mod tests;
