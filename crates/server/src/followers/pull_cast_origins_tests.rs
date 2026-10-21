//! Bounded authored-origin diagnosis; a failed setup is never a movement-policy RED.
use super::super::{party, pull_endpoint_tests, pull_sources};
use super::{CasterFixture, Fixture, OVERHEAT_FIREBALL, OVERHEAT_GUID, REBUKE};
use super::{
    overheat_fixture, physical_fixture, place_party_at_authored_point, read_caster_boundary, source,
};
use bevy::prelude::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn observe_refusal_boundaries(f: &mut Fixture) -> Value {
    f.h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    crate::creature_ai::install_follower_follow_fixture(&mut f.h.app);
    let step = f.h.app.world().resource::<Time<Fixed>>().timestep();
    assert_eq!(
        step,
        std::time::Duration::from_millis(50),
        "native fixed-tick observation"
    );
    let caster = source(&mut f.h, f.tank, OVERHEAT_GUID);
    let mut boundaries = Vec::new();
    let mut qualified = 0;
    let mut violations = Vec::new();
    for tick in 0..10 {
        let before_point = crate::spell_cast::targets::position(f.h.app.world(), f.tank).unwrap();
        let before = read_caster_boundary(f);
        let before_tick = f.h.app.world().resource::<Time<Fixed>>().elapsed();
        f.h.advance(1);
        assert_eq!(
            f.h.app.world().resource::<Time<Fixed>>().elapsed() - before_tick,
            step
        );
        let after = read_caster_boundary(f);
        let at = crate::spell_cast::targets::position(f.h.app.world(), f.tank).unwrap();
        let interrupted =
            f.h.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| {
                    event.kind == shared::protocol::CombatLogKind::Interrupt
                        && event.target == Some(caster.to_bits())
                        && event.spell_id == Some(2139)
                        && event.extra_spell_id == Some(OVERHEAT_FIREBALL)
                });
        // Real Counterspell can end the pickup opportunity on the first tick.
        // It does not authorize entering the same unengaged route afterwards.
        let cast_accounted_for = (before.casting && after.casting) || interrupted;
        let qualifies = before.exposed
            && after.exposed
            && before.refused
            && after.refused
            && cast_accounted_for;
        if qualifies {
            qualified += 1;
            if at != before_point {
                violations.push(tick);
            }
        }
        boundaries.push(
            json!({"tick":tick,"qualified":qualifies,"moved":at!=before_point,"native_counterspell":interrupted,
            "before":before.details,"after":after.details}),
        );
    }
    let events =
        &f.h.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
    let tail: Vec<_> = events.iter().rev().take(24).collect();
    json!({"qualified_boundaries":qualified,"violation_ticks":violations,"boundaries":boundaries,"recent_events":tail})
}

struct OriginProbe {
    caster: Entity,
    fireball: crate::spell_info::SpellInfo,
    rebuke: crate::spell_info::SpellInfo,
    party: Vec<Entity>,
}

impl OriginProbe {
    fn read_origin(&self, f: &mut Fixture, source: &pull_sources::Source) -> Value {
        let mut row = json!({"origin_kind":"source_spawn","origin_guid":source.guid,"entry":source.entry,"candidate":false});
        let origin = match place_party_at_authored_point(f, source.position, &self.party) {
            Ok(origin) => origin,
            Err(error) => {
                row["error"] = json!(error);
                return row;
            }
        };
        row["origin"] = json!([origin.x, origin.y, origin.z]);
        row["native_los"] = json!(crate::line_of_sight::sees_in_world(
            f.h.app.world(),
            self.caster,
            f.tank
        ));
        let cast = crate::spell_cast::read_native_cast_start(
            f.h.app.world_mut(),
            self.caster,
            &self.fireball,
            f.tank,
        );
        row["ordinary_cast_start"] = json!(format!("{cast:?}"));
        row["native_range_metadata"] = json!(crate::spell_cast::targets::spell_range(
            &self.fireball,
            true
        ));
        let actor = source
            .actor
            .expect("authored origin enumeration requires actual source actors");
        let eligible = actor != self.caster && pull_endpoint_tests::eligible_target(f, actor);
        row["eligible_selected_trash"] = json!(eligible);
        let melee = crate::spell_cast::read_native_cast_start(
            f.h.app.world_mut(),
            f.tank,
            &self.rebuke,
            actor,
        );
        row["selected_native_melee_start"] = json!(format!("{melee:?}"));
        row["candidate"] = json!(
            eligible
                && matches!(cast,Ok(Some(target)) if target==f.tank)
                && matches!(melee,Ok(Some(target)) if target==actor)
        );
        row
    }
}

fn panic_message(error: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = error.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = error.downcast_ref::<&str>() {
        return (*message).to_owned();
    }
    "non-string native fixture panic".to_owned()
}

