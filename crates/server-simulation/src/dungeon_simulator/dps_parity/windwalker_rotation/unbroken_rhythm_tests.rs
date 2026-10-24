//! Earned native Unbroken Rhythm; no aura, stack, power, stat or coefficient grants.
use super::*;
use crate::auras::{ActiveAuras, AuraInstance};
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::{CombatLogEvent, CombatLogKind};

type Run = super::super::Run;
const ROOT: u32 = 101546;
const TICK: u32 = 107270;
const PRODUCER: u32 = 1296624;
const BUFF: u32 = 1297033;

fn prepare() -> Run {
    let mut run = super::super::prepare_measurement(269, 1).unwrap();
    let started = run.app.world().resource::<Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, started);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, PRODUCER));
    // Actual Tiger Palm Chi and the existing native aura42 producer, not fixture stacks.
    for _ in 0..3 {
        super::tests::cast(&mut run, 100780);
    }
    assert!(
        buff(&run).is_some(),
        "actual TP impacts must earn native1297033"
    );
    assert!(
        events(&run)
            .iter()
            .any(|event| event.spell_id == Some(100780)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    run
}

fn buff(run: &Run) -> Option<AuraInstance> {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| aura.spell_id == BUFF && aura.caster == run.actor)
        .cloned()
}

fn multiplier(run: &Run) -> f64 {
    buff(run).map_or(1.0, |aura| {
        let percent = aura
            .effects
            .iter()
            .find(|effect| effect.index == 1)
            .unwrap()
            .amount;
        assert_eq!(
            percent, 20.0,
            "CURRENT native effect1, not a copied coefficient"
        );
        1.0 + f64::from(aura.stacks) * f64::from(percent) / 100.0
    })
}

fn events(run: &Run) -> Vec<CombatLogEvent> {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.source == Some(run.actor.to_bits()))
        .cloned()
        .collect()
}

fn tick_damage(run: &Run) -> Vec<CombatLogEvent> {
    events(run)
        .into_iter()
        .filter(|event| {
            event.spell_id == Some(TICK)
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
        })
        .collect()
}

/// Observe exactly the accepted parent boundary, before its first native frame/tick.
fn cast_parent(run: &mut Run) -> f64 {
    use crate::spell_cast::CastFailure;
    for _ in 0..1200 {
        let before = multiplier(run);
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            ROOT,
            Some(run.targets[0]),
            false,
        ) {
            Ok(()) => return before,
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::InProgress
                | CastFailure::NotEnoughPower(_),
            ) => run.app.update(),
            Err(reason) => panic!("ordinary native SCK refused: {reason:?}"),
        }
    }
    panic!("native SCK never became ready through its actual resource/GCD state");
}

/// Finish the actual channel/aura; retain native producer changes seen on real damage frames.
fn finish_parent(run: &mut Run) -> Vec<u8> {
    let mut states = Vec::new();
    let mut damage = tick_damage(run).len();
    for _ in 0..200 {
        run.app.update();
        let current = tick_damage(run).len();
        if current > damage {
            states.push(buff(run).map_or(0, |aura| aura.stacks));
            damage = current;
        }
        if !crate::auras::has_aura(run.app.world(), run.actor, ROOT) {
            return states;
        }
    }
    panic!("actual native SCK aura did not finish within10s");
}

fn wait_for_natural_buff_expiry(run: &mut Run) {
    for _ in 0..1200 {
        if buff(run).is_none() {
            return;
        }
        run.app.update();
    }
    panic!("native1297033 failed to expire without further source actions");
}

fn capture_ticks(run: &mut Run) {
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        TICK,
        "spinning_crane_kick_tick",
        64,
    ));
}

