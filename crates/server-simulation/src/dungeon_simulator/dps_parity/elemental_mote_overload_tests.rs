//! Master of the Elements survives consumption between launch and overload flight.
use super::elemental_mechanics_tests::{cast, fixture, own_aura, settle};
use crate::auras::RemoveReason;
use crate::spell_cast::effects::factor_capture::FactorCapture;

fn overload_cast_factor(keep_mote: bool, late_mote: bool) -> (f64, f64) {
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
    cast(&mut run, 51505);
    let mote = own_aura(&run, 260734).expect("real Lava Burst earns MOTE");
    let percent = mote
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount as f64;
    if !keep_mote {
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.actor,
            RemoveReason::Consumed,
            |aura| aura.spell_id == 260734,
        );
    }
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        45297,
        "chain_lightning_overload",
        1,
    ));
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        188443,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    assert!(
        own_aura(&run, 260734).is_none(),
        "foreground Chain Lightning consumes MOTE before the delayed overload"
    );
    if late_mote {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            260734,
            Some(run.actor),
            1,
        );
        assert!(own_aura(&run, 260734).is_some());
    }
    for _ in 0..80 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let amount = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["spell_id"] == 45297 && event["event"] == "direct_amount")
        .expect("actual scheduled Chain Lightning Overload impact");
    let multiplier = amount["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["factor"] == "cast_effectiveness")
        .unwrap()["value"]
        .as_f64()
        .unwrap();
    (multiplier, percent)
}

#[test]
#[ignore = "requires native Elemental262 profile and world data"]
fn elemental_mote_snapshot_survives_parent_consumption_and_overload_flight() {
    let (without, _) = overload_cast_factor(false, false);
    let (with, percent) = overload_cast_factor(true, false);
    assert!(
        (with / without - (1.0 + percent / 100.0)).abs() < 0.001,
        "scheduled overload must retain earned MOTE through real parent and child flight: {with}/{without}, native {percent}%"
    );
    let (late, _) = overload_cast_factor(false, true);
    assert!(
        (late / without - 1.0).abs() < 0.001,
        "MOTE earned after scheduling must not change the already-snapshotted overload: {late}/{without}"
    );
}
