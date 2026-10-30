//! DAWNBREAKER_FLIGHT_ALTITUDE_TERRAIN_CLEARANCE_INFERRED.
use bevy::prelude::*;

// Fifteen yards keeps the inferred flight corridor above sampled native geometry;
// LOS checks still reject segments that cross unsampled or vertical obstacles.
const CLEARANCE: f32 = 15.0;
const SAMPLE_SPACING: f32 = 5.0;

/// Highest baked solid in this column, including sloped geometry and neighbouring-tile overhangs.
fn native_surface(los: &mut los::LosMap, point: Vec2) -> Result<Option<f32>, String> {
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
        return Ok(None);
    };
    let top = Vec3::new(point.x, high + 1.0, point.y);
    let bottom = Vec3::new(point.x, low - 1.0, point.y);
    Ok(los.first_hit(top, bottom)?.map(|t| top.lerp(bottom, t).y))
}

/// Give logged horizontal waypoints inferred altitudes, preserving projected ground endpoints.
fn flight_path(
    ground_start: Vec3,
    ground_end: Vec3,
    samples: &[Vec2],
    mut surface: impl FnMut(Vec2) -> Result<Option<f32>, String>,
    mut clear: impl FnMut(Vec3, Vec3) -> Result<bool, String>,
) -> Result<Vec<Vec3>, String> {
    if samples.is_empty() {
        return Err("flight leg has no logged horizontal samples".into());
    }
    let horizontal: Vec<_> = std::iter::once(Vec2::new(ground_start.x, ground_start.z))
        .chain(samples.iter().copied())
        .chain(std::iter::once(Vec2::new(ground_end.x, ground_end.z)))
        .collect();
    let mut heights = Vec::with_capacity(horizontal.len() - 1);
    for pair in horizontal.windows(2) {
        let count = (pair[0].distance(pair[1]) / SAMPLE_SPACING).ceil().max(1.0) as usize;
        let mut highest = f32::NEG_INFINITY;
        for i in 0..=count {
            let point = pair[0].lerp(pair[1], i as f32 / count as f32);
            let height =
                surface(point)?.ok_or_else(|| format!("missing flight surface at {point:?}"))?;
            if !height.is_finite() {
                return Err(format!("invalid flight surface at {point:?}"));
            }
            highest = highest.max(height);
        }
        heights.push(highest + CLEARANCE);
    }
    let mut path = vec![ground_start];
    for (index, point) in horizontal.iter().enumerate() {
        let height = match (
            index.checked_sub(1).and_then(|i| heights.get(i)),
            heights.get(index),
        ) {
            (Some(a), Some(b)) => a.max(*b),
            (Some(a), None) | (None, Some(a)) => *a,
            (None, None) => unreachable!(),
        };
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
        let surface = native_surface(&mut los, Vec2::new(x, z)).unwrap();
        let floors: Vec<_> = heights
            .iter()
            .filter_map(|&y| nav.floor_polygon(Vec3::new(x, y, z)).unwrap())
            .collect();
        println!(
            "DAWNBREAKER_SURFACES x={x} z={z} highest={surface:?} los={heights:?} nav={floors:?}"
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
fn inferred_flight_uses_sampled_corridor_heights_and_checks_every_segment() {
    let start = Vec3::new(0.0, 2.0, 0.0);
    let end = Vec3::new(20.0, 3.0, 0.0);
    let mut checked = Vec::new();
    let path = flight_path(
        start,
        end,
        &[Vec2::new(10.0, 0.0)],
        |point| Ok(Some(if point.x == 5.0 { 30.0 } else { 10.0 })),
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
            Vec3::new(0.0, 45.0, 0.0),
            Vec3::new(10.0, 45.0, 0.0),
            Vec3::new(20.0, 25.0, 0.0),
            end
        ]
    );
    assert_eq!(
        checked,
        path.windows(2).map(|p| (p[0], p[1])).collect::<Vec<_>>()
    );
}

#[test]
fn inferred_flight_rejects_missing_surface_and_blocked_segments() {
    let start = Vec3::ZERO;
    let end = Vec3::new(10.0, 0.0, 0.0);
    let waypoints = [Vec2::new(5.0, 0.0)];
    let missing = flight_path(start, end, &waypoints, |_| Ok(None), |_, _| Ok(true)).unwrap_err();
    assert!(missing.contains("missing flight surface"));
    let blocked = flight_path(
        start,
        end,
        &waypoints,
        |_| Ok(Some(0.0)),
        |a, b| Ok(a.x == b.x),
    )
    .unwrap_err();
    assert!(blocked.contains("flight segment 1 blocked"));
}
