//! Reproducible local-CASC bake; selection comes from the existing LOS tile directory.
use los::bake::{Baker, DEFAULT_THICKNESS_CUTOFF, index_triangles};
use navigation::bake::{VERTEX_CAPACITY_ERROR, bake_collected_on_grid};
use navigation::{SUBDIVISIONS, TILE_CELLS};
use std::path::{Path, PathBuf};
use std::time::Instant;

struct Args {
    data: PathBuf,
    world_db: PathBuf,
    out: PathBuf,
    map: u32,
    los_tiles: Option<PathBuf>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("nav-bake: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let mut subdivisions = SUBDIVISIONS;
    loop {
        match run_on_grid(&args, subdivisions) {
            Err(error) if error.contains(VERTEX_CAPACITY_ERROR) && subdivisions < TILE_CELLS => {
                subdivisions *= 2;
                eprintln!("{error}; refining whole map to {subdivisions}x{subdivisions}");
            }
            result => return result,
        }
    }
}

fn run_on_grid(args: &Args, subdivisions: u32) -> Result<(), String> {
    let db = rusqlite::Connection::open_with_flags(
        &args.world_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|err| err.to_string())?;
    let wdt: u32 = db
        .query_row(
            "SELECT WdtFileDataID FROM map WHERE ID=?1",
            [args.map],
            |row| row.get(0),
        )
        .map_err(|err| err.to_string())?;
    let mut baker = Baker::for_navigation(&args.data);
    let dir = args.out.join("navmesh").join(args.map.to_string());
    std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let mut report = String::from("tile\ttriangles\tbytes\tmilliseconds\n");
    if let Some(global) = baker.collect_global(wdt)? {
        let all = los::bake::CollectedGeometry {
            triangles: global
                .iter()
                .flat_map(|(_, g)| g.triangles.iter().copied())
                .collect(),
            obstacles: global
                .iter()
                .flat_map(|(_, g)| g.obstacles.iter().copied())
                .collect(),
        };
        for (tile, _) in &global {
            write_tile(&dir, *tile, &all, &[], subdivisions, &mut report)?;
        }
        // Global-WMO support is shared with the LOS collector. The test oracle remains
        // the existing LosMap runtime, not a navigation-specific raycaster.
        bake_global_los(&args, wdt)?;
    } else {
        let tiles = baker.map_tiles(wdt)?;
        let selected = select_los_tiles(
            args.los_tiles
                .as_deref()
                .ok_or("continent bake requires --los-tiles <existing LOS map directory>")?,
        )?;
        for tile in selected {
            let (triangles, _) = baker
                .collect_tile(&tiles, tile)?
                .ok_or_else(|| format!("LOS tile {tile:?} has no terrain"))?;
            let water = baker.collect_water(tiles[(tile.1 * 64 + tile.0) as usize])?;
            write_tile(&dir, tile, &triangles, &water, subdivisions, &mut report)?;
        }
    }
    report.push_str(&format!("\nSubdivisions per axis: {subdivisions}\n"));
    baker.problems.sort();
    baker.problems.dedup();
    report.push_str(&format!(
        "\nMissing or unreadable files ({}):\n",
        baker.problems.len()
    ));
    for problem in &baker.problems {
        report.push_str(problem);
        report.push('\n');
    }
    std::fs::write(dir.join("report.txt"), report).map_err(|err| err.to_string())?;
    if !baker.problems.is_empty() {
        return Err(format!(
            "{} geometry gaps; see {}/report.txt",
            baker.problems.len(),
            dir.display()
        ));
    }
    Ok(())
}

fn write_tile(
    dir: &Path,
    tile: (u32, u32),
    geometry: &los::bake::CollectedGeometry,
    water: &[(glam::Vec3, glam::Vec3)],
    subdivisions: u32,
    report: &mut String,
) -> Result<(), String> {
    let started = Instant::now();
    let bytes = bake_collected_on_grid(geometry, water, tile, subdivisions)?;
    // Write even empty tiles: an authored unnavigable tile is not an unbaked map.
    std::fs::write(dir.join(format!("{}_{}.nav", tile.0, tile.1)), &bytes)
        .map_err(|err| err.to_string())?;
    let line = format!(
        "{},{}\t{}\t{}\t{}\n",
        tile.0,
        tile.1,
        geometry.triangles.len(),
        bytes.len(),
        started.elapsed().as_millis()
    );
    eprint!("{line}");
    report.push_str(&line);
    Ok(())
}

fn bake_global_los(args: &Args, wdt: u32) -> Result<(), String> {
    let mut baker = Baker::new(&args.data, DEFAULT_THICKNESS_CUTOFF);
    let geometry = baker.collect_global(wdt)?.ok_or("not a global WMO")?;
    let dir = args.out.join("los").join(args.map.to_string());
    std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    for (tile, triangles) in geometry {
        let (vertices, indexed) = index_triangles(&triangles.triangles);
        let bytes = los::LosTile::build(tile.0, tile.1, vertices, indexed).to_bytes();
        std::fs::write(dir.join(format!("{}_{}.los", tile.0, tile.1)), bytes)
            .map_err(|err| err.to_string())?;
    }
    if !baker.problems.is_empty() {
        return Err(format!("global LOS geometry gaps: {:?}", baker.problems));
    }
    Ok(())
}

fn select_los_tiles(dir: &Path) -> Result<Vec<(u32, u32)>, String> {
    let mut selected = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|err| format!("{}: {err}", dir.display()))? {
        let path = entry.map_err(|err| err.to_string())?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("los") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("invalid LOS filename")?;
        let (row, col) = stem.split_once('_').ok_or("invalid LOS tile filename")?;
        let tile = (
            row.parse().map_err(|_| "invalid row")?,
            col.parse().map_err(|_| "invalid column")?,
        );
        if tile.0 >= 64 || tile.1 >= 64 {
            return Err("LOS tile outside map".into());
        }
        selected.push(tile);
    }
    selected.sort_unstable();
    selected.dedup();
    if selected.is_empty() {
        return Err(format!("{} contains no LOS tiles", dir.display()));
    }
    Ok(selected)
}

fn parse_args() -> Result<Args, String> {
    let mut values = std::collections::HashMap::new();
    let mut argv = std::env::args().skip(1);
    while let Some(flag) = argv.next() {
        if !["--data", "--world-db", "--out", "--map", "--los-tiles"].contains(&flag.as_str()) {
            return Err(format!("unknown argument {flag}"));
        }
        values.insert(
            flag.clone(),
            argv.next().ok_or_else(|| format!("{flag} needs a value"))?,
        );
    }
    let mut required = |name: &str| {
        values
            .remove(name)
            .ok_or_else(|| format!("{name} is required"))
    };
    Ok(Args {
        data: required("--data")?.into(),
        world_db: required("--world-db")?.into(),
        out: required("--out")?.into(),
        map: required("--map")?.parse().map_err(|_| "invalid map")?,
        los_tiles: values.remove("--los-tiles").map(Into::into),
    })
}
