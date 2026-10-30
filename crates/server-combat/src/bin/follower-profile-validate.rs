//! Validate Warcraft Logs Mythic+ follower profile candidates read as JSON from stdin:
//! native item levels against the key's upgrade tracks and strict talent purchases.
//! Usage: follower-profile-validate <world.db> <db2 dir> <season time event> <map>
use std::io::Read;
use std::path::Path;

use server_combat::followers::gear::keystone::{Candidate, NativeProfileData};

fn parse_arg(args: &[String], index: usize, name: &str) -> Result<u32, String> {
    args.get(index)
        .ok_or(format!("missing {name}"))?
        .parse()
        .map_err(|error| format!("{name}: {error}"))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [world_db, db2, ..] = args.as_slice() else {
        return Err("usage: follower-profile-validate <world.db> <db2> <time event> <map>".into());
    };
    let season_time_event = parse_arg(&args, 2, "season time event")?;
    let map = parse_arg(&args, 3, "map")?;
    let data =
        NativeProfileData::read(Path::new(world_db), Path::new(db2), season_time_event, map)?;
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("read candidates: {error}"))?;
    let candidates: Vec<Candidate> =
        serde_json::from_str(&input).map_err(|error| format!("parse candidates: {error}"))?;
    let verdicts: Vec<_> = candidates
        .iter()
        .map(|candidate| data.validate(candidate))
        .collect();
    let output =
        serde_json::to_string(&verdicts).map_err(|error| format!("encode verdicts: {error}"))?;
    println!("{output}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
