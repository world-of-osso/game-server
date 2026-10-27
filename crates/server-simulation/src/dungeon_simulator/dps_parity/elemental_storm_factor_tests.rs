//! SimC76f8118 sc_shaman.cpp3254–3257/7320–7327: live Stormkeeper affects
//! the delayed Lightning Bolt Overload, even though its native mask does not.
use super::elemental_mechanics_tests::{fixture, own_aura, settle};
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::Position;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_stormkeeper_overload_snapshots_before_projectile_impact() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    let actor_pos = *run.app.world().get::<Position>(run.actor).unwrap();
    *run.app
        .world_mut()
        .get_mut::<Position>(run.targets[0])
        .unwrap() = Position {
        x: actor_pos.x + 30.0,
        ..actor_pos
    };
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        191634,
        Some(run.actor),
        false,
    )
    .unwrap();
    settle(&mut run);
    assert_eq!(own_aura(&run, 191634).unwrap().stacks, 2);
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        45284,
        "lightning_bolt_overload",
        1,
    ));
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        188196,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    for _ in 0..30 {
        run.app.update();
        if run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.spell_id == Some(45284) && event.kind == CombatLogKind::CastSuccess)
        {
            break;
        }
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.spell_id == Some(45284) && event.kind == CombatLogKind::CastSuccess),
        "real delayed overload launched"
    );
    assert!(
        !events
            .iter()
            .any(|event| event.spell_id == Some(45284) && event.kind == CombatLogKind::Damage),
        "overload is in flight"
    );
    let instance = own_aura(&run, 191634)
        .expect("one Stormkeeper use at overload launch")
        .instance_id;
    crate::auras::remove_instance(
        run.app.world_mut(),
        run.actor,
        instance,
        crate::auras::RemoveReason::Consumed,
    );
    if let Some(flowing) = own_aura(&run, 1300219) {
        let instance = flowing.instance_id;
        crate::auras::remove_instance(
            run.app.world_mut(),
            run.actor,
            instance,
            crate::auras::RemoveReason::Consumed,
        );
    }
    for _ in 0..40 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let events = capture["events"].as_array().unwrap();
    let result = events
        .iter()
        .find(|event| event["spell_id"] == 45284 && event["event"] == "result")
        .expect("overload impact");
    let amount = events
        .iter()
        .find(|event| event["spell_id"] == 45284 && event["event"] == "direct_amount")
        .unwrap();
    let cast = amount["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|factor| factor["factor"] == "cast_effectiveness")
        .unwrap()["value"]
        .as_f64()
        .unwrap();
    let done = result["factors"]["native_done_product"].as_f64().unwrap();
    let versatility = result["factors"]["versatility"].as_f64().unwrap();
    let factor = cast * done / versatility;
    assert!(
        (factor - 2.5).abs() < 0.001,
        "in-flight overload retains earned native Stormkeeper factor: {factor}"
    );
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_stormkeeper_first_bolt_overload_captures_live_damage_factor() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        191634,
        Some(run.actor),
        false,
    )
    .unwrap();
    settle(&mut run);
    let aura = own_aura(&run, 191634).expect("real Stormkeeper");
    assert_eq!(aura.stacks, 2);
    let percent = aura
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount as f64;
    assert_eq!(percent, 150.0, "native70448 Stormkeeper effect1");
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        45284,
        "lightning_bolt_overload",
        1,
    ));
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        188196,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    assert_eq!(
        own_aura(&run, 191634).unwrap().stacks,
        1,
        "first actual Bolt consumes one use"
    );
    for _ in 0..60 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let events = capture["events"]
        .as_array()
        .expect("captured native events");
    let hit = events
        .iter()
        .find(|event| {
            event["spell_id"] == 45284 && event["factors"]["native_done_product"].is_number()
        })
        .expect("scheduled overload impact");
    let amount = events
        .iter()
        .find(|event| event["spell_id"] == 45284 && event["event"] == "direct_amount")
        .unwrap();
    let cast = amount["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|factor| factor["factor"] == "cast_effectiveness")
        .unwrap()["value"]
        .as_f64()
        .unwrap();
    let done = hit["factors"]["native_done_product"].as_f64().unwrap();
    let versatility = hit["factors"]["versatility"].as_f64().unwrap();
    let product = cast * done / versatility;
    let expected = 1.0 + percent / 100.0;
    assert!(
        (product - expected).abs() < 0.001,
        "live Stormkeeper damage product {product}, expected {expected}; {hit}"
    );
}
