//! DAWNBREAKER_FLIGHT_ALTITUDE_TERRAIN_CLEARANCE_INFERRED.
use bevy::prelude::*;
use shared::los;

// Fifteen yards separates the inferred path from the selected collision surface.
const CLEARANCE: f32 = 15.0;
const MAX_LOS_REPAIRS: usize = 8;
const MAX_DESCENT_HOLD_SECONDS: f32 = super::campaign::STALL_SECONDS as f32;
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

#[derive(Clone, Copy, Debug)]
struct LoggedPoint {
    horizontal: Vec2,
    seconds: f32,
}

#[derive(Clone, Copy)]
struct FlightPoint {
    position: Vec3,
    seconds: f32,
}

/// Split at no-fly terrain cells; only logged points become takeoffs or landings.
fn permitted_flight_windows(
    samples: &[LoggedPoint],
    mut permits: impl FnMut(Vec2) -> Result<bool, String>,
) -> Result<Vec<Vec<LoggedPoint>>, String> {
    let mut windows = Vec::new();
    let mut current: Vec<LoggedPoint> = Vec::new();
    for &sample in samples {
        let allowed = permits(sample.horizontal)?;
        if let Some(previous) = current.last().copied()
            && (!allowed
                || !segment_permitted(previous.horizontal, sample.horizontal, &mut permits)?)
        {
            if current.len() >= 2 {
                windows.push(std::mem::take(&mut current));
            } else {
                current.clear();
            }
        }
        if allowed {
            current.push(sample);
        }
    }
    if current.len() >= 2 {
        windows.push(current);
    }
    Ok(windows)
}

/// Check every crossed MCNK cell, not only the two logged endpoints.
fn segment_permitted(
    from: Vec2,
    to: Vec2,
    permits: &mut impl FnMut(Vec2) -> Result<bool, String>,
) -> Result<bool, String> {
    let size = shared::navmesh::TILE_SIZE / 16.0;
    let mut boundaries = vec![0.0_f32, 1.0];
    for (start, end) in [(from.x, to.x), (from.y, to.y)] {
        if start == end {
            continue;
        }
        let first = (start.min(end) / size).floor() as i32 + 1;
        let last = (start.max(end) / size).floor() as i32;
        for cell in first..=last {
            let t = (cell as f32 * size - start) / (end - start);
            if t > 0.0 && t < 1.0 {
                boundaries.push(t);
            }
        }
    }
    boundaries.sort_by(f32::total_cmp);
    boundaries.dedup();
    for &t in boundaries
        .iter()
        .skip(1)
        .take(boundaries.len().saturating_sub(2))
    {
        if !permits(from.lerp(to, t))? {
            return Ok(false);
        }
    }
    for pair in boundaries.windows(2) {
        if !permits(from.lerp(to, (pair[0] + pair[1]) * 0.5))? {
            return Ok(false);
        }
    }
    Ok(true)
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
    mut hit: impl FnMut(Vec3, Vec3) -> Result<Option<f32>, String>,
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
    let mut points = vec![FlightPoint {
        position: ground_start + Vec3::Y * CLEARANCE,
        seconds: first.seconds,
    }];
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
        points.push(FlightPoint {
            position: Vec3::new(point.x, height, point.y),
            seconds: pair[1].seconds,
        });
    }
    repair_flight_segments(&mut points, ground_end, &mut hit)?;
    let mut path = vec![ground_start];
    path.extend(points.iter().map(|point| point.position));
    path.push(ground_end);
    for (index, pair) in path.windows(2).enumerate() {
        if hit(pair[0], pair[1])?.is_some() {
            return Err(format!(
                "flight segment {index} blocked: {:?} -> {:?}",
                pair[0], pair[1]
            ));
        }
    }
    Ok(path)
}