/// Check resolved damage from its observed native inputs and unaffected source/target
/// products. This allows script or cast-effectiveness placement, and never rerolls a hit.
fn assert_tick_damage(run: &Run, expected_bonus: f64, before_hits: usize) -> Value {
    let exported = run.app.world().resource::<FactorCapture>().export();
    let rows: Vec<_> = exported["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["event"] == "result" && row["spell_id"] == TICK)
        .collect();
    let actual = tick_damage(run);
    let actual = &actual[before_hits..];
    assert!(
        rows.len() >= 2,
        "multiple actual native ticks must be observed"
    );
    assert_eq!(
        rows.len(),
        actual.len(),
        "every real damage tick, not a selected subset"
    );
    for (row, event) in rows.iter().zip(actual) {
        let factors = &row["factors"];
        let number = |key: &str| factors[key].as_f64().unwrap();
        let expected = number("native_input_amount")
            * number("native_spell_mod_product")
            * number("native_done_product")
            * number("target_product")
            * expected_bonus;
        let resolved = number("native_rounded_amount");
        assert!(
            (resolved - expected.round()).abs() <= 1.0,
            "native107270 snapshot bonus{expected_bonus}: resolved{resolved}, expected{}; {row}",
            expected.round()
        );
        assert_eq!(
            factors["actual_amount"].as_i64().unwrap(),
            i64::from(event.amount)
        );
        assert!(
            event.amount > 0,
            "real native damage, not a calculation-only substitute"
        );
    }
    exported
}

fn write_factor_receipt(label: &str, factors: &Value) {
    let path = super::super::inputs::directory()
        .unwrap()
        .join(format!("{label}.factors.json"));
    std::fs::write(path, serde_json::to_vec_pretty(factors).unwrap()).unwrap();
}

#[test]
#[ignore = "actual earned native1297033 is consumed once at the SCK parent, producer remains native"]
fn literal_ww_native_sck_snapshot_consumes_earned_unbroken_once() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare();
    let removed_before = events(&run)
        .iter()
        .filter(|event| event.spell_id == Some(BUFF) && event.kind == CombatLogKind::AuraRemoved)
        .count();
    let expected = cast_parent(&mut run);
    assert!(expected > 1.0, "earned native buff at the accepted parent");
    assert!(
        buff(&run).is_none(),
        "SCK must consume its earned1297033 at parent execute, before ticks"
    );
    let removed = events(&run)
        .iter()
        .filter(|event| event.spell_id == Some(BUFF) && event.kind == CombatLogKind::AuraRemoved)
        .count();
    assert_eq!(
        removed,
        removed_before + 1,
        "one ordinary native consumption"
    );
    let live = finish_parent(&mut run);
    assert!(
        live.iter().any(|&stacks| stacks > 0),
        "native aura42/CAN_PROC_FROM_PROCS must still earn a new buff from real ticks"
    );
    assert!(crate::auras::has_aura(run.app.world(), run.actor, PRODUCER));
}

#[test]
#[ignore = "snapshot actual native1297033 stacks/effect1 across all real107270 damage ticks"]
fn literal_ww_native_sck_snapshot_earned_bonus_is_fixed_across_every_tick() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare();
    capture_ticks(&mut run);
    let before = tick_damage(&run).len();
    let expected = cast_parent(&mut run);
    assert!(expected > 1.0);
    let original_stacks = ((expected - 1.0) / 0.2).round() as u8;
    let live = finish_parent(&mut run);
    assert!(!live.is_empty());
    assert!(
        live.iter().any(|&stacks| stacks != original_stacks),
        "live native producer state must differ from the parent's frozen earned amount"
    );
    let factors = assert_tick_damage(&run, expected, before);
    write_factor_receipt("sck-earned-snapshot", &factors);
}

#[test]
#[ignore = "no-earned parent stays unboosted even as its native ticks legitimately earn a new buff"]
fn literal_ww_native_sck_snapshot_no_earned_buff_does_not_read_later_live_stacks() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare();
    wait_for_natural_buff_expiry(&mut run);
    capture_ticks(&mut run);
    let before = tick_damage(&run).len();
    assert_eq!(
        cast_parent(&mut run),
        1.0,
        "no earned buff at accepted parent"
    );
    let live = finish_parent(&mut run);
    assert!(
        live.iter().any(|&stacks| stacks > 0),
        "unchanged native producer control"
    );
    let factors = assert_tick_damage(&run, 1.0, before);
    write_factor_receipt("sck-no-earned-snapshot", &factors);
}

#[test]
#[ignore = "a later native SCK parent must not reuse an earlier parent's earned snapshot"]
fn literal_ww_native_sck_snapshot_later_parent_does_not_reuse_old_bonus() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare();
    assert!(cast_parent(&mut run) > 1.0);
    finish_parent(&mut run);
    wait_for_natural_buff_expiry(&mut run);
    capture_ticks(&mut run);
    let before = tick_damage(&run).len();
    assert_eq!(
        cast_parent(&mut run),
        1.0,
        "later parent has no earned buff"
    );
    finish_parent(&mut run);
    let factors = assert_tick_damage(&run, 1.0, before);
    write_factor_receipt("sck-later-parent-snapshot", &factors);
}
