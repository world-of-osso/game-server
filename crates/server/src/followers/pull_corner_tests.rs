//! Actual nav-corner origins, ordinary native cast eligibility, and no pre-acquisition.
use super::pull_endpoint_tests::{
    eligible_target, place_party_at_authored_point, read_route_budget_row,
    require_zero_measured_healing,
};
use super::pull_goal_tests::{Fixture, damage, physical_measured_fixture};
use super::{Follower, FollowerCatalog, ai, kit, kit_ai, pull_history, pull_sources};
use bevy::prelude::*;
use serde_json::{Value, json};
use shared::components::{Level, Mana};
use std::collections::{BTreeMap, BTreeSet};

const TARGET_GUID: u32 = 20375689;

#[derive(Clone, Copy)]
struct BakedCorner {
    point: Vec3,
    source_kind: &'static str,
    source_id: u32,
    path_index: usize,
}

fn collect_baked_corners(
    f: &mut Fixture,
    target: Entity,
    party: &[Entity],
) -> (Vec<BakedCorner>, Vec<Value>) {
    let map = *f.h.app.world().get::<crate::maps::InMap>(f.tank).unwrap();
    let target_point = crate::spell_cast::targets::position(f.h.app.world(), target).unwrap();
    let sources =
        pull_sources::read_present_sources(f.h.app.world_mut(), map, 1, &BTreeSet::new()).unwrap();
    let entrance = crate::spell_cast::targets::position(f.h.app.world(), f.tank).unwrap();
    let origins = std::iter::once(("entrance", 3599, entrance)).chain(
        sources
            .values()
            .map(|source| ("source_spawn", source.guid, source.position)),
    );
    let mut corners = BTreeMap::new();
    let mut errors = Vec::new();
    for (kind, id, origin) in origins {
        let from = match place_party_at_authored_point(f, origin, party) {
            Ok(from) => from,
            Err(error) => {
                errors.push(json!({"source_kind":kind,"source_id":id,"error":error}));
                continue;
            }
        };
        let path = match f
            .h
            .app
            .world()
            .resource::<crate::navigation::Navigation>()
            .query_path(map.map_id, from, target_point)
        {
            Ok(Some(path)) => path,
            result => {
                errors
                    .push(json!({"source_kind":kind,"source_id":id,"error":format!("{result:?}")}));
                continue;
            }
        };
        // Only interior native route vertices; never manufacture offsets or reuse endpoints.
        for (index, &point) in path
            .iter()
            .enumerate()
            .skip(1)
            .take(path.len().saturating_sub(2))
        {
            let key = [point.x.to_bits(), point.y.to_bits(), point.z.to_bits()];
            corners.entry(key).or_insert(BakedCorner {
                point,
                source_kind: kind,
                source_id: id,
                path_index: index,
            });
        }
    }
    (corners.into_values().collect(), errors)
}

fn available_offensive_action(
    f: &mut Fixture,
    target: Entity,
    party: &[Entity],
) -> (Option<u32>, Vec<Value>) {
    let follower = f.h.app.world().get::<Follower>(f.tank).unwrap().clone();
    let level = f.h.app.world().get::<Level>(f.tank).unwrap().0;
    let entries: Vec<_> =
        f.h.app
            .world()
            .resource::<FollowerCatalog>()
            .kits
            .iter()
            .filter(|entry| {
                entry.available(follower.profile.spec_id, level)
                    && matches!(entry.slot, kit::Slot::Attack | kit::Slot::Aoe)
                    && entry.conditions.target == kit::Target::Enemy
            })
            .cloned()
            .collect();
    assert!(
        !entries.is_empty(),
        "physical fixture requires sourced available offensive kit actions"
    );
    let enemies = ai::engaged_enemies(f.h.app.world_mut(), &follower, f.tank, party);
    let mut refusals = Vec::new();
    for entry in entries {
        if !kit_ai::conditions_met(f.h.app.world_mut(), f.tank, target, party, &enemies, &entry) {
            refusals.push(json!({"spell":entry.spell_id,"refusal":"actual kit conditions"}));
            continue;
        }
        let spell =
            f.h.app
                .world()
                .resource::<crate::spell_info::SpellStore>()
                .get(entry.spell_id)
                .expect("source kit spell metadata must exist")
                .clone();
        match crate::spell_cast::read_native_cast_start(f.h.app.world_mut(), f.tank, &spell, target)
        {
            Ok(Some(actual)) if actual == target => return (Some(entry.spell_id), refusals),
            result => {
                refusals.push(json!({"spell":entry.spell_id,"refusal":format!("{result:?}")}))
            }
        }
    }
    (None, refusals)
}

