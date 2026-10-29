#[path = "floor-resolver/anchors.rs"]
mod anchors;

use anchors::AnchorIndex;
use rusqlite::{Connection, OpenFlags};
use server_combat::floor_resolver::{FloorResolver, Hint, Request, Resolution};
use std::{collections::HashMap, env, error::Error, fs::File, path::Path};

#[cfg(test)]
#[path = "floor-resolver/continuity_tests.rs"]
mod continuity_tests;

type KnownKey = (String, String, u32, String, u32, u32);

fn insert_known(known: &mut HashMap<KnownKey, Option<f32>>, request: &Request, z: f32) {
    let key = (
        request.file.clone(),
        request.run_start.clone(),
        request.map,
        request.guid.clone(),
        request.x.to_bits(),
        request.y.to_bits(),
    );
    known
        .entry(key)
        .and_modify(|previous| {
            if previous.is_some_and(|height| (height - z).abs() > navigation::AGENT_CLIMB) {
                *previous = None;
            }
        })
        .or_insert(Some(z));
}

fn find_known_hint(
    known: &HashMap<KnownKey, Option<f32>>,
    request: &Request,
    hint: &Hint,
) -> Option<f32> {
    let file = if hint.file.is_empty() {
        &request.file
    } else {
        &hint.file
    };
    let run = if hint.run_start.is_empty() {
        &request.run_start
    } else {
        &hint.run_start
    };
    known
        .get(&(
            file.clone(),
            run.clone(),
            request.map,
            hint.guid.clone(),
            hint.x.to_bits(),
            hint.y.to_bits(),
        ))
        .and_then(|z| *z)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() < 6 || (args.len() - 6) % 2 != 0 {
        return Err(
            "usage: floor-resolver INPUT.json OUTPUT.json GEOMETRY_DIR NAVMESH_DIR WORLD_DB [PRIOR_INPUT.json PRIOR_OUTPUT.json]...".into(),
        );
    }
    let db = Connection::open_with_flags(&args[5], OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut query = db.prepare("SELECT ID, WdtFileDataID FROM map")?;
    let wdts: HashMap<u32, u32> = query
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let requests: Vec<Request> = serde_json::from_reader(File::open(&args[1])?)?;
    let anchor_index = AnchorIndex::read(&db, &requests)?;
    let mut prior = Vec::new();
    for pair in args[6..].chunks_exact(2) {
        let inputs: Vec<Request> = serde_json::from_reader(File::open(&pair[0])?)?;
        let outputs: Vec<serde_json::Value> = serde_json::from_reader(File::open(&pair[1])?)?;
        if inputs.len() != outputs.len() {
            return Err(format!("{} and {} have different row counts", pair[0], pair[1]).into());
        }
        prior.extend(
            inputs
                .into_iter()
                .zip(outputs)
                .filter_map(|(request, result)| {
                    result
                        .get("z")
                        .and_then(|z| z.as_f64())
                        .map(|z| (request, z as f32))
                }),
        );
    }
    let mut resolver = FloorResolver::new(Path::new(&args[3]), Path::new(&args[4]), wdts);
    let mut output: Vec<_> = requests
        .iter()
        .map(|request| resolver.resolve_with_hints(request, &[], &anchor_index.near(request)))
        .collect();
    loop {
        let mut known = HashMap::new();
        for (request, z) in &prior {
            insert_known(&mut known, request, *z);
        }
        for (request, result) in requests.iter().zip(&output) {
            if let Resolution::Resolved { z, .. } = result {
                insert_known(&mut known, request, *z);
            }
        }
        let mut changed = false;
        for (request, result) in requests.iter().zip(output.iter_mut()) {
            if !matches!(result, Resolution::Gap { .. }) {
                continue;
            }
            let anchors: Vec<_> = request
                .neighbor_hints
                .iter()
                .filter_map(|hint| {
                    find_known_hint(&known, request, hint)
                        .map(|z| (hint.x, hint.y, z, hint.guid.clone()))
                })
                .collect();
            if anchors.is_empty() {
                continue;
            }
            let next = resolver.resolve_with_hints(request, &anchors, &anchor_index.near(request));
            if matches!(next, Resolution::Resolved { .. }) {
                *result = next;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let output_path = Path::new(&args[2]);
    let temporary_path = output_path.with_extension("json.tmp");
    serde_json::to_writer(File::create(&temporary_path)?, &output)?;
    std::fs::rename(temporary_path, output_path)?;
    Ok(())
}
