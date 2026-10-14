//! Currency requests read fresh FIFO wallets and publish only durable replies.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

#[derive(Clone)]
pub(super) struct Publication {
    message: Option<String>,
    error: Option<String>,
    gained: Option<(u32, u32)>,
}
struct PendingCommit {
    connection: Entity,
    player: Entity,
    character: u64,
    receipt: CommitReceipt<Publication>,
}
#[derive(Resource, Default)]
pub(super) struct PendingCurrencyCommits(VecDeque<PendingCommit>);
impl PendingCurrencyCommits {
    pub(super) fn push(
        &mut self,
        connection: Entity,
        player: Entity,
        character: u64,
        receipt: CommitReceipt<Publication>,
    ) {
        self.0.push_back(PendingCommit {
            connection,
            player,
            character,
            receipt,
        });
    }
}

pub(super) fn execute<R: CurrencyRequest>(
    db: &redb::Database,
    character: u64,
    maintain_pvp: bool,
    request: R,
) -> Result<Publication, String> {
    let mut wallet = persistence::load_currencies(db, character);
    let gained_currency = request.gained_currency();
    let previous = gained_currency.map(|id| wallet.get(id));
    let message = match R::apply(&mut wallet, &request) {
        Ok(message) => message,
        Err(error) => {
            return Ok(Publication {
                message: None,
                error: Some(error),
                gained: None,
            });
        }
    };
    persist_wallet_and_progress(db, character, &wallet, maintain_pvp)?;
    let gained = gained_currency.zip(previous).and_then(|(id, previous)| {
        let amount = wallet.get(id).saturating_sub(previous);
        (amount != 0).then_some((id, amount))
    });
    Ok(Publication {
        message: Some(message),
        error: None,
        gained,
    })
}

fn persist_wallet_and_progress(
    db: &redb::Database,
    character: u64,
    wallet: &CurrencyWallet,
    maintain_pvp: bool,
) -> Result<(), String> {
    let bytes =
        serde_json::to_vec(wallet).map_err(|error| format!("encode currency wallet: {error}"))?;
    let pvp = if maintain_pvp {
        let mut progress = persistence::load_pvp(db, character);
        crate::pvp::sync_progress_currencies_from_wallet(&mut progress, wallet).then_some(progress)
    } else {
        None
    };
    let txn = db
        .begin_write()
        .map_err(|error| format!("begin currency transaction: {error}"))?;
    txn.open_table(persistence::CURRENCIES)
        .map_err(|error| error.to_string())?
        .insert(character, bytes.as_slice())
        .map_err(|error| error.to_string())?;
    if let Some(progress) = pvp {
        let bytes = serde_json::to_vec(&progress)
            .map_err(|error| format!("encode currency PvP progress: {error}"))?;
        txn.open_table(persistence::PVP_STATES)
            .map_err(|error| error.to_string())?
            .insert(character, bytes.as_slice())
            .map_err(|error| error.to_string())?;
    }
    txn.commit()
        .map_err(|error| format!("commit currency transaction: {error}"))
}

pub(super) fn publish_currency_commits(
    mut pending: ResMut<PendingCurrencyCommits>,
    db: Option<Res<GameDatabase>>,
    sessions: Query<(&InWorld, &PlayerLink), With<Authenticated>>,
    mut currencies: Query<&mut PlayerCurrencies>,
    mut pvp_states: Query<&mut crate::pvp::PlayerPvpState>,
    mut senders: Query<&mut MessageSender<CurrencyStateUpdate>>,
    mut credits: MessageWriter<crate::achievement::CriteriaCredit>,
) {
    let Some(db) = db else {
        return;
    };
    while let Some(result) = pending.0.front().and_then(|commit| commit.receipt.result()) {
        let commit = pending.0.pop_front().expect("completed currency receipt");
        let same_player = sessions
            .get(commit.connection)
            .is_ok_and(|(session, link)| {
                session.character_id == commit.character && link.0 == commit.player
            });
        if !same_player {
            continue;
        }
        let publication = match result {
            Ok(publication) => publication,
            Err(error) => {
                send_currency_error(commit.connection, &mut senders, error);
                continue;
            }
        };
        // Other committed actors may have changed the wallet before this tick polls.
        let wallet = persistence::load_currencies(&db.db, commit.character);
        if let Ok(mut live) = currencies.get_mut(commit.player) {
            live.0 = wallet.clone();
        }
        if let Ok(mut state) = pvp_states.get_mut(commit.player) {
            crate::pvp::sync_progress_currencies_from_wallet(&mut state.progress, &wallet);
        }
        send_currency_update(
            commit.connection,
            &wallet,
            publication.message,
            publication.error,
            &mut senders,
        );
        if let Some((currency, amount)) = publication.gained {
            credits.write(crate::achievement::CriteriaCredit {
                player: commit.player,
                event: crate::achievement::CriteriaEvent::CurrencyGained(currency, amount),
            });
        }
    }
}
