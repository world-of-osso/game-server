//! Ordinary native casts retain Nightfall's admission, damage and stacked-use boundaries.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;

const BOLT: u32 = 686;
const NIGHTFALL: u32 = 264571;

fn prepare_nightfall() -> Run {
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    run.app.world_mut().insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        BOLT,
        "shadow_bolt",
        16,
    ));
    run
}

fn grant_nightfall(run: &mut Run) {
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.actor,
            target: run.actor,
            spell_id: NIGHTFALL,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

pub(super) fn uses(run: &Run) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == NIGHTFALL))
        .map_or(0, |aura| aura.stacks)
}

fn start_bolt(run: &mut Run) {
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        BOLT,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
}

fn finish_bolt(run: &mut Run) {
    for _ in 0..70 {
        run.app.update();
    }
}

fn damage_products(run: &Run) -> Vec<f64> {
    let capture = run.app.world().resource::<FactorCapture>().export();
    capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["event"] == "result" && event["spell_id"] == BOLT)
        .map(|event| {
            event["factors"]["native_spell_mod_product"]
                .as_f64()
                .unwrap()
        })
        .collect()
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_nightfall_instant_bolt_retains_native_damage_after_consumption() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_nightfall();
    start_bolt(&mut run);
    finish_bolt(&mut run);
    grant_nightfall(&mut run);
    let percent =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, NIGHTFALL, 1);
    assert!(percent > 0.0);
    start_bolt(&mut run);
    assert_eq!(
        uses(&run),
        0,
        "the admitted use is spent before missile impact"
    );
    finish_bolt(&mut run);
    let products = damage_products(&run);
    assert_eq!(products.len(), 2, "both ordinary native bolts must land");
    let ratio = products[1] / products[0];
    let expected = 1.0 + f64::from(percent) / 100.0;
    // SimC76f8118 actions902–908: instant admitted Bolt snapshots one native bonus.
    assert!(
        (ratio - expected).abs() < 0.00001,
        "actual native instant damage product={ratio}, expected={expected}"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_nightfall_two_real_uses_require_two_ordinary_bolts() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_nightfall();
    grant_nightfall(&mut run);
    grant_nightfall(&mut run);
    assert_eq!(uses(&run), 2, "native cumulative aura admits two uses");
    start_bolt(&mut run);
    // SimC76f8118 actions877–878: decrement once, not the entire charged aura.
    assert_eq!(
        uses(&run),
        1,
        "one ordinary instant Bolt spends exactly one use"
    );
    finish_bolt(&mut run);
    assert_eq!(uses(&run), 1, "first impact cannot spend the next use");
    start_bolt(&mut run);
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none(),
        "the second admitted use must really make its Bolt instant"
    );
    assert_eq!(
        uses(&run),
        0,
        "second ordinary instant Bolt spends final use"
    );
    finish_bolt(&mut run);
    assert_eq!(damage_products(&run).len(), 2);
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_nightfall_gained_during_real_hardcast_is_retained_without_bonus() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_nightfall();
    start_bolt(&mut run);
    finish_bolt(&mut run);
    start_bolt(&mut run);
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_some(),
        "unbuffed ordinary Bolt must really be in flight as a hardcast"
    );
    grant_nightfall(&mut run);
    finish_bolt(&mut run);
    // SimC76f8118 actions877–878/902–908 use admission's time_to_execute.
    assert_eq!(
        uses(&run),
        1,
        "a mid-hardcast proc belongs to the next Bolt"
    );
    let products = damage_products(&run);
    assert_eq!(products.len(), 2);
    let ratio = products[1] / products[0];
    assert!(
        (ratio - 1.0).abs() < 0.00001,
        "the hardcast cannot borrow its newly gained Nightfall: {products:?}"
    );
}

fn cast_grasp_and_assert_clock(run: &mut Run, charged_factor: f32) {
    let world = run.app.world();
    let haste = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .haste_pct;
    let native = world
        .resource::<crate::spell_info::SpellStore>()
        .get(1261153)
        .unwrap();
    let base = native.misc.as_ref().unwrap().duration.unwrap().duration_ms as u32;
    let period = native
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .aura_period_ms;
    assert_eq!(base / period, 4, "native Grasp has four original ticks");
    // SimC76f8118 action.cpp4390–4432: quantize each cast's hasted period,
    // then rebuild its span. Other casts can have different native haste.
    let expected_period = (base as f32 * charged_factor / (1.0 + haste / 100.0) / 4.0).round();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        1261153,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    let world = run.app.world();
    let cast = world
        .get::<shared::casting::CastState>(run.actor)
        .expect("actual Grasp retains its nonzero channel duration");
    assert!(
        (cast.duration * 1000.0 - 4.0 * expected_period).abs() < 0.1,
        "channel span uses its own haste{haste}% and charged factor{charged_factor}: actual{}s expected{}ms",
        cast.duration,
        4.0 * expected_period
    );
    assert!(
        (cast.channel_tick_interval * 1000.0 - expected_period).abs() < 0.1,
        "channel bar period retains the admitted snapshot"
    );
    let aura = world
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1261153 && aura.caster == run.actor)
        .unwrap();
    let actual_period = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .period_ms;
    assert_eq!(
        actual_period, expected_period as u32,
        "damage aura period retains the same native snapshot as the channel bar"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_nightfall_channel_uses_do_not_multiply_duration_reduction() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_nightfall();
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 205180, Some(run.actor), 0);
    cast_grasp_and_assert_clock(&mut run, 1.0);
    finish_bolt(&mut run);
    grant_nightfall(&mut run);
    let percent =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, NIGHTFALL, 3);
    let factor = 1.0 + percent / 100.0;
    cast_grasp_and_assert_clock(&mut run, factor);
    assert_eq!(uses(&run), 0);
    finish_bolt(&mut run);
    grant_nightfall(&mut run);
    grant_nightfall(&mut run);
    cast_grasp_and_assert_clock(&mut run, factor);
    // SimC76f8118 actions199/2151–2163/2182: one boolean bonus and one consumed use.
    assert_eq!(uses(&run), 1, "a channel spends one of the two real uses");
    finish_bolt(&mut run);
    let hits = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let ticks = hits
        .iter()
        .filter(|event| {
            event.spell_id == Some(1261153)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(
        ticks, 12,
        "all three real channels retain all four native ticks"
    );
}
