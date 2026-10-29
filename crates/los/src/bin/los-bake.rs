//! Bakes LOS tiles and the eye-height table from the local CASC extraction.
//!
//! los-bake --data ../game-engine/data --world-db data/world.db --out ../game-engine/data/los
//!          --map 0 (--tile ROW,COL ... | --around WOW_X,WOW_Y,RADIUS_TILES | --all)
//!          [--cutoff YARDS] [--no-eyes]
//! los-bake --out DIR --map 0 --probe AX,AY,AZ:BX,BY,BZ   (WoW coordinates)
//!
//! Writes `<out>/<map>/<row>_<col>.los`, `<out>/<map>/report.txt` and
//! `<out>/eye_heights.tsv` and `<out>/player_displays.tsv`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use los::bake::{Baker, DEFAULT_THICKNESS_CUTOFF, TileStats, eye};
use los::eye::{EYE_TABLE_FILE, EyeTable, PLAYER_DISPLAYS_FILE};
use los::{LosMap, LosTile, MAP_TILES, Vec3, tile_file_name, tile_of};

const RAY_SAMPLES: usize = 2000;
const RAY_LENGTH: f32 = 30.0;

#[derive(Default)]
struct Args {
    data: Option<PathBuf>,
    world_db: Option<PathBuf>,
    out: Option<PathBuf>,
    map: Option<u32>,
    tiles: Vec<(u32, u32)>,
    around: Vec<(f32, f32, u32)>,
    all: bool,
    cutoff: Option<f32>,
    no_eyes: bool,
    eyes_only: bool,
    m2_only: bool,
    probe: Option<(Vec3, Vec3)>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("los-bake: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let out = args.out.clone().ok_or("--out is required")?;
    if args.eyes_only {
        let data = args.data.as_ref().ok_or("--data is required")?;
        let world_db = args.world_db.as_ref().ok_or("--world-db is required")?;
        std::fs::create_dir_all(&out).map_err(|err| format!("{}: {err}", out.display()))?;
        let mut report = String::new();
        report_eyes(&mut report, data, world_db, &out)?;
        std::fs::write(out.join("eye_report.txt"), report).map_err(|err| err.to_string())?;
        return Ok(());
    }
    let map = args.map.ok_or("--map is required")?;
    if let Some((a, b)) = args.probe {
        return probe(&out.join(map.to_string()), a, b);
    }
    let data = args.data.clone().ok_or("--data is required")?;
    let world_db = args.world_db.clone().ok_or("--world-db is required")?;
    let mut baker = if args.m2_only {
        Baker::for_m2_floors(&data)
    } else {
        Baker::new(&data, args.cutoff.unwrap_or(DEFAULT_THICKNESS_CUTOFF))
    };
    let mut floors = Baker::for_m2_floors(&data);
    let wdt = map_wdt(&world_db, map)?;
    let map_dir = out.join(map.to_string());
    let map_dir = if args.m2_only {
        map_dir.join("m2")
    } else {
        map_dir
    };
    std::fs::create_dir_all(&map_dir).map_err(|err| format!("{}: {err}", map_dir.display()))?;
    let mut report = format!(
        "LOS bake of map {map} (WDT {wdt}), thickness cutoff {:.2} yd\n\n\
         tile\tterrain\twmo\tdoodad\ttriangles\tfile_bytes\tmemory_bytes\tray_us\talpha_blend_dropped\tthin_doodads\n",
        baker.cutoff
    );
    bake_requested_tiles(&mut baker, wdt, &args, |tile, baked, stats| {
        let started = Instant::now();
        let path = map_dir.join(tile_file_name(tile));
        let bytes = baked.to_bytes();
        std::fs::write(&path, &bytes).map_err(|err| format!("{}: {err}", path.display()))?;
        let loaded = LosTile::from_bytes(&std::fs::read(&path).map_err(|err| err.to_string())?)?;
        let ray_us = ray_time_us(&loaded);
        report_tile(&mut report, tile, &stats, &loaded, bytes.len(), ray_us);
        eprintln!(
            "tile {},{}: {} triangles in {:?}",
            tile.0,
            tile.1,
            loaded.triangle_count(),
            started.elapsed()
        );
        Ok(())
    })?;
    if !args.m2_only {
        let floor_dir = map_dir.join("m2");
        std::fs::create_dir_all(&floor_dir).map_err(|error| error.to_string())?;
        bake_requested_tiles(&mut floors, wdt, &args, |tile, baked, _| {
            std::fs::write(floor_dir.join(tile_file_name(tile)), baked.to_bytes())
                .map_err(|error| error.to_string())
        })?;
        for problem in &floors.problems {
            writeln!(report, "M2 floor asset gap: {problem}").unwrap();
        }
    }
    report_dropped(&mut report, &baker);
    if !args.no_eyes && !args.m2_only {
        report_eyes(&mut report, &data, &world_db, &out)?;
    }
    let report_path = map_dir.join("report.txt");
    std::fs::write(&report_path, &report)
        .map_err(|err| format!("{}: {err}", report_path.display()))?;
    eprintln!("report: {}", report_path.display());
    Ok(())
}

/// Global WMO doodad sets and continent MDDFs use the same collector.
fn bake_requested_tiles(
    baker: &mut Baker,
    wdt: u32,
    args: &Args,
    mut write_tile: impl FnMut((u32, u32), LosTile, TileStats) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(global) = baker.collect_global(wdt)? {
        for (tile, geometry) in global {
            let (vertices, triangles) = los::bake::index_triangles(&geometry.triangles);
            let baked = LosTile::build(tile.0, tile.1, vertices, triangles);
            write_tile(tile, baked, TileStats::default())?;
        }
        return Ok(());
    }
    let tiles = baker.map_tiles(wdt)?;
    for tile in wanted_tiles(args, &tiles) {
        if let Some((baked, stats)) = baker.bake_tile(&tiles, tile)? {
            write_tile(tile, baked, stats)?;
        }
    }
    Ok(())
}

fn report_eyes(
    report: &mut String,
    data: &Path,
    world_db: &Path,
    out: &Path,
) -> Result<(), String> {
    let (rows, stats) = eye::eye_rows(data, world_db)?;
    let write = |name: &str, text: String| {
        let path = out.join(name);
        std::fs::write(&path, text).map_err(|err| format!("{}: {err}", path.display()))
    };
    write(EYE_TABLE_FILE, EyeTable::to_tsv(&rows))?;
    let mut gaps = String::from("display_id\tcause\n");
    for (display, cause) in &stats.gaps {
        writeln!(gaps, "{display}\t{cause}").unwrap();
    }
    write("eye_gaps.tsv", gaps)?;
    write(
        PLAYER_DISPLAYS_FILE,
        EyeTable::players_to_tsv(&eye::player_displays(data)?),
    )?;
    writeln!(
        report,
        "\nEye points ({}): {} displays, {} rows; from eye geometry: {}, from the head bone: {}, \
         from the bounding box top: {}; (display, stand state) holding the Stand eye \
         (no sequence): {}",
        out.join(EYE_TABLE_FILE).display(),
        stats.displays,
        stats.rows,
        stats.from_eyes,
        stats.from_head,
        stats.from_bounds,
        stats.stand_pose,
    )
    .unwrap();
    writeln!(
        report,
        "Skipped template displays no spawn or player shows ({}):",
        stats.skipped.len()
    )
    .unwrap();
    for (display, why) in &stats.skipped {
        writeln!(report, "{display}\t{why}").unwrap();
    }
    Ok(())
}

fn report_tile(
    report: &mut String,
    (row, col): (u32, u32),
    stats: &TileStats,
    tile: &LosTile,
    file_bytes: usize,
    ray_us: f64,
) {
    writeln!(
        report,
        "{row},{col}\t{}\t{}\t{}\t{}\t{file_bytes}\t{}\t{ray_us:.2}\t{}\t{}",
        stats.terrain_triangles,
        stats.wmo_triangles,
        stats.doodad_triangles,
        tile.triangle_count(),
        tile.memory_bytes(),
        stats.alpha_or_blended_triangles,
        stats.thin_doodads,
    )
    .unwrap();
}

fn report_dropped(report: &mut String, baker: &Baker) {
    let dropped = baker.dropped_doodads();
    writeln!(report, "\nDoodad models dropped as thinner than {:.2} yd ({}):\nthickness_yd\tplacements\tfdid\tmodel", baker.cutoff, dropped.len()).unwrap();
    for doodad in dropped {
        writeln!(
            report,
            "{:.2}\t{}\t{}\t{}",
            doodad.thickness, doodad.placements, doodad.fdid, doodad.name
        )
        .unwrap();
    }
    writeln!(
        report,
        "\nMissing or unreadable files ({}):",
        baker.problems.len()
    )
    .unwrap();
    let mut problems = baker.problems.clone();
    problems.sort();
    problems.dedup();
    for problem in problems {
        writeln!(report, "{problem}").unwrap();
    }
}

/// Mean microseconds per `blocks` query of `RAY_LENGTH`-yard segments spread over the
/// tile's bounds (a fixed pseudo-random set).
fn ray_time_us(tile: &LosTile) -> f64 {
    let Some((min, max)) = tile.bounds() else {
        return 0.0;
    };
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 40) as f32 / (1u64 << 24) as f32
    };
    let segments: Vec<(Vec3, Vec3)> = (0..RAY_SAMPLES)
        .map(|_| {
            let start = min + (max - min) * Vec3::new(next(), next(), next());
            let dir =
                Vec3::new(next() - 0.5, (next() - 0.5) * 0.2, next() - 0.5).normalize_or(Vec3::X);
            (start, start + dir * RAY_LENGTH)
        })
        .collect();
    let started = Instant::now();
    let blocked = segments.iter().filter(|(a, b)| tile.blocks(*a, *b)).count();
    let elapsed = started.elapsed();
    std::hint::black_box(blocked);
    elapsed.as_secs_f64() * 1e6 / RAY_SAMPLES as f64
}

