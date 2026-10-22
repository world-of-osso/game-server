//! Seed50's actual party positions and surviving source watchers, not invented geometry.
use super::super::{Space, diagnostics, runtime};
use super::{run, spawn_run};
use bevy::prelude::Vec3;
use serde::Deserialize;
use shared::components::{CombatStatus, Position};
use std::collections::BTreeSet;

#[derive(Deserialize)]
struct RecordedPositions {
    healer: [f32; 3],
    owner: [f32; 3],
    idle_watchers: BTreeSet<u32>,
}

fn segment_distance(center: Vec3, from: Vec3, to: Vec3) -> f32 {
    let direction = to - from;
    let squared = direction.length_squared();
    let fraction = if squared > 0.0 {
        ((center - from).dot(direction) / squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    center.distance(from + direction * fraction)
}

#[test]
#[ignore = "explicit native Stockade32 recorded late healer path, not its unwalked chord"]
fn late_healer_follows_the_safe_frozen_path_without_recruiting_idle_sources() {
    const PREPARATION_TICKS: usize = 200;
    const OBSERVATION_TICKS: usize = 40;
    let _rng_scope = crate::random::seed_scope(50);
    let fixture: RecordedPositions =
        serde_json::from_str(include_str!("fixtures/stockade50_party_positions.json")).unwrap();
    let mut run = spawn_run(50, &mut None).unwrap();
    assert_eq!(run.graph.map_id, 34);
    // Isolate the four actual surviving watchers. This is a movement-boundary
    // fixture, not a claim that despawned sources were killed or the dungeon cleared.
    for (guid, actor) in &run.actors {
        if !fixture.idle_watchers.contains(guid) {
            run.app.world_mut().despawn(*actor);
        }
    }
    for _ in 0..PREPARATION_TICKS {
        run.app.update();
    }
    let position = |[x, y, z]: [f32; 3]| Position { x, y, z };
    let healer = run.party.members[1];
    let owner = run.party.owner;
    let members: Vec<_> = run.members().collect();
    for member in members {
        let at = if member == healer {
            fixture.healer
        } else {
            fixture.owner
        };
        run.app.world_mut().entity_mut(member).insert(position(at));
    }
    let from = Vec3::from_array(fixture.healer);
    let to = Vec3::from_array(fixture.owner);
    let mut path = run.app.world().resource::<Space>().route(from, to).unwrap();
    path.insert(0, from);
    for guid in &fixture.idle_watchers {
        let actor = run.actors[guid];
        let geometry = diagnostics::inspect_aggro_pair(run.app.world_mut(), actor, healer).unwrap();
        let radius = geometry["npc_acquisition_radius"].as_f64().unwrap() as f32;
        let at = *run.app.world().get::<Position>(actor).unwrap();
        let center = Vec3::new(at.x, at.y, at.z);
        assert!(
            segment_distance(center, from, to) < radius,
            "recorded source{guid} must actually expose the unwalked chord"
        );
        assert!(
            path.windows(2)
                .all(|pair| segment_distance(center, pair[0], pair[1]) >= radius),
            "actual frozen movement must avoid source{guid}'s native sphere"
        );
    }
    for _ in 0..OBSERVATION_TICKS {
        run.app.update();
    }
    let after = run.app.world().get::<Position>(healer).unwrap();
    assert_ne!(
        after,
        &position(fixture.healer),
        "native healer must move on the clear frozen path instead of holding on an unwalked chord"
    );
    assert_eq!(
        run.app.world().get::<Position>(owner),
        Some(&position(fixture.owner))
    );
    for guid in &fixture.idle_watchers {
        assert!(
            run.app
                .world()
                .get::<CombatStatus>(run.actors[guid])
                .is_none_or(|status| !status.0),
            "native follow must not recruit isolated source{guid}"
        );
    }
}

#[test]
#[ignore = "explicit full Stockade32 seed50 late tank-death counterexample"]
fn stockade_seed_fifty_keeps_the_native_tank_alive_through_the_route() {
    let report = run(50).unwrap();
    runtime::write_report(&report).unwrap();
    runtime::print_report(&report);
    let tank = &report.samples.last().unwrap().members[0];
    assert!(
        tank.exists && tank.health.is_some_and(|health| health > 0.0),
        "native tank must not die while its healer is held behind the actual route: {}",
        report.detail
    );
    assert_eq!(report.status, "cleared");
}
