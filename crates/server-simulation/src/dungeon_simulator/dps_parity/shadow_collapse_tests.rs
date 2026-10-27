//! SimC76f8118 sc_priest.cpp1557–1595/1640–1650/3564–3571/3594–3598;
//! sc_priest_shadow.cpp1329–1331: actual Rift end splits, ordinary Madness grows it.
use super::literal_rotation::execution::{self, Execution};
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::{CastFailure, read_native_cast_start, test_harness::ObservedCombatEvents};
use crate::spell_info::SpellStore;
use shared::protocol::CombatLogKind;

fn cast_when_ready(run: &mut Run, id: u32) {
    let native = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(id)
        .unwrap()
        .clone();
    for _ in 0..400 {
        let ready = read_native_cast_start(run.app.world_mut(), run.actor, &native, run.targets[0]);
        match ready {
            Ok(_) => {
                let action = Execution::Spell(id, run.targets[0], None);
                execution::execute(run.app.world_mut(), run.actor, action).unwrap();
                return;
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::OnCooldown
                | CastFailure::NoCharges
                | CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("ordinary native spell{id}: {reason:?}"),
        }
    }
    panic!("native spell{id} never became ready");
}

fn enter_shadowform(run: &mut Run) {
    let actor = run.actor;
    execution::execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(232698, actor, None),
    )
    .expect("ordinary profile precombat Shadowform");
    assert!(crate::auras::has_aura(run.app.world(), actor, 232698));
}

fn finish_cast(run: &mut Run) {
    for _ in 0..240 {
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none()
        {
            return;
        }
        run.app.update();
    }
    panic!("ordinary cast must finish at native timing");
}

fn read_factor(run: &Run, spell: u32, event: &str, name: &str) -> Option<f64> {
    let capture = run.app.world().resource::<FactorCapture>().export();
    let row = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["event"] == event && row["spell_id"] == spell)?;
    Some(
        row["factors"][name]
            .as_f64()
            .unwrap_or_else(|| panic!("actual native factor {event}/{name} missing: {row}")),
    )
}

fn owns_rift(run: &Run) -> bool {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .any(|aura| aura.caster == run.actor && aura.spell_id == 459314)
}

fn advance_to_rift_expiry(run: &mut Run) {
    for _ in 0..400 {
        if !owns_rift(run) {
            break;
        }
        run.app.update();
    }
    assert!(!owns_rift(run), "real Rift reaches its finite expiry");
}

fn burst_hits(run: &Run, target: Entity) -> usize {
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    events
        .iter()
        .filter(|event| {
            let source = event.source == Some(run.actor.to_bits());
            let recipient = event.target == Some(target.to_bits());
            let damage = event.kind == CombatLogKind::Damage && event.amount > 0;
            source && recipient && damage && event.spell_id == Some(448405)
        })
        .count()
}

fn run_burst_fixture(count: usize) -> f64 {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(258, count).unwrap();
    enter_shadowform(&mut run);
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 448403));
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        448405,
        "collapsing_void",
        1,
    ));
    cast_when_ready(&mut run, 263165);
    assert!(owns_rift(&run), "ordinary Torrent starts real Rift");
    advance_to_rift_expiry(&mut run);
    for &target in &run.targets {
        assert_eq!(
            burst_hits(&run, target),
            1,
            "actual Rift expiry emits one native Collapse per nearby enemy"
        );
    }
    let amount = read_factor(&run, 448405, "result", "raw_amount").unwrap();
    let power = read_factor(&run, 448405, "direct_amount", "s_power").unwrap();
    let versatility = read_factor(&run, 448405, "result", "versatility").unwrap();
    assert!(power > 0.0 && versatility >= 1.0);
    amount / (power * versatility)
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_collapse_actual_rift_end_bursts_once_and_splits_across_three_targets() {
    // Real native observed amounts, normalized only for crit, live SP and Vers.
    let single = run_burst_fixture(1);
    let three = run_burst_fixture(3);
    let fraction = three / single;
    assert!(
        (fraction - 1.0 / 3.0).abs() < 0.1,
        "actual primary damage splits across the real pack: single={single}, three={three}, fraction={fraction}"
    );
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_collapse_ordinary_funded_madness_grows_actual_rift_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(258, 1).unwrap();
    enter_shadowform(&mut run);
    engage(&mut run);
    // Three real Slam gains fund the ordinary spender; never assign a pool.
    for _ in 0..3 {
        cast_when_ready(&mut run, 1227280);
        finish_cast(&mut run);
    }
    cast_when_ready(&mut run, 589);
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        447448,
        "entropic_rift",
        1,
    ));
    cast_when_ready(&mut run, 263165);
    finish_cast(&mut run);
    let before =
        read_factor(&run, 447448, "result", "source_product").expect("real pre-growth Rift pulse");
    assert!(owns_rift(&run));
    cast_when_ready(&mut run, 335467);
    finish_cast(&mut run);
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        447448,
        "entropic_rift",
        1,
    ));
    for _ in 0..80 {
        if read_factor(&run, 447448, "result", "source_product").is_some() {
            break;
        }
        run.app.update();
    }
    let after = read_factor(&run, 447448, "result", "source_product")
        .expect("real post-Madness Rift pulse");
    let percent =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 448403, 2);
    let expected = 1.0 + f64::from(percent) / 100.0;
    assert!(
        (after / before - expected).abs() < 0.005,
        "ordinary Madness adds the native Rift damage bonus: before={before}, after={after}, expected={expected}"
    );
}
