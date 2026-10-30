//! DAWNBREAKER_FLIGHT_ALTITUDE_TERRAIN_CLEARANCE_INFERRED.
use bevy::prelude::*;

// Fifteen yards separates the inferred path from the selected collision surface.
const CLEARANCE: f32 = 15.0;
const SAMPLE_SPACING: f32 = 5.0;
// FlightCapability 11 caps an impulse at 100 yd/s. Treating that as a vertical
// continuity bound is inferred; dives may exceed the impulse speed.
const VERTICAL_BOUND: f32 = shared::skyriding::SKYRIDING.add_impulse_max_speed;

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

/// Use logged horizontal positions and continuity-selected native layers, then check LOS.
fn flight_path(
    ground_start: Vec3,
    ground_end: Vec3,
    samples: &[LoggedPoint],
    mut surfaces: impl FnMut(Vec2) -> Result<Vec<f32>, String>,
    mut clear: impl FnMut(Vec3, Vec3) -> Result<bool, String>,
) -> Result<Vec<Vec3>, String> {
    let Some(first) = samples.first() else {
        return Err("flight leg has no logged samples".into());
    };
    let last = samples.last().unwrap();
    if first.horizontal != Vec2::new(ground_start.x, ground_start.z)
        || last.horizontal != Vec2::new(ground_end.x, ground_end.z)
    {
        return Err("projected flight endpoints do not match logged samples".into());
    }
    let mut path = vec![ground_start, ground_start + Vec3::Y * CLEARANCE];
    let mut previous_height = ground_start.y;
    for pair in samples.windows(2) {
        let seconds = pair[1].seconds - pair[0].seconds;
        if !seconds.is_finite() || seconds <= 0.0 {
            return Err("flight sample timestamps are not strictly increasing".into());
        }
        let count = (pair[0].horizontal.distance(pair[1].horizontal) / SAMPLE_SPACING)
            .ceil()
            .max(1.0) as usize;
        for i in 1..=count {
            let point = pair[0]
                .horizontal
                .lerp(pair[1].horizontal, i as f32 / count as f32);
            let max_change = VERTICAL_BOUND * seconds / count as f32;
            let height = surfaces(point)?
                .into_iter()
                .filter(|height| {
                    height.is_finite() && (height - previous_height).abs() <= max_change
                })
                .min_by(|a, b| {
                    (a - previous_height)
                        .abs()
                        .total_cmp(&(b - previous_height).abs())
                })
                .ok_or_else(|| format!("no flight surface within continuity bound at {point:?}"))?;
            previous_height = height;
            path.push(Vec3::new(point.x, height + CLEARANCE, point.y));
        }
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

#[test]
#[ignore = "requires baked Dawnbreaker LOS and navmesh"]
fn dawnbreaker_logged_endpoint_surfaces_are_reported() {
    let root = std::path::Path::new("/home/osso/Projects/world-of-osso/game-engine/data");
    let mut los = los::LosMap::new(&root.join("los/2662"));
    let mut nav = navigation::NavigationMap::open(&root.join("navmesh/2662")).unwrap();
    let points = [
        (1373.77, 178.47),
        (2352.87, -687.51),
        (1877.25, -970.72),
        (2300.01, -636.16),
    ];
    let mut missing = Vec::new();
    for (x, z) in points {
        let heights = los.floor_heights(x, z).unwrap();
        let solids = native_surfaces(&mut los, Vec2::new(x, z)).unwrap();
        let floors: Vec<_> = heights
            .iter()
            .filter_map(|&y| nav.floor_polygon(Vec3::new(x, y, z)).unwrap())
            .collect();
        println!(
            "DAWNBREAKER_SURFACES x={x} z={z} solids={solids:?} walkable={heights:?} nav={floors:?}"
        );
        if floors.is_empty() {
            missing.push((x, z));
        }
    }
    assert!(
        missing.is_empty(),
        "no projected floor at logged samples: {missing:?}"
    );
}

#[test]
fn inferred_flight_selects_continuous_layer_and_checks_every_segment() {
    let start = Vec3::new(0.0, 2.0, 0.0);
    let end = Vec3::new(10.0, 4.0, 0.0);
    let samples = [
        LoggedPoint {
            horizontal: Vec2::ZERO,
            seconds: 0.0,
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
        |point| Ok(vec![5000.0, if point.x == 5.0 { 3.0 } else { 4.0 }]),
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
fn inferred_flight_rejects_missing_layers_impossible_changes_and_blocked_segments() {
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
    let missing = flight_path(start, end, &samples, |_| Ok(vec![]), |_, _| Ok(true)).unwrap_err();
    assert!(missing.contains("continuity bound"));
    let impossible =
        flight_path(start, end, &samples, |_| Ok(vec![1000.0]), |_, _| Ok(true)).unwrap_err();
    assert!(impossible.contains("continuity bound"));
    let blocked = flight_path(
        start,
        end,
        &samples,
        |_| Ok(vec![0.0]),
        |a, b| Ok(a.x == b.x),
    )
    .unwrap_err();
    assert!(blocked.contains("flight segment 1 blocked"));
}
