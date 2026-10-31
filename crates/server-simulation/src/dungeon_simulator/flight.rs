//! DAWNBREAKER_FLIGHT_ALTITUDE_TERRAIN_CLEARANCE_INFERRED.
use bevy::prelude::*;
use shared::los;

// Fifteen yards separates the inferred path from the selected collision surface.
const CLEARANCE: f32 = 15.0;
// FlightCapability 11 caps an impulse at 100 yd/s. Treating that as a vertical
// continuity bound is inferred; dives may exceed the impulse speed.
const VERTICAL_BOUND: f32 = shared::skyriding::SKYRIDING.add_impulse_max_speed;

#[derive(serde::Deserialize)]
struct Trace {
    report: String,
    fight: u32,
    actor: u32,
    legs: Vec<TraceLeg>,
}

#[derive(serde::Deserialize)]
struct TraceLeg {
    encounter: u32,
    points: Vec<(f32, f32, f32)>,
}

#[derive(Clone, Copy)]
struct LoggedPoint {
    horizontal: Vec2,
    seconds: f32,
}

/// Find baked solids in a column, including neighbouring-tile overhangs.
fn native_surfaces(los: &mut los::LosMap, point: Vec2) -> Result<Vec<f32>, String> {
    let (row, col) = los::tile_of(point.x, point.y);
    if los.tile((row, col))?.is_none() {
        return Err(format!("missing flight LOS tile ({row}, {col})"));
    }
    let mut bounds = None::<(f32, f32)>;
    for r in row.saturating_sub(1)..=(row + 1).min(los::MAP_TILES - 1) {
        for c in col.saturating_sub(1)..=(col + 1).min(los::MAP_TILES - 1) {
            if let Some((min, max)) = los.tile((r, c))?.and_then(|tile| tile.bounds()) {
                bounds =
                    Some(bounds.map_or((min.y, max.y), |(lo, hi)| (lo.min(min.y), hi.max(max.y))));
            }
        }
    }
    let Some((low, high)) = bounds else {
        return Ok(Vec::new());
    };
    let bottom = Vec3::new(point.x, low - 1.0, point.y);
    let mut top = Vec3::new(point.x, high + 1.0, point.y);
    let mut surfaces = los.floor_heights(point.x, point.y)?;
    while top.y > bottom.y {
        let Some(t) = los.first_hit(top, bottom)? else {
            break;
        };
        let height = top.lerp(bottom, t).y;
        surfaces.push(height);
        // Avoid hitting the same face again. Walkable floors are also enumerated above.
        top.y = height - 0.01;
    }
    surfaces.sort_by(f32::total_cmp);
    surfaces.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    Ok(surfaces)
}