#[test]
fn native_overheat_caster_collection_refuses_measured_unengaged_route() {
    // Authored-origin enumeration88b0309ac measured ten qualifying boundaries here.
    let CasterFixture { mut f, .. } = overheat_fixture(false, 20375694);
    let observation = observe_refusal_boundaries(&mut f);
    assert_eq!(observation["qualified_boundaries"], 10, "{observation}");
    assert!(
        observation["violation_ticks"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{observation}"
    );
}

#[test]
fn native_overheat_authored_origin_refusal_enumeration() {
    let mut probe_fixture = physical_fixture();
    let map = *probe_fixture
        .h
        .app
        .world()
        .get::<crate::maps::InMap>(probe_fixture.tank)
        .unwrap();
    let (_, members, _) =
        party::read_admission(probe_fixture.h.app.world(), probe_fixture.h.caster).unwrap();
    let mut party = members.to_vec();
    party.push(probe_fixture.h.caster);
    let caster = source(&mut probe_fixture.h, probe_fixture.tank, OVERHEAT_GUID);
    let store = probe_fixture
        .h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>();
    let probe = OriginProbe {
        caster,
        fireball: store.get(OVERHEAT_FIREBALL).unwrap().clone(),
        rebuke: store.get(REBUKE).unwrap().clone(),
        party,
    };
    let origins = pull_sources::read_present_sources(
        probe_fixture.h.app.world_mut(),
        map,
        1,
        &BTreeSet::new(),
    )
    .unwrap();
    let entrance =
        crate::spell_cast::targets::position(probe_fixture.h.app.world(), probe_fixture.tank)
            .unwrap();
    let entrance_cast = crate::spell_cast::read_native_cast_start(
        probe_fixture.h.app.world_mut(),
        caster,
        &probe.fireball,
        probe_fixture.tank,
    );
    let entrance_los = crate::line_of_sight::sees_in_world(
        probe_fixture.h.app.world(),
        caster,
        probe_fixture.tank,
    );
    let mut rows = vec![
        json!({"origin_kind":"entrance","origin_id":3599,"origin":[entrance.x,entrance.y,entrance.z],
        "ordinary_cast_start":format!("{entrance_cast:?}"),"native_los":entrance_los,
        "native_range_metadata":crate::spell_cast::targets::spell_range(&probe.fireball,true),
        "candidate":false,"eligible_selected_trash":false,"reason":"authored entrance has no selected trash at its own point"}),
    ];
    rows.extend(
        origins
            .values()
            .map(|source| probe.read_origin(&mut probe_fixture, source)),
    );
    assert_eq!(
        rows.len(),
        origins.len() + 1,
        "every authored source origin and native entrance3599 is recorded"
    );
    drop(probe_fixture);
    let probe_errors = rows.iter().filter(|row| row.get("error").is_some()).count();
    let mut prerequisite_failures = 0;
    for row in &mut rows {
        if row["candidate"] != true {
            continue;
        }
        let guid = row["origin_guid"].as_u64().unwrap() as u32;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let CasterFixture { mut f, .. } = overheat_fixture(false, guid);
            observe_refusal_boundaries(&mut f)
        }));
        match result {
            Ok(observation) => row["observation"] = observation,
            Err(error) => {
                row["prerequisite_error"] = json!(panic_message(error));
                prerequisite_failures += 1;
            }
        }
    }
    let qualified: Vec<_> = rows
        .iter()
        .filter(|row| {
            row["observation"]["qualified_boundaries"]
                .as_u64()
                .is_some_and(|count| count > 0)
        })
        .map(|row| row["origin_guid"].as_u64().unwrap())
        .collect();
    let violations: Vec<_> = rows
        .iter()
        .filter(|row| {
            row["observation"]["violation_ticks"]
                .as_array()
                .is_some_and(|ticks| !ticks.is_empty())
        })
        .map(|row| row["origin_guid"].as_u64().unwrap())
        .collect();
    println!("OVERHEAT_ORIGIN_ENUMERATION {}",serde_json::to_string(&json!({
        "caster_guid":OVERHEAT_GUID,"native_spell":OVERHEAT_FIREBALL,"origins":rows.len(),
        "qualified_origins":qualified,"violating_origins":violations,"probe_errors":probe_errors,"prerequisite_failures":prerequisite_failures,
        "scope":"read-only native cast validation, then fresh disposable actual-kit fixed-tick observations; no live gameplay or offsets",
        "rows":rows})).unwrap());
    assert_eq!(
        probe_errors, 0,
        "recorded native origin input errors are not behavioral evidence"
    );
    assert_eq!(
        prerequisite_failures, 0,
        "recorded fixture prerequisites are not behavioral evidence"
    );
    assert!(
        violations.is_empty(),
        "actual unengaged caster-refusal movement violations: {violations:?}"
    );
}
