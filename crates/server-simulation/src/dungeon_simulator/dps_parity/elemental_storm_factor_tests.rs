//! SimC76f8118 sc_shaman.cpp3254–3257/7320–7327: live Stormkeeper affects
//! the delayed Lightning Bolt Overload, even though its native mask does not.
use super::elemental_mechanics_tests::{fixture, own_aura, settle};
use crate::spell_cast::effects::factor_capture::FactorCapture;

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
        .find(|event| event["spell_id"] == 45284 && event["factors"]["target_product"].is_number())
        .expect("scheduled overload impact");
    let product = hit["factors"]["target_product"].as_f64().unwrap();
    let expected = 1.0 + percent / 100.0;
    assert!(
        (product - expected).abs() < 0.001,
        "live Stormkeeper target product {product}, expected {expected}; {hit}"
    );
}