/// A blocked descent holds its height; other hits get a clearance waypoint.
fn repair_flight_segments(
    points: &mut Vec<FlightPoint>,
    landing: Vec3,
    hit: &mut impl FnMut(Vec3, Vec3) -> Result<Option<f32>, String>,
) -> Result<(), String> {
    bound_flight_altitudes(points)?;
    let mut index = 0;
    let mut repairs = 0;
    let mut holding_since = None::<f32>;
    while index + 1 < points.len() {
        let from = points[index];
        let to = points[index + 1];
        if holding_since.is_some_and(|since| to.seconds - since > MAX_DESCENT_HOLD_SECONDS) {
            return Err(format!(
                "FLIGHT_DESCENT_HOLD_TIMEOUT: no clear descent within {MAX_DESCENT_HOLD_SECONDS}s"
            ));
        }
        let allowance = VERTICAL_BOUND * (to.seconds - from.seconds);
        if allowance <= 0.0 || (to.position.y - from.position.y).abs() > allowance {
            return Err(format!(
                "flight altitude exceeds continuity bound: {:?} -> {:?}, allowance={allowance}",
                from.position, to.position
            ));
        }
        let Some(fraction) = hit(from.position, to.position)? else {
            if holding_since.is_some_and(|since| from.seconds >= since)
                && to.position.y < from.position.y
            {
                holding_since = None;
            }
            index += 1;
            continue;
        };
        if to.position.y < from.position.y {
            let held = Vec3::new(to.position.x, from.position.y, to.position.z);
            if hit(from.position, held)?.is_none() {
                holding_since.get_or_insert(from.seconds);
                points[index + 1].position = held;
                bound_flight_altitudes(points)?;
                index = 0;
                continue;
            }
        }
        if repairs == MAX_LOS_REPAIRS {
            return Err(format!(
                "flight LOS repair limit reached at segment {index}: {:?} -> {:?}",
                from.position, to.position
            ));
        }
        repairs += 1;
        if !(0.0..1.0).contains(&fraction) || fraction == 0.0 {
            return Err(format!(
                "invalid flight LOS hit fraction {fraction} at segment {index}"
            ));
        }
        let mut waypoint = from.position.lerp(to.position, fraction);
        waypoint.y += CLEARANCE;
        let seconds = from.seconds + (to.seconds - from.seconds) * fraction;
        if seconds <= from.seconds || seconds >= to.seconds {
            return Err(format!(
                "flight LOS hit has no distinct timestamp at segment {index}"
            ));
        }
        points.insert(
            index + 1,
            FlightPoint {
                position: waypoint,
                seconds,
            },
        );
        bound_flight_altitudes(points)?;
        index = 0;
    }
    if holding_since.is_some() && hit(points.last().unwrap().position, landing)?.is_some() {
        return Err("FLIGHT_DESCENT_HOLD_UNCLEARED: no clear descent before landing".into());
    }
    Ok(())
}

/// Raise the minimum required flight heights to satisfy the per-interval speed bound.
fn bound_flight_altitudes(points: &mut [FlightPoint]) -> Result<(), String> {
    for i in 1..points.len() {
        let allowance = VERTICAL_BOUND * (points[i].seconds - points[i - 1].seconds);
        if !allowance.is_finite() || allowance <= 0.0 {
            return Err("flight sample timestamps are not strictly increasing".into());
        }
        points[i].position.y = points[i]
            .position
            .y
            .max(points[i - 1].position.y - allowance);
    }
    for i in (1..points.len()).rev() {
        let allowance = VERTICAL_BOUND * (points[i].seconds - points[i - 1].seconds);
        points[i - 1].position.y = points[i - 1]
            .position
            .y
            .max(points[i].position.y - allowance);
    }
    Ok(())
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
    db: &rusqlite::Connection,
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
    let los = std::cell::RefCell::new(los::LosMap::new(
        &root.join(format!("los/{}", graph.map_id)),
    ));
    let nav = std::cell::RefCell::new(nav);
    let mut areas = server::test_support::flight::Areas::open(db, &root)?;
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
        if points.is_empty() {
            return Err(format!(
                "encounter {} has no logged flight samples",
                leg.encounter
            ));
        }
        let windows = permitted_flight_windows(&points, |point| areas.permits(graph.map_id, point))
            .map_err(|error| format!("encounter {} flight areas: {error}", leg.encounter))?;
        let count = windows.len();
        let mut reference = previous_boss_height;
        for (index, window) in windows.into_iter().enumerate() {
            let first = window.first().unwrap();
            let last = window.last().unwrap();
            let start = project_endpoint(
                &mut los.borrow_mut(),
                &mut nav.borrow_mut(),
                first.horizontal,
                reference,
            )?;
            let end_reference = if index + 1 == count {
                boss_height
            } else {
                start.y
            };
            let end = project_endpoint(
                &mut los.borrow_mut(),
                &mut nav.borrow_mut(),
                last.horizontal,
                end_reference,
            )?;
            let path = flight_path(
                start,
                end,
                &window,
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
                |from, to| los.borrow_mut().first_hit(from, to),
            )
            .map_err(|error| format!("encounter {} flight: {error}", leg.encounter))?;
            println!(
                "FLIGHT_AREA_CLIP encounter={} takeoff={start:?} landing={end:?}",
                leg.encounter
            );
            paths.push(path);
            reference = end.y;
        }
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
fn flight_lands_at_last_permitted_logged_point_before_the_no_fly_destination() {
    let samples: Vec<_> = [0.0, 10.0, 20.0, 30.0]
        .into_iter()
        .enumerate()
        .map(|(seconds, x)| LoggedPoint {
            horizontal: Vec2::new(x, 0.0),
            seconds: seconds as f32,
        })
        .collect();
    let windows = permitted_flight_windows(&samples, |point| Ok(point.x < 25.0)).unwrap();
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].first().unwrap().horizontal, Vec2::ZERO);
    assert_eq!(windows[0].last().unwrap().horizontal, Vec2::new(20.0, 0.0));
    assert_eq!(samples.last().unwrap().horizontal, Vec2::new(30.0, 0.0));
}

