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

pub struct CurrencyPlugin;

impl Plugin for CurrencyPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<crate::achievement::CriteriaCredit>();
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
    mut currencies: Query<&mut PlayerCurrencies>,
    mut pvp_states: Query<&mut crate::pvp::PlayerPvpState>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<CurrencyStateUpdate>>,
    mut credits: MessageWriter<crate::achievement::CriteriaCredit>,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, in_world, mut receiver) in receivers.iter_mut() {
        for request in receiver.receive() {
            if let Some(credit) = handle_currency_request(
                CurrencyRequestTarget {
                    conn_entity,
                    player_entity: link.0,
                    character_id: in_world.character_id,
                },
                &request,
                &mut currencies,
                &mut pvp_states,
                &db,
                &mut senders,
            ) {
                credits.write(credit);
            }
        }
    }
}

struct CurrencyRequestTarget {
    conn_entity: Entity,
    player_entity: Entity,
    character_id: u64,
}

fn handle_currency_request<R: CurrencyRequest>(
    target: CurrencyRequestTarget,
    request: &R,
    currencies: &mut Query<&mut PlayerCurrencies>,
    pvp_states: &mut Query<&mut crate::pvp::PlayerPvpState>,
    db: &GameDatabase,
    senders: &mut Query<&mut MessageSender<CurrencyStateUpdate>>,
) -> Option<crate::achievement::CriteriaCredit> {
    let Ok(mut wallet) = currencies.get_mut(target.player_entity) else {
        send_currency_error(
            target.conn_entity,
            senders,
            "currencies unavailable for player",
        );
        return None;
    };
    let gained_currency = request.gained_currency();
    let previous = gained_currency.map(|id| wallet.0.get(id));
    match R::apply(&mut wallet.0, request) {
        Ok(message) => {
            persistence::save_currencies(&db.db, target.character_id, &wallet.0);
            if let Ok(mut pvp_state) = pvp_states.get_mut(target.player_entity)
                && crate::pvp::sync_progress_currencies_from_wallet(
                    &mut pvp_state.progress,
                    &wallet.0,
                )
            {
                persistence::save_pvp(&db.db, target.character_id, &pvp_state.progress);
            }
            send_currency_update(target.conn_entity, &wallet.0, Some(message), None, senders);
            let currency = gained_currency?;
            let amount = wallet.0.get(currency).saturating_sub(previous?);
            (amount != 0).then_some(crate::achievement::CriteriaCredit {
                player: target.player_entity,
                event: crate::achievement::CriteriaEvent::CurrencyGained(currency, amount),
            })
        }
        Err(error) => {
            send_currency_update(target.conn_entity, &wallet.0, None, Some(error), senders);
            None
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
