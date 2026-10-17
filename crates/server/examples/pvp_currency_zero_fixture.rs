//! Approved private5941 presentation fixture: discover Honor1792 with quantity0.
//! Never grants a reward, never edits an online database, never overwrites a balance.
use redb::{Database, ReadableTable, TableDefinition};
use shared::currency::{CurrencyEntry, CurrencyWallet, HONOR_CURRENCY_ID};
use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let (path, character) = read_private_target()?;
    let db = Database::open(path)?; // Exclusive redb lock refuses an online realm.
    preload_discovered_zero(&db, character)?;
    println!(
        "PRESENTATION FIXTURE ONLY: character{character}, discovered Honor1792 quantity0; no earned balance or award"
    );
    Ok(())
}

fn read_private_target() -> Result<(PathBuf, u64), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(args.next().ok_or("expected private redb path")?).canonicalize()?;
    let approved = PathBuf::from("/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/pvp-progression-2026-10-16/live/game.redb").canonicalize()?;
    if path != approved {
        return Err("only the approved private5941 fixture database is allowed".into());
    }
    let character: u64 = args
        .next()
        .ok_or("expected existing character ID")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    Ok((path, character))
}

fn preload_discovered_zero(db: &Database, character: u64) -> Result<(), Box<dyn Error>> {
    let transaction = db.begin_write()?;
    {
        let mut table = transaction.open_table(TableDefinition::<u64, &[u8]>::new("currencies"))?;
        let mut wallet: CurrencyWallet = {
            let saved = table
                .get(character)?
                .ok_or("authenticate and save the empty wallet first")?;
            serde_json::from_slice(saved.value())?
        };
        if wallet.entries.iter().any(|entry| {
            entry.id != HONOR_CURRENCY_ID || entry.amount != 0 || entry.week_earned != 0
        }) {
            return Err("refusing to change a nonempty, earned or unrelated wallet".into());
        }
        if wallet.entries.is_empty() {
            wallet.entries.push(CurrencyEntry {
                id: HONOR_CURRENCY_ID,
                amount: 0,
                week_earned: 0,
            });
        }
        let bytes = serde_json::to_vec(&wallet)?;
        table.insert(character, bytes.as_slice())?;
    }
    transaction.commit()?;
    Ok(())
}
