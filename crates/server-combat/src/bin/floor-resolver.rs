use rusqlite::{Connection, OpenFlags};
use server_combat::floor_resolver::{FloorResolver, Request};
use std::{collections::HashMap, env, error::Error, fs::File, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 6 {
        return Err(
            "usage: floor-resolver INPUT.json OUTPUT.json GEOMETRY_DIR NAVMESH_DIR WORLD_DB".into(),
        );
    }
    let db = Connection::open_with_flags(&args[5], OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut query = db.prepare("SELECT ID, WdtFileDataID FROM map")?;
    let wdts: HashMap<u32, u32> = query
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let requests: Vec<Request> = serde_json::from_reader(File::open(&args[1])?)?;
    let mut resolver = FloorResolver::new(Path::new(&args[3]), Path::new(&args[4]), wdts);
    let output: Vec<_> = requests
        .iter()
        .map(|request| resolver.resolve(request))
        .collect();
    serde_json::to_writer(File::create(&args[2])?, &output)?;
    Ok(())
}