/// Interpolate grounded anchors, clear lower native surfaces, and check every flight segment.
fn flight_path(
    ground_start: Vec3,
    ground_end: Vec3,
    samples: &[LoggedPoint],
    mut surfaces: impl FnMut(Vec2, f32) -> Result<Vec<f32>, String>,
    mut clear: impl FnMut(Vec3, Vec3) -> Result<bool, String>,
) -> Result<Vec<Vec3>, String> {
    if samples.len() < 2 {
        return Err("flight leg needs at least two logged samples".into());
    }
    let first = &samples[0];
    let last = samples.last().unwrap();
    if first.horizontal != Vec2::new(ground_start.x, ground_start.z)
        || last.horizontal != Vec2::new(ground_end.x, ground_end.z)
    {
        return Err("projected flight endpoints do not match logged samples".into());
    }
    let duration = last.seconds - first.seconds;
    if !duration.is_finite() || duration <= 0.0 {
        return Err("flight sample timestamps are not strictly increasing".into());
    }
    let mut path = vec![ground_start, ground_start + Vec3::Y * CLEARANCE];
    let mut previous_height = ground_start.y + CLEARANCE;
    for pair in samples.windows(2) {
        let seconds = pair[1].seconds - pair[0].seconds;
        if !seconds.is_finite() || seconds <= 0.0 {
            return Err("flight sample timestamps are not strictly increasing".into());
        }
        let point = pair[1].horizontal;
        let fraction = (pair[1].seconds - first.seconds) / duration;
        let baseline = ground_start.y + (ground_end.y - ground_start.y) * fraction;
        let candidates = surfaces(point, baseline)?;
        if candidates.iter().any(|height| !height.is_finite()) {
            return Err(format!(
                "invalid flight surface at {point:?}: {candidates:?}"
            ));
        }
        let below = candidates
            .into_iter()
            .filter(|height| *height <= baseline)
            .max_by(f32::total_cmp);
        let mut height = below.map_or(baseline, |surface| baseline.max(surface + CLEARANCE));
        if pair[1].seconds == last.seconds {
            height = height.max(ground_end.y + CLEARANCE);
        }
        let max_change = VERTICAL_BOUND * seconds;
        if (height - previous_height).abs() > max_change {
            return Err(format!(
                "flight altitude exceeds continuity bound at {point:?}: previous={previous_height}, next={height}, max_change={max_change}"
            ));
        }
        previous_height = height;
        path.push(Vec3::new(point.x, height, point.y));
    }
    path.push(ground_end);
    for (index, pair) in path.windows(2).enumerate() {
        if !clear(pair[0], pair[1])? {
            return Err(format!(
                "flight segment {index} blocked: {:?} -> {:?}",
                pair[0], pair[1]
            ));
        }
    }
    Ok(path)
}

fn project_endpoint(
    los: &mut los::LosMap,
    nav: &mut navigation::NavigationMap,
    point: Vec2,
    reference: f32,
) -> Result<Vec3, String> {
    let mut floors = Vec::new();
    if let Some(floor) = nav.floor_polygon(Vec3::new(point.x, reference, point.y))?
        && (floor.x - point.x).hypot(floor.z - point.y) < 0.001
    {
        floors.push(Vec3::new(point.x, floor.y, point.y));
    }
    for height in los.floor_heights(point.x, point.y)? {
        if let Some(floor) = nav.floor_polygon(Vec3::new(point.x, height, point.y))?
            && (floor.x - point.x).hypot(floor.z - point.y) < 0.001
        {
            floors.push(Vec3::new(point.x, floor.y, point.y));
        }
    }
    floors
        .into_iter()
        .min_by(|a, b| (a.y - reference).abs().total_cmp(&(b.y - reference).abs()))
        .ok_or_else(|| format!("no native projected floor at logged point {point:?}"))
}

