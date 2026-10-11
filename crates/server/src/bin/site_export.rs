//! Offline static-site snapshots. No server initialization or sockets.
use server::character_data;
#[path = "../site_snapshot.rs"]
mod site_snapshot;

fn main() {
    if let Err(error) = site_snapshot::run(std::env::args().skip(1).collect()) {
        eprintln!("site export: {error}");
        std::process::exit(1);
    }
}