fn proximity_recipients(
    f: &mut Fixture,
    target: Entity,
    party: &[Entity],
) -> Result<Vec<u64>, String> {
    let mut recipients = Vec::new();
    for &unit in party {
        let acquires = if f
            .h
            .app
            .world()
            .get::<shared::components::Player>(unit)
            .is_some()
        {
            crate::creature_aggro::read_player_proximity_acquisition(
                f.h.app.world_mut(),
                target,
                unit,
            )?
        } else {
            crate::creature_sight::read_npc_proximity_acquisition(
                f.h.app.world_mut(),
                target,
                unit,
            )?
        };
        if acquires {
            recipients.push(unit.to_bits());
        }
    }
    Ok(recipients)
}

fn nearby_unengaged_sources(
    f: &mut Fixture,
    party: &[Entity],
    cleared: &BTreeSet<u32>,
) -> Result<Vec<(u32, Entity)>, String> {
    let world = f.h.app.world_mut();
    let follower = world.get::<Follower>(f.tank).unwrap().clone();
    let engaged: BTreeSet<_> = ai::engaged_enemies(world, &follower, f.tank, party)
        .into_iter()
        .map(|(unit, _)| unit)
        .collect();
    let map = *world.get::<crate::maps::InMap>(f.tank).unwrap();
    let sources = pull_sources::read_present_sources(world, map, 1, cleared)?;
    let origin = crate::spell_cast::targets::position(world, f.tank).unwrap();
    let mut state =
        bevy::ecs::system::SystemState::<crate::creature_aggro::AggroDistance>::new(world);
    let distance = state
        .get(world)
        .map_err(|error| format!("Native reaches unavailable: {error:?}"))?;
    let party_reach = party
        .iter()
        .map(|&unit| {
            distance
                .combat_reach(unit)
                .ok_or_else(|| format!("Missing actual party reach for {unit:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .fold(0.0_f32, f32::max);
    let mut nearby = Vec::new();
    for source in sources.values() {
        let actor = source
            .actor
            .ok_or_else(|| format!("Missing source actor {}", source.guid))?;
        if engaged.contains(&actor)
            || world
                .get::<crate::creature_aggro::AggroRange>(actor)
                .is_none()
        {
            continue;
        }
        let reach = distance
            .combat_reach(actor)
            .ok_or_else(|| format!("Missing source reach {}", source.guid))?;
        let maximum = crate::creature_aggro::MAX_AGGRO_RADIUS + reach + party_reach;
        let position = crate::spell_cast::targets::position(world, actor)
            .ok_or_else(|| format!("Missing source position {}", source.guid))?;
        if position.distance(origin) < maximum {
            nearby.push((source.guid, actor));
        }
    }
    Ok(nearby)
}

fn read_other_acquisitions(
    f: &mut Fixture,
    party: &[Entity],
    cleared: &BTreeSet<u32>,
) -> Result<Vec<Value>, String> {
    let nearby = nearby_unengaged_sources(f, party, cleared)?;
    let mut acquisitions = Vec::new();
    for (guid, actor) in nearby {
        if guid == TARGET_GUID {
            continue;
        }
        let recipients = proximity_recipients(f, actor, party)?;
        if !recipients.is_empty() {
            acquisitions.push(json!({"guid": guid, "recipients": recipients}));
        }
    }
    Ok(acquisitions)
}

fn read_corner(
    f: &mut Fixture,
    target: Entity,
    party: &[Entity],
    cleared: &BTreeSet<u32>,
    corner: BakedCorner,
) -> Value {
    let mut row = json!({"baked_corner":[corner.point.x,corner.point.y,corner.point.z],
        "source_kind":corner.source_kind,"source_id":corner.source_id,"path_index":corner.path_index,"qualified":false});
    let point = match place_party_at_authored_point(f, corner.point, party) {
        Ok(point) => point,
        Err(error) => {
            row["rejection"] = json!(error);
            return row;
        }
    };
    row["origin"] = json!([point.x, point.y, point.z]);
    let at = crate::spell_cast::targets::position(f.h.app.world(), target).unwrap();
    if point.x == at.x && point.z == at.z {
        row["rejection"] = json!("target-self origin has no real horizontal approach");
        return row;
    }
    let (spell, refusals) = available_offensive_action(f, target, party);
    row["native_cast_refusals"] = json!(refusals);
    let Some(spell) = spell else {
        row["rejection"] =
            json!("no available sourced action passes ordinary native cast-start validation");
        return row;
    };
    row["native_acquisition_spell"] = json!(spell);
    match proximity_recipients(f, target, party) {
        Ok(units) if units.is_empty() => {}
        Ok(units) => {
            row["proximity_recipients"] = json!(units);
            row["rejection"] =
                json!("target would already natively acquire party before planned pull");
            return row;
        }
        Err(error) => {
            row["rejection"] = json!(error);
            return row;
        }
    }
    let budget = read_route_budget_row(f, TARGET_GUID, cleared);
    row.as_object_mut()
        .unwrap()
        .extend(budget.as_object().unwrap().clone());
    let acquisitions = match read_other_acquisitions(f, party, cleared) {
        Ok(acquisitions) => acquisitions,
        Err(error) => {
            row["rejection"] = json!(error);
            return row;
        }
    };
    let footprint: BTreeSet<_> = row["spawns"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .collect();
    let uncovered = acquisitions
        .iter()
        .any(|source| !footprint.contains(&source["guid"].as_u64().unwrap()));
    row["other_native_acquisitions"] = json!(acquisitions);
    if uncovered {
        row["rejection"] =
            json!("another unengaged source would acquire outside the forecast footprint");
        return row;
    }
    if row["allowed"] == true {
        let length = row["horizontal_route_length"]
            .as_f64()
            .expect("native complete route length");
        if length > 0.0 {
            row["qualified"] = json!(true);
        } else {
            row["rejection"] = json!("native route has no real horizontal approach");
        }
    } else {
        row["rejection"] =
            json!("complete native route fails admission; scripted refusals remain explicit");
    }
    row
}

#[test]
fn native_tank_baked_corner_origins_for20375689() {
    let mut f = physical_measured_fixture();
    damage(&mut f.h, f.engaged, 1.0);
    pull_history::sample_all(f.h.app.world_mut());
    let history = pull_history::snapshot(f.h.app.world(), f.h.caster).unwrap();
    require_zero_measured_healing(history.healer_hps);
    assert!(
        history.completed_fights > 0,
        "this is measured active chaining, never a first-pull fixture"
    );
    let (map, members, _) = super::party::read_admission(f.h.app.world(), f.h.caster).unwrap();
    let mut party = members.to_vec();
    party.push(f.h.caster);
    let sources =
        pull_sources::read_present_sources(f.h.app.world_mut(), map, 1, &history.cleared_spawns)
            .unwrap();
    let target = sources[&TARGET_GUID].actor.unwrap();
    assert!(eligible_target(&f, target));
    let follower = f.h.app.world().get::<Follower>(f.tank).unwrap().clone();
    assert!(
        !ai::engaged_enemies(f.h.app.world_mut(), &follower, f.tank, &party)
            .iter()
            .any(|(enemy, _)| *enemy == target)
    );
    let mana_before = f.h.app.world().get::<Mana>(f.tank).map(|mana| mana.current);
    let cast_before =
        f.h.app
            .world()
            .get::<shared::casting::CastState>(f.tank)
            .map(|cast| cast.spell_id);
    let victim_before =
        f.h.app
            .world()
            .get::<crate::networking::CombatTarget>(f.tank)
            .and_then(|victim| victim.0);
    let (corners, path_errors) = collect_baked_corners(&mut f, target, &party);
    assert!(
        !corners.is_empty(),
        "actual baked routes must supply interior corners"
    );
    let rows: Vec<_> = corners
        .into_iter()
        .map(|corner| read_corner(&mut f, target, &party, &history.cleared_spawns, corner))
        .collect();
    assert_eq!(
        mana_before,
        f.h.app.world().get::<Mana>(f.tank).map(|mana| mana.current),
        "eligibility reads must not spend"
    );
    assert_eq!(
        cast_before,
        f.h.app
            .world()
            .get::<shared::casting::CastState>(f.tank)
            .map(|cast| cast.spell_id),
        "eligibility reads must not cast"
    );
    assert_eq!(
        victim_before,
        f.h.app
            .world()
            .get::<crate::networking::CombatTarget>(f.tank)
            .and_then(|victim| victim.0),
        "eligibility reads must not retarget"
    );
    assert!(
        !ai::engaged_enemies(f.h.app.world_mut(), &follower, f.tank, &party)
            .iter()
            .any(|(enemy, _)| *enemy == target),
        "target must stay genuinely unengaged"
    );
    let qualified = rows.iter().filter(|row| row["qualified"] == true).count();
    println!("TANK_BAKED_CORNER_ENUMERATION {}",serde_json::to_string(&json!({
        "target":TARGET_GUID,"origins":"interior points returned by actual native nav queries only; no offsets",
        "production_spawns_and_real_los":true,"native_combat_stats_unchanged":true,
        "measured_dps":history.group_dps,"measured_hps":history.healer_hps,
        "corner_count":rows.len(),"qualified_corners":qualified,"path_errors":path_errors,"rows":rows,
    })).unwrap());
}
