//! Natural flying-parent qualification precedes any flying Liquid Fire hook.
use super::herald_call::first_west;
use super::*;
use crate::scripted_ai::tests::cast_admission::CastAdmissions;
use crate::spell_cast::test_harness::ObservedCombatEvents;

fn qualify_flying_parents(difficulty: u32) {
    let (mut h, map, nazan, _) = first_west(difficulty);
    h.app.init_resource::<ObservedCombatEvents>();
    let parents = if difficulty == 2 {
        [32491, 33794]
    } else {
        [30691, 33793]
    };
    for parent in parents {
        let spell = h.app.world().resource::<SpellStore>().get(parent).unwrap();
        assert_eq!(spell.effects.len(), 2);
        assert_eq!((spell.effects[0].index, spell.effects[0].effect), (0, 2));
        assert_eq!((spell.effects[1].index, spell.effects[1].effect), (1, 77));
        assert!(
            spell
                .effects
                .iter()
                .all(|effect| effect.implicit_targets == [6, 0])
        );
        let misc = spell.misc.as_ref().unwrap();
        assert_eq!(misc.cast_time.unwrap().base_ms, 3000);
        assert_eq!(misc.speed, 24.0);
        assert_eq!(misc.launch_delay, 0.0);
        println!(
            "native flying parent: {}",
            serde_json::to_string(spell).unwrap()
        );
    }
    let hover = *h.app.world().get::<Position>(nazan).unwrap();
    let ground = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, Vec3::new(hover.x + 3.0, hover.y, hover.z));
    let crate::ground::GroundProbe::Supported(floor) = ground else {
        panic!("native floor required for ordinary flying-phase recipient: {ground:?}")
    };
    let player = h.player();
    // Harness pose only: a native grounded recipient near actual first West.
    h.app.world_mut().entity_mut(player).insert(Position {
        x: hover.x + 3.0,
        y: floor,
        z: hover.z,
    });
    let origin = h.app.world().resource::<Time<Fixed>>().elapsed_secs_f64();
    h.app.insert_resource(CastAdmissions {
        actor: nazan,
        origin,
        entries: Vec::new(),
    });
    let mut hits = [false; 2];
    for _ in 0..2240 {
        h.advance(0.05);
        let events = &h.app.world().resource::<ObservedCombatEvents>().0;
        for (index, parent) in parents.into_iter().enumerate() {
            hits[index] = events.iter().any(|event| {
                event.source == Some(nazan.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(parent)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            });
        }
        if hits.into_iter().all(|hit| hit) {
            break;
        }
    }
    let admissions = &h.app.world().resource::<CastAdmissions>().entries;
    println!("natural flying difficulty{difficulty} admissions: {admissions:?}");
    println!("natural flying difficulty{difficulty} positive-hit qualification: {hits:?}");
    for (index, parent) in parents.into_iter().enumerate() {
        assert!(
            admissions
                .iter()
                .any(|entry| entry.spell == parent && entry.result.is_ok()),
            "ordinary naturally scheduled parent{parent} admission must qualify within112s"
        );
        assert!(
            hits[index],
            "ordinary native parent{parent} must really impact within112s; no forced hit"
        );
    }
    assert!(
        h.app
            .world()
            .get::<crate::scripted_ai::waypoint_path::GravityDisabled>(nazan)
            .is_some(),
        "qualification must remain in source flying phase"
    );
}

#[test]
fn encounter_coverage_herald_flying_liquid_fire_normal_admission_qualification() {
    qualify_flying_parents(1);
}

#[test]
fn encounter_coverage_herald_flying_liquid_fire_heroic_admission_qualification() {
    qualify_flying_parents(2);
}