/// Resolve the retained WCL trace against the map's real collision and navmesh.
pub(super) fn build_paths(
    graph: &super::graph::Graph,
    nav: &mut navigation::NavigationMap,
) -> Result<Vec<Vec<Vec3>>, String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/dungeons/dawnbreaker-flight-wcl.json");
    let trace: Trace = serde_json::from_slice(
        &std::fs::read(&path)
            .map_err(|error| format!("flight trace {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("flight trace {}: {error}", path.display()))?;
    if trace.report != "x3HzDRn6yVjqCbt8"
        || trace.fight != 7
        || trace.actor != 5
        || trace
            .legs
            .iter()
            .map(|leg| leg.encounter)
            .collect::<Vec<_>>()
            != [2837, 2838, 2839]
        || graph.follower_boss_entries.len() != trace.legs.len()
    {
        return Err("Dawnbreaker flight trace does not match encounter order".into());
    }
    let mut boss_heights = Vec::new();
    for entry in &graph.follower_boss_entries {
        let source = *graph
            .follower_boss_sources
            .get(entry)
            .ok_or_else(|| format!("flight boss {entry} lacks a source binding"))?;
        let matches: Vec<_> = graph
            .packs
            .iter()
            .filter(|pack| pack.spawns.iter().any(|spawn| spawn.entry == source))
            .collect();
        let [boss] = matches.as_slice() else {
            return Err(format!("flight boss source {source} is not unique"));
        };
        boss_heights.push(boss.height);
    }
    let root = super::graph::extraction::asset_root();
    let los = std::cell::RefCell::new(los::LosMap::new(&root.join("los/2662")));
    let nav = std::cell::RefCell::new(nav);
    let mut paths = Vec::new();
    let mut previous_boss_height = graph.entrance_height;
    for (leg, &boss_height) in trace.legs.iter().zip(&boss_heights) {
        let points: Vec<_> = leg
            .points
            .iter()
            .map(|&(ms, x, z)| LoggedPoint {
                horizontal: Vec2::new(x, z),
                seconds: ms / 1000.0,
            })
            .collect();
        let (Some(first), Some(last)) = (points.first(), points.last()) else {
            return Err(format!(
                "encounter {} has no logged flight samples",
                leg.encounter
            ));
        };
        let start = project_endpoint(
            &mut los.borrow_mut(),
            &mut nav.borrow_mut(),
            first.horizontal,
            previous_boss_height,
        )?;
        let end = project_endpoint(
            &mut los.borrow_mut(),
            &mut nav.borrow_mut(),
            last.horizontal,
            boss_height,
        )?;
        let path = flight_path(
            start,
            end,
            &points,
            |point, baseline| {
                let mut candidates = native_surfaces(&mut los.borrow_mut(), point)?;
                if let Some(floor) = nav
                    .borrow_mut()
                    .floor_polygon(Vec3::new(point.x, baseline, point.y))?
                    && (floor.x - point.x).hypot(floor.z - point.y) < 0.001
                {
                    candidates.push(floor.y);
                }
                Ok(candidates)
            },
            |from, to| los.borrow_mut().clear(from, to),
        )
        .map_err(|error| format!("encounter {} flight: {error}", leg.encounter))?;
        paths.push(path);
        previous_boss_height = boss_height;
    }
    Ok(paths)
}

#[test]
#[ignore = "requires baked Dawnbreaker LOS and navmesh"]
fn dawnbreaker_logged_endpoint_surfaces_are_reported() {
    let root = std::path::Path::new("/home/osso/Projects/world-of-osso/game-engine/data");
    let mut los = los::LosMap::new(&root.join("los/2662"));
    let mut nav = navigation::NavigationMap::open(&root.join("navmesh/2662")).unwrap();
    let points = [
        (1373.77, 178.47, 3.23963),
        (2352.87, -687.51, 1104.8579),
        (1877.25, -970.72, 124.9919),
        (2300.01, -636.16, 1104.7925),
    ];
    let mut missing = Vec::new();
    for (x, z, reference) in points {
        let heights = los.floor_heights(x, z).unwrap();
        let solids = native_surfaces(&mut los, Vec2::new(x, z)).unwrap();
        let floors: Vec<_> = heights
            .iter()
            .filter_map(|&y| nav.floor_polygon(Vec3::new(x, y, z)).unwrap())
            .collect();
        let projected = project_endpoint(&mut los, &mut nav, Vec2::new(x, z), reference);
        println!(
            "DAWNBREAKER_SURFACES x={x} z={z} solids={solids:?} walkable={heights:?} nav={floors:?} projected={projected:?}"
        );
        if projected.is_err() {
            missing.push((x, z));
        }
    }
    assert!(
        missing.is_empty(),
        "no projected floor at logged samples: {missing:?}"
    );
}

#[test]
fn inferred_flight_clears_lower_surfaces_and_checks_every_segment() {
    let start = Vec3::new(0.0, 2.0, 0.0);
    let end = Vec3::new(10.0, 4.0, 0.0);
    let samples = [
        LoggedPoint {
            horizontal: Vec2::ZERO,
            seconds: 0.0,
        },
        LoggedPoint {
            horizontal: Vec2::new(5.0, 0.0),
            seconds: 1.0,
        },
        LoggedPoint {
            horizontal: Vec2::new(10.0, 0.0),
            seconds: 2.0,
        },
    ];
    let mut checked = Vec::new();
    let path = flight_path(
        start,
        end,
        &samples,
        |point, _| Ok(vec![5000.0, if point.x == 5.0 { 3.0 } else { 4.0 }]),
        |a, b| {
            checked.push((a, b));
            Ok(true)
        },
    )
    .unwrap();
    assert_eq!(
        path,
        vec![
            start,
            Vec3::new(0.0, 17.0, 0.0),
            Vec3::new(5.0, 18.0, 0.0),
            Vec3::new(10.0, 19.0, 0.0),
            end
        ]
    );
    assert_eq!(
        checked,
        path.windows(2).map(|p| (p[0], p[1])).collect::<Vec<_>>()
    );
}

#[test]
fn flight_continuity_uses_the_logged_sample_interval() {
    let start = Vec3::ZERO;
    let end = Vec3::new(13.0, 90.0, 0.0);
    let samples = [
        LoggedPoint {
            horizontal: Vec2::ZERO,
            seconds: 0.0,
        },
        LoggedPoint {
            horizontal: Vec2::new(13.0, 0.0),
            seconds: 1.0,
        },
    ];
    let mut queried = Vec::new();
    let path = flight_path(
        start,
        end,
        &samples,
        |point, _| {
            queried.push(point);
            Ok(vec![90.0])
        },
        |_, _| Ok(true),
    )
    .unwrap();
    assert_eq!(queried, vec![Vec2::new(13.0, 0.0)]);
    assert_eq!(path[2], Vec3::new(13.0, 105.0, 0.0));
}

#[test]
fn inferred_flight_ignores_higher_layers_and_raises_for_lower_surfaces() {
    let start = Vec3::ZERO;
    let end = Vec3::new(10.0, 100.0, 0.0);
    let samples = [
        LoggedPoint {
            horizontal: Vec2::ZERO,
            seconds: 0.0,
        },
        LoggedPoint {
            horizontal: Vec2::new(5.0, 0.0),
            seconds: 1.0,
        },
        LoggedPoint {
            horizontal: Vec2::new(10.0, 0.0),
            seconds: 2.0,
        },
    ];
    let path = flight_path(
        start,
        end,
        &samples,
        |point, _| {
            Ok(if point.x == 5.0 {
                vec![45.0, 55.0, 5000.0]
            } else {
                vec![100.0]
            })
        },
        |_, _| Ok(true),
    )
    .unwrap();
    assert_eq!(path[2], Vec3::new(5.0, 60.0, 0.0));
    assert_eq!(path[3], Vec3::new(10.0, 115.0, 0.0));
}

#[test]
fn inferred_flight_rejects_invalid_surfaces_speed_and_blocked_segments() {
    let start = Vec3::ZERO;
    let end = Vec3::new(10.0, 0.0, 0.0);
    let samples = [
        LoggedPoint {
            horizontal: Vec2::ZERO,
            seconds: 0.0,
        },
        LoggedPoint {
            horizontal: Vec2::new(10.0, 0.0),
            seconds: 1.0,
        },
    ];
    let clear_sky = flight_path(start, end, &samples, |_, _| Ok(vec![]), |_, _| Ok(true)).unwrap();
    assert_eq!(clear_sky[2], Vec3::new(10.0, 15.0, 0.0));
    let invalid = flight_path(
        start,
        end,
        &samples,
        |_, _| Ok(vec![f32::NAN]),
        |_, _| Ok(true),
    )
    .unwrap_err();
    assert!(invalid.contains("invalid flight surface"));
    let high_landing = Vec3::new(end.x, 250.0, end.z);
    let impossible = flight_path(
        start,
        high_landing,
        &samples,
        |_, _| Ok(vec![0.0]),
        |_, _| Ok(true),
    )
    .unwrap_err();
    assert!(impossible.contains("continuity bound"));
    let blocked = flight_path(
        start,
        end,
        &samples,
        |_, _| Ok(vec![0.0]),
        |a, b| Ok(a.x == b.x),
    )
    .unwrap_err();
    assert!(blocked.contains("flight segment 1 blocked"));
}
