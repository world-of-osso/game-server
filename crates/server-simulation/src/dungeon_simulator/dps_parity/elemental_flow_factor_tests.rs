//! SimC76f8118 sc_shaman.cpp3265–3267/7323–7324: actual delayed Bolt
//! Overload observes the remaining native Flowing Elements builder use.
use super::elemental_mechanics_tests::{cast, fixture, own_aura, settle};
use crate::spell_cast::effects::factor_capture::FactorCapture;

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_flowing_first_actual_bolt_overload_captures_live_damage_factor() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    assert!(
        own_aura(&run, 1296626).is_some(),
        "native MID2 set equipped"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        191634,
        Some(run.actor),
        false,
    )
    .unwrap();
    settle(&mut run);
    cast(&mut run, 188196);
    cast(&mut run, 188196);
    let aura =
        own_aura(&run, 1300219).expect("actual Stormkeeper exhaustion earns Flowing Elements");
    assert_eq!(aura.stacks, 2);
    let percent = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount as f64;
    assert_eq!(percent, 25.0, "native70448 Flowing Elements effect0");
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        45284,
        "lightning_bolt_overload",
        1,
    ));
    let _cast_seed = crate::random::seed_scope(70448);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        188196,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    for _ in 0..120 {
        run.app.update();
    }
    assert_eq!(
        own_aura(&run, 1300219).unwrap().stacks,
        1,
        "real builder consumes one use"
    );
    let capture = run.app.world().resource::<FactorCapture>().export();
    let events = capture["events"]
        .as_array()
        .expect("captured native events");
    let hit = events
        .iter()
        .find(|event| {
            event["spell_id"] == 45284 && event["factors"]["native_done_product"].is_number()
        })
        .expect("actual scheduled overload impact");
    let done = hit["factors"]["native_done_product"].as_f64().unwrap();
    let versatility = hit["factors"]["versatility"].as_f64().unwrap();
    let product = done / versatility;
    let expected = 1.0 + percent / 100.0;
    assert!(
        (product - expected).abs() < 0.001,
        "live Flowing Elements product {product}, expected {expected}; {hit}"
    );
}
