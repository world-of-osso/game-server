//! Constant-haste native Ray must not conjure a fractional extra channel tick.
use super::channel_timing_tests::remove_equipped_proc_drivers;
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const RAY: u32 = 205021;

#[test]
#[ignore = "native70448 exact Frost64 Ray channel rounding RED"]
fn frost_ray_native_constant_haste_pays_exact_native_channel_tick_count() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    remove_equipped_proc_drivers(&mut run);
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(RAY)
        .unwrap();
    let base_duration = spell.misc.as_ref().unwrap().duration.unwrap().duration_ms;
    let base_period = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .aura_period_ms;
    assert_eq!(base_duration as u32 % base_period, 0);
    let expected_ticks = base_duration as u32 / base_period;
    assert_eq!(expected_ticks, 8, "exact native4000ms/500ms fixture");
    let haste = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .haste_pct;
    assert!(haste > 0.0);
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        RAY,
        "ray_rounding",
        32,
    ));
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(RAY, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    let mut largest_haste_change = 0.0_f32;
    for _ in 0..120 {
        run.app.update();
        let current = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .haste_pct;
        largest_haste_change = largest_haste_change.max((current - haste).abs());
    }
    assert!(
        largest_haste_change < 0.00001,
        "this regression must isolate constant haste, not a real mid-channel stat proc: before{haste}, largest change{largest_haste_change}"
    );
    let hits: Vec<_> = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(RAY)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0
        })
        .map(|event| event.amount)
        .collect();
    let trace = run.app.world().resource::<FactorCapture>().export();
    let fractions: Vec<_> = trace["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["spell_id"] == RAY && event["event"] == "periodic_amount")
        .flat_map(|event| event["applied_multipliers"].as_array().unwrap())
        .filter(|row| row["factor"] == "cast_effectiveness")
        .map(|row| row["value"].clone())
        .collect();
    // SimC76f8118 action.cpp4297–4314/4397–4421 rounds the hasted
    // period and rebuilds duration = that period × original tick count.
    // Independent duration/period truncation must not create a ninth event,
    // which would also earn full Ray Freezing/Splinter/Hand callbacks.
    assert_eq!(
        hits.len(),
        expected_ticks as usize,
        "constant-haste native Ray must retain its8 scheduled ticks, not add an expiry partial: haste{haste}, damage{hits:?}, fractions{fractions:?}"
    );
}