fn probe(map_dir: &Path, a: Vec3, b: Vec3) -> Result<(), String> {
    let mut map = LosMap::new(map_dir);
    let hit = map.first_hit(a, b)?;
    match hit {
        Some(t) => {
            let point = a + (b - a) * t;
            println!(
                "blocked at t={t:.4}: WoW ({:.2}, {:.2}, {:.2})",
                point.x, -point.z, point.y
            );
        }
        None => println!("clear"),
    }
    Ok(())
}

fn wanted_tiles(args: &Args, tiles: &[los::bake::adt::TileFiles]) -> Vec<(u32, u32)> {
    let mut wanted = args.tiles.clone();
    for &(wow_x, wow_y, radius) in &args.around {
        let (row, col) = tile_of(wow_x, -wow_y);
        for r in row.saturating_sub(radius)..=(row + radius).min(MAP_TILES - 1) {
            for c in col.saturating_sub(radius)..=(col + radius).min(MAP_TILES - 1) {
                wanted.push((r, c));
            }
        }
    }
    if args.all {
        for row in 0..MAP_TILES {
            for col in 0..MAP_TILES {
                if tiles[(col * MAP_TILES + row) as usize].root != 0 {
                    wanted.push((row, col));
                }
            }
        }
    }
    wanted.sort_unstable();
    wanted.dedup();
    wanted
}

