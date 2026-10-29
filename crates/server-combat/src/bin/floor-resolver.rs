use rusqlite::{Connection, OpenFlags};
use server_combat::floor_resolver::{FloorResolver, Request, Resolution};
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
    let mut output: Vec<_> = requests
        .iter()
        .map(|request| resolver.resolve(request))
        .collect();
    loop {
        let known: HashMap<_, _> = requests
            .iter()
            .zip(&output)
            .filter_map(|(request, result)| {
                if let Resolution::Resolved { z, .. } = result {
                    Some((
                        (
                            request.file.as_str(),
                            request.run_start.as_str(),
                            request.map,
                            request.guid.as_str(),
                            request.x.to_bits(),
                            request.y.to_bits(),
                        ),
                        *z,
                    ))
                } else {
                    None
                }
            })
            .collect();
        let mut changed = false;
        for (request, result) in requests.iter().zip(output.iter_mut()) {
            if !matches!(result, Resolution::Gap { .. }) {
                continue;
            }
            let anchors: Vec<_> = request
                .neighbor_hints
                .iter()
                .filter_map(|hint| {
                    known
                        .get(&(
                            request.file.as_str(),
                            request.run_start.as_str(),
                            request.map,
                            hint.guid.as_str(),
                            hint.x.to_bits(),
                            hint.y.to_bits(),
                        ))
                        .map(|&z| (hint.x, hint.y, z))
                })
                .collect();
            if anchors.is_empty() {
                continue;
            }
            let next = resolver.resolve_with_known_hints(request, &anchors);
            if matches!(next, Resolution::Resolved { .. }) {
                *result = next;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    serde_json::to_writer(File::create(&args[2])?, &output)?;
    Ok(())
}