#[test]
fn flight_does_not_cross_a_no_fly_cell_between_permitted_samples() {
    let samples = [
        LoggedPoint {
            horizontal: Vec2::new(1.0, 0.0),
            seconds: 0.0,
        },
        LoggedPoint {
            horizontal: Vec2::new(100.0, 0.0),
            seconds: 10.0,
        },
    ];
    let windows =
        permitted_flight_windows(&samples, |point| Ok(point.x < 33.0 || point.x >= 67.0)).unwrap();
    assert!(
        windows.is_empty(),
        "the graph must use walking across the restricted cell"
    );
    let error = permitted_flight_windows(&samples, |_| Err("unknown area".into())).unwrap_err();
    assert!(error.contains("unknown area"));
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
            Ok(None)
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
        path[1..path.len() - 1]
            .windows(2)
            .chain(path.windows(2))
            .map(|p| (p[0], p[1]))
            .collect::<Vec<_>>()
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
        |_, _| Ok(None),
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
        |_, _| Ok(None),
    )
    .unwrap();
    assert_eq!(path[2], Vec3::new(5.0, 60.0, 0.0));
    assert_eq!(path[3], Vec3::new(10.0, 115.0, 0.0));
}

#[test]
fn flight_holds_a_blocked_descent_until_the_floor_is_clear() {
    let mut points = vec![
        FlightPoint {
            position: Vec3::new(0.0, 10.0, 0.0),
            seconds: 0.0,
        },
        FlightPoint {
            position: Vec3::new(1.0, 0.0, 0.0),
            seconds: 1.0,
        },
        FlightPoint {
            position: Vec3::new(20.0, 0.0, 0.0),
            seconds: 2.0,
        },
    ];
    let mut floor = |a: Vec3, b: Vec3| -> Result<Option<f32>, String> {
        if a.y > 5.0 && b.y < 5.0 {
            let t = (a.y - 5.0) / (a.y - b.y);
            if a.lerp(b, t).x <= 10.0 {
                return Ok(Some(t));
            }
        }
        Ok(None)
    };
    repair_flight_segments(&mut points, Vec3::new(20.0, 0.0, 0.0), &mut floor).unwrap();
    assert_eq!(points[1].position, Vec3::new(1.0, 10.0, 0.0));
    assert_eq!(points[2].position, Vec3::new(20.0, 0.0, 0.0));
    assert!(
        points
            .windows(2)
            .all(|pair| floor(pair[0].position, pair[1].position).unwrap().is_none())
    );
}

#[test]
fn flight_can_hold_across_more_logged_waypoints_than_the_insertion_limit() {
    let mut points: Vec<_> = (0..=12)
        .map(|i| FlightPoint {
            position: Vec3::new(i as f32, if i == 0 { 10.0 } else { 0.0 }, 0.0),
            seconds: i as f32,
        })
        .collect();
    points.push(FlightPoint {
        position: Vec3::new(30.0, 0.0, 0.0),
        seconds: 13.0,
    });
    let mut floor = |a: Vec3, b: Vec3| -> Result<Option<f32>, String> {
        if a.y > 5.0 && b.y < 5.0 {
            let t = (a.y - 5.0) / (a.y - b.y);
            if a.lerp(b, t).x <= 12.0 {
                return Ok(Some(t));
            }
        }
        Ok(None)
    };
    repair_flight_segments(&mut points, Vec3::new(30.0, 0.0, 0.0), &mut floor).unwrap();
    assert!(points[1..=12].iter().all(|point| point.position.y == 10.0));
    assert_eq!(points[13].position.y, 0.0);
}

