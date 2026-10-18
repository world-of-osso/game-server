//! Actual Stockade trigger actors are not pull candidates or proximity debt.
use super::pull_goal_tests::place_at_authored_entrance;
use super::pull_route::{self, Request};
use super::pull_route_tests::route_fixture;
use crate::maps::InMap;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use std::collections::BTreeSet;

fn actor(h: &mut Harness, tank: Entity, guid: u32) -> Entity {
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    h.app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin, &InMap)>()
        .iter(h.app.world())
        .find(|(_, origin, copy)| origin.guid == guid && **copy == map)
        .unwrap()
        .0
}

#[test]
fn native_pull_eligibility_stockade_triggers_are_never_candidates() {
    let (mut h, tank) = route_fixture();
    place_at_authored_entrance(&mut h, tank);
    let mut results = Vec::new();
    for guid in [20375809, 20375810] {
        let target = actor(&mut h, tank, guid);
        assert!(crate::spell_cast::targets::is_hostile(
            h.app.world(),
            tank,
            target
        ));
        assert!(
            !crate::spell_cast::targets::relation_matches(
                h.app.world(),
                tank,
                target,
                crate::spell_cast::targets::Relation::Enemy
            ),
            "real source flags make this target nonselectable"
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::world_data::WorldData>()
                .templates[&53488]
                .aggro_mode,
            crate::creature_aggro::AggroMode::Passive
        );
        assert!(
            h.app
                .world()
                .get::<crate::creature_aggro::AggroRange>(target)
                .is_none()
        );
        results.push(pull_route::read_route(
            h.app.world_mut(),
            &Request {
                tank,
                target_guid: guid,
                difficulty: 1,
                cleared: &BTreeSet::new(),
            },
        ));
    }
    assert!(
        results.iter().all(Result::is_err),
        "actual nonselectable passive trigger sources were accepted as pull candidates: {results:?}"
    );
}

#[test]
fn native_pull_eligibility_passive_triggers_never_cost_a_real_combat_route() {
    let (mut h, tank) = route_fixture();
    place_at_authored_entrance(&mut h, tank);
    let target = actor(&mut h, tank, 20375760);
    assert!(crate::spell_cast::targets::relation_matches(
        h.app.world(),
        tank,
        target,
        crate::spell_cast::targets::Relation::Enemy
    ));
    let route = pull_route::read_route(
        h.app.world_mut(),
        &Request {
            tank,
            target_guid: 20375760,
            difficulty: 1,
            cleared: &BTreeSet::new(),
        },
    )
    .unwrap();
    assert!(
        route.spawns.contains(&20375760),
        "keep the actual combat target's authored pack"
    );
    assert!(
        !route.spawns.contains(&20375809) && !route.spawns.contains(&20375810),
        "native passive trigger actors became proximity debt on a real walked combat route: {route:?}"
    );
}
