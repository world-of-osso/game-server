//! Lead-approved, origin-independent endpoint lower bounds for the current native run.
use super::pull_budget::{Decision, TANK_HP_RESERVE_FRACTION};
use super::pull_goal_tests::{Fixture, damage, measured_fixture};
use super::{pull_admission, pull_history, pull_route, pull_sources};
use bevy::prelude::*;
use serde_json::{Value, json};
use shared::components::{Health, Position};

fn require_zero_measured_healing(hps: Option<f64>) {
    assert_eq!(
        hps,
        Some(0.0),
        "endpoint lower bound requires measured HPS0; positive or unknown healing invalidates monotonicity"
    );
}

#[test]
#[should_panic(expected = "endpoint lower bound requires measured HPS0")]
fn endpoint_lower_bound_refuses_positive_measured_healing() {
    require_zero_measured_healing(Some(1.0));
}

fn place_party_at_endpoint(
    f: &mut Fixture,
    actor: Entity,
    party: &[Entity],
) -> Result<Vec3, String> {
    let map = *f.h.app.world().get::<crate::maps::InMap>(f.tank).unwrap();
    let position =
        *f.h.app
            .world()
            .get::<Position>(actor)
            .ok_or("Endpoint actor lacks position")?;
    let point = Vec3::new(position.x, position.y, position.z);
    let crate::ground::GroundProbe::Supported(y) =
        f.h.app
            .world_mut()
            .resource_mut::<crate::ground::GroundMap>()
            .creature_ground_in(map, point)
    else {
        return Err("Native endpoint lacks actual ground support".into());
    };
    let from = Position { y, ..position };
    for &member in party {
        f.h.app.world_mut().entity_mut(member).insert(from);
    }
    Ok(Vec3::new(from.x, from.y, from.z))
}

fn eligible_target(f: &Fixture, actor: Entity) -> bool {
    let world = f.h.app.world();
    if !crate::spell_cast::targets::is_alive(world, actor)
        || !crate::spell_cast::targets::relation_matches(
            world,
            f.tank,
            actor,
            crate::spell_cast::targets::Relation::Enemy,
        )
    {
        return false;
    }
    let npc = world.get::<shared::components::Npc>(actor).unwrap();
    let difficulty = world
        .get::<crate::zones::CreatureDifficulty>(actor)
        .map_or(0, |difficulty| difficulty.0);
    world
        .resource::<crate::world_data::WorldData>()
        .template_for(npc.template_id, difficulty)
        .unwrap()
        .aggro_mode
        != crate::creature_aggro::AggroMode::Passive
}

fn endpoint_budget_row(
    f: &mut Fixture,
    guid: u32,
    cleared: &std::collections::BTreeSet<u32>,
) -> Value {
    let route = match pull_route::read_route(
        f.h.app.world_mut(),
        &pull_route::Request {
            tank: f.tank,
            target_guid: guid,
            difficulty: 1,
            cleared,
        },
    ) {
        Ok(route) => route,
        Err(error) => return json!({"error": error, "allowed": false}),
    };
    let mut row = json!({
        "endpoint_pack_count": route.pack_count,
        "spawns": route.spawns,
        "allowed": false,
    });
    let decision =
        match pull_admission::read_pull_budget(f.h.app.world_mut(), f.tank, &route.spawns, 1) {
            Ok(decision) => decision,
            Err(error) => {
                row["error"] = json!(error);
                return row;
            }
        };
    row["decision"] = json!(format!("{decision:?}"));
    match pull_admission::read_forecast_health(f.h.app.world_mut(), f.tank, &route.spawns, 1) {
        Ok((forecast, reserve, packs)) => {
            match &decision {
                Decision::Allow {
                    bootstrap: false,
                    forecast_health: Some(accepted),
                } => assert_eq!(forecast, *accepted),
                Decision::Wait(super::pull_budget::Reason::Reserve) => assert!(forecast < reserve),
                _ => {}
            }
            row["total_projected_pack_count"] = json!(packs);
            row["forecast_health"] = json!(forecast);
            row["reserve_health"] = json!(reserve);
            row["allowed"] = json!(matches!(&decision, Decision::Allow { .. }));
        }
        Err(error) => row["error"] = json!(error),
    }
    row
}

#[test]
fn native_tank_endpoint_budget_lower_bound_enumeration() {
    let mut f = measured_fixture();
    damage(&mut f.h, f.engaged, 1.0);
    pull_history::sample_all(f.h.app.world_mut());
    let history = pull_history::snapshot(f.h.app.world(), f.h.caster).unwrap();
    require_zero_measured_healing(history.healer_hps);
    assert!(history.completed_fights > 0 && history.group_dps.is_some_and(|dps| dps > 0.0));
    let (map, members, _) = super::party::read_admission(f.h.app.world(), f.h.caster).unwrap();
    let mut party = members.to_vec();
    party.push(f.h.caster);
    let sources =
        pull_sources::read_present_sources(f.h.app.world_mut(), map, 1, &history.cleared_spawns)
            .unwrap();
    let health = *f.h.app.world().get::<Health>(f.tank).unwrap();
    let reserve = f64::from(health.max) * TANK_HP_RESERVE_FRACTION;
    let mut rows = Vec::new();
    let mut rejected = Vec::new();
    for source in sources.values() {
        let actor = source
            .actor
            .expect("Full native source population must be present for endpoint proof");
        if !eligible_target(&f, actor) {
            rejected.push(source.guid);
            continue;
        }
        let mut row = match place_party_at_endpoint(&mut f, actor, &party) {
            Ok(point) => {
                let mut row = endpoint_budget_row(&mut f, source.guid, &history.cleared_spawns);
                row["endpoint"] = json!([point.x, point.y, point.z]);
                row
            }
            Err(error) => json!({"error": error, "allowed": false}),
        };
        row["target"] = json!(source.guid);
        row["entry"] = json!(source.entry);
        row["source_pack_guid"] = json!(source.pack_guid);
        row["source_pack_size"] = json!(
            sources
                .values()
                .filter(|other| other.pack_guid == source.pack_guid)
                .count()
        );
        row["reserve_health"] = json!(reserve);
        rows.push(row);
    }
    assert!(
        !rows.is_empty(),
        "Endpoint proof must enumerate actual combat targets"
    );
    let passing: Vec<_> = rows
        .iter()
        .filter(|row| row["allowed"] == true)
        .map(|row| row["target"].as_u64().unwrap())
        .collect();
    let report = json!({
        "proof": "mandatory complete endpoint footprint lower bound, not an origin-route matrix",
        "monotonicity": "Measured HPS is exactly0. Added source packs only add hostile HP and nonnegative native swing debt; they cannot improve forecast HP. Unsupported source/stat lifetimes still refuse.",
        "measured_dps": history.group_dps,
        "measured_hps": history.healer_hps,
        "observation_seconds": history.observation_seconds,
        "completed_fights": history.completed_fights,
        "native_tank_health": health.current,
        "native_tank_max_health": health.max,
        "reserve_health": reserve,
        "cleared_spawns": history.cleared_spawns,
        "eligible_targets": rows.len(),
        "rejected_targets": rejected,
        "passing_endpoint_targets_requiring_full_authored_routes": passing,
        "rows": rows,
    });
    println!(
        "TANK_ENDPOINT_ENUMERATION {}",
        serde_json::to_string(&report).unwrap()
    );
}