#[test]
fn flight_descent_that_never_clears_fails_at_the_hold_timeout() {
    let mut points: Vec<_> = (0..=4)
        .map(|i| FlightPoint {
            position: Vec3::new(i as f32, if i == 0 { 10.0 } else { 0.0 }, 0.0),
            seconds: (i * 20) as f32,
        })
        .collect();
    let mut floor = |a: Vec3, b: Vec3| -> Result<Option<f32>, String> {
        Ok((a.y > 5.0 && b.y < 5.0).then(|| (a.y - 5.0) / (a.y - b.y)))
    };
    let error =
        repair_flight_segments(&mut points, Vec3::new(4.0, 0.0, 0.0), &mut floor).unwrap_err();
    assert!(error.contains("FLIGHT_DESCENT_HOLD_TIMEOUT"), "{error}");
}

#[test]
fn flight_descent_without_a_clear_landing_fails_explicitly() {
    let mut points = vec![
        FlightPoint {
            position: Vec3::new(0.0, 10.0, 0.0),
            seconds: 0.0,
        },
        FlightPoint {
            position: Vec3::new(1.0, 0.0, 0.0),
            seconds: 1.0,
        },
    ];
    let mut floor = |a: Vec3, b: Vec3| -> Result<Option<f32>, String> {
        Ok((a.y > 5.0 && b.y < 5.0).then(|| (a.y - 5.0) / (a.y - b.y)))
    };
    let error =
        repair_flight_segments(&mut points, Vec3::new(1.0, 0.0, 0.0), &mut floor).unwrap_err();
    assert!(error.contains("FLIGHT_DESCENT_HOLD_UNCLEARED"), "{error}");
}

#[test]
fn flight_inserts_clearance_above_a_blocking_wall() {
    let mut points = vec![
        FlightPoint {
            position: Vec3::ZERO,
            seconds: 0.0,
        },
        FlightPoint {
            position: Vec3::new(10.0, 0.0, 0.0),
            seconds: 2.0,
        },
    ];
    let mut wall = |a: Vec3, b: Vec3| -> Result<Option<f32>, String> {
        if a.x < 5.0 && b.x > 5.0 {
            let t = (5.0 - a.x) / (b.x - a.x);
            if a.lerp(b, t).y < 5.0 {
                return Ok(Some(t));
            }
        }
        Ok(None)
    };
    repair_flight_segments(&mut points, Vec3::new(10.0, 0.0, 0.0), &mut wall).unwrap();
    assert_eq!(points.len(), 3);
    assert_eq!(points[1].position, Vec3::new(5.0, 15.0, 0.0));
    assert!(
        points
            .windows(2)
            .all(|pair| wall(pair[0].position, pair[1].position).unwrap().is_none())
    );
}

#[test]
fn inferred_flight_rejects_invalid_surfaces_and_unrepairable_segments() {
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
    let clear_sky = flight_path(start, end, &samples, |_, _| Ok(vec![]), |_, _| Ok(None)).unwrap();
    assert_eq!(clear_sky[2], Vec3::new(10.0, 15.0, 0.0));
    let invalid = flight_path(
        start,
        end,
        &samples,
        |_, _| Ok(vec![f32::NAN]),
        |_, _| Ok(None),
    )
    .unwrap_err();
    assert!(invalid.contains("invalid flight surface"));
    let high_landing = Vec3::new(end.x, 250.0, end.z);
    let bounded = flight_path(
        start,
        high_landing,
        &samples,
        |_, _| Ok(vec![0.0]),
        |_, _| Ok(None),
    )
    .unwrap();
    assert_eq!(bounded[1].y, 165.0);
    assert_eq!(bounded[2].y, 265.0);
    let blocked = flight_path(
        start,
        end,
        &samples,
        |_, _| Ok(vec![0.0]),
        |a, b| Ok((a.x != b.x).then_some(0.5)),
    )
    .unwrap_err();
    assert!(blocked.contains("flight LOS repair limit"));
}
