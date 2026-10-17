//! Offline, private UDP5941 presentation fixture. Never awards currency.
use redb::{ReadableDatabase, TableDefinition};
use shared::currency::{CurrencyEntry, CurrencyWallet, HONOR_CURRENCY_ID};
use std::{error::Error, net::UdpSocket};

const PRIVATE_DB: &str = "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/pvp-progression-2026-10-16/live/game.redb";
const HONORPROOF: u64 = 30;
const CURRENCIES: TableDefinition<u64, &[u8]> = TableDefinition::new("currencies");

fn main() -> Result<(), Box<dyn Error>> {
    // A stale admin socket survives process exit; reserve the actual private realm port.
    let _private_port = UdpSocket::bind("127.0.0.1:5941")?;
    let mode = std::env::args()
        .nth(1)
        .ok_or("expected empty or discovered")?;
    let entries = match mode.as_str() {
        "empty" => vec![],
        "discovered" => vec![CurrencyEntry {
            id: HONOR_CURRENCY_ID,
            amount: 0,
            week_earned: 0,
        }],
        _ => return Err("expected empty or discovered".into()),
    };
    let wallet = CurrencyWallet { entries };
    let bytes = serde_json::to_vec(&wallet)?;
    let db = redb::Database::open(PRIVATE_DB)?;
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(CURRENCIES)?;
        table.insert(HONORPROOF, bytes.as_slice())?;
    }
    txn.commit()?;
    let txn = db.begin_read()?;
    let table = txn.open_table(CURRENCIES)?;
    let saved = table.get(HONORPROOF)?.ok_or("missing saved fixture")?;
    let actual: CurrencyWallet = serde_json::from_slice(saved.value())?;
    assert_eq!(actual, wallet);
    println!(
        "Presentation only: {PRIVATE_DB}, character{HONORPROOF}, {mode}: {}",
        String::from_utf8(bytes)?
    );
    Ok(())
}