fn map_wdt(world_db: &Path, map: u32) -> Result<u32, String> {
    let db =
        rusqlite::Connection::open_with_flags(world_db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|err| format!("{}: {err}", world_db.display()))?;
    db.query_row(
        "SELECT WdtFileDataID FROM map WHERE ID = ?1",
        [map],
        |row| row.get(0),
    )
    .map_err(|err| format!("map {map} WdtFileDataID: {err}"))
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args::default();
    let mut argv = std::env::args().skip(1);
    while let Some(flag) = argv.next() {
        let mut value = || argv.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--data" => args.data = Some(value()?.into()),
            "--world-db" => args.world_db = Some(value()?.into()),
            "--out" => args.out = Some(value()?.into()),
            "--map" => args.map = Some(number(&value()?)?),
            "--tile" => {
                let [row, col] = numbers::<2>(&value()?)?;
                args.tiles.push((row as u32, col as u32));
            }
            "--around" => {
                let [x, y, radius] = numbers::<3>(&value()?)?;
                args.around.push((x, y, radius as u32));
            }
            "--all" => args.all = true,
            "--cutoff" => args.cutoff = Some(number(&value()?)?),
            "--no-eyes" => args.no_eyes = true,
            "--eyes-only" => args.eyes_only = true,
            "--m2-only" => args.m2_only = true,
            "--probe" => {
                let raw = value()?;
                let (a, b) = raw.split_once(':').ok_or("--probe takes A:B")?;
                let wow = |text: &str| numbers::<3>(text).map(|[x, y, z]| Vec3::new(x, z, -y));
                args.probe = Some((wow(a)?, wow(b)?));
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(args)
}

fn number<T: std::str::FromStr>(text: &str) -> Result<T, String> {
    text.parse().map_err(|_| format!("bad number {text:?}"))
}

fn numbers<const N: usize>(text: &str) -> Result<[f32; N], String> {
    let values: Vec<f32> = text.split(',').map(number).collect::<Result<_, _>>()?;
    values
        .try_into()
        .map_err(|_| format!("{text:?} needs {N} comma-separated numbers"))
}
