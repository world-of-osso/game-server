//! Real Crash proc damage must retain the mean MID2 snapshot of its live Crash stacks.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use std::time::Duration;

const CRASH: u32 = 187874;
const CRASH_STACKS: u32 = 1252415;
const CRASH_DAMAGE: u32 = 195592;
const SHORT_CIRCUIT: u32 = 1299991;

struct Probe {
    damage: f64,
    inputs: Value,
    retained_bonus: f64,
}

fn short_circuit_bonus(run: &Run) -> f64 {
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .and_then(|active| {
            active
                .iter()
                .find(|aura| aura.caster == run.actor && aura.spell_id == SHORT_CIRCUIT)
        });
    let Some(aura) = aura else {
        return 0.0;
    };
    let amount = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    f64::from(amount) * f64::from(aura.stacks) / 100.0
}

fn prepare_probe_actor(voltaic_casts: usize) -> Run {
    let mut run = prepare_literal(263, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1296628));
    engage(&mut run);
    let target = run.targets[0];
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::combat::AutoAttack {
            timer: Duration::from_secs(60),
            off_hand_timer: Duration::from_secs(60),
            victim: target,
        });
    for _ in 0..voltaic_casts {
        enhancement_generation_tests::cast(&mut run, 470057, target);
    }
    for _ in 0..600 {
        if crate::spell_cast::now(run.app.world()) >= 10.0 {
            return run;
        }
        run.app.update();
    }
    panic!("native idle clock did not reach ten seconds");
}

fn cast_two_crash_roots(run: &mut Run) -> f64 {
    let first_bonus = short_circuit_bonus(run);
    let target = run.targets[0];
    enhancement_generation_tests::cast(run, CRASH, target);
    assert_eq!(
        short_circuit_bonus(run),
        0.0,
        "first real Crash consumes Short Circuit"
    );
    // Isolate overlapping real Crash applications: the ordinary cooldown can exceed
    // the first stack's 12-second lifetime before the helper admits a second cast.
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::spell_cast::SpellHistory::default());
    enhancement_generation_tests::cast(run, CRASH, target);
    assert_eq!(
        short_circuit_bonus(run),
        0.0,
        "no new Fire Nova follows the expired Burning Core"
    );
    let stacks = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == CRASH_STACKS && aura.caster == run.actor)
        .expect("real Crash stack aura")
        .stacks;
    assert_eq!(
        stacks, 2,
        "both native Crash roots must still own live stacks"
    );
    // SimC4564–4573: the second real root snapshots zero, so two live roots average this value.
    first_bonus / f64::from(stacks)
}

fn read_crash_hit(run: &Run) -> Option<Value> {
    run.app.world().resource::<FactorCapture>().export()["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "result" && event["spell_id"] == CRASH_DAMAGE)
        .cloned()
}

fn wait_for_strike_ready(run: &mut Run) {
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(17364)
        .cloned()
        .unwrap();
    for _ in 0..40 {
        match crate::spell_cast::read_native_cast_start(
            run.app.world_mut(),
            run.actor,
            &spell,
            run.targets[0],
        ) {
            Ok(_) => return,
            Err(crate::spell_cast::CastFailure::OnGlobalCooldown) => run.app.update(),
            result => panic!("native Stormstrike readiness: {result:?}"),
        }
    }
    panic!("native GCD did not clear before the matched damage roll");
}

fn cast_and_capture_crash(run: &mut Run) -> Value {
    // Drain real GCD events before matching the damage seed; a live Flame Shock may tick meanwhile.
    wait_for_strike_ready(run);
    let actor = run.actor;
    let target = run.targets[0];
    run.app.insert_resource(FactorCapture::new(
        actor,
        target,
        CRASH_DAMAGE,
        "crash_mid2",
        2,
    ));
    let _damage_seed = crate::random::seed_scope(818181);
    enhancement_generation_tests::cast(run, 17364, target);
    for _ in 0..40 {
        if let Some(hit) = read_crash_hit(run) {
            return hit;
        }
        run.app.update();
    }
    panic!("real Stormstrike must land its native Crash proc");
}

fn observed_damage_inputs(hit: &Value) -> Value {
    let factors = &hit["factors"];
    let mastery: Vec<_> = hit["factor_sources"]["source_product"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|source| source["source_spell_id"] == 77223 && source["native_effect_index"] == 0)
        .map(|source| source["value"].clone())
        .collect();
    json!({"roll": factors["native_input_amount"], "crit": factors["crit_bonus"],
        "versatility": factors["versatility"], "mastery": mastery, "target": factors["target_product"]})
}

fn cast_probe(voltaic_casts: usize) -> Probe {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_probe_actor(voltaic_casts);
    let retained_bonus = cast_two_crash_roots(&mut run);
    assert_eq!(
        retained_bonus > 0.0,
        voltaic_casts > 0,
        "only real Voltaic earns the native bonus"
    );
    let hit = cast_and_capture_crash(&mut run);
    let inputs = observed_damage_inputs(&hit);
    let damage = hit["factors"]["actual_amount"].as_f64().unwrap();
    assert!(damage > 0.0);
    println!(
        "CRASH_MID2 voltaic={voltaic_casts} retained={retained_bonus} damage={damage} inputs={inputs}"
    );
    Probe {
        damage,
        inputs,
        retained_bonus,
    }
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_crash_mid2_real_proc_uses_average_retained_short_circuit_after_consumption() {
    let baseline = cast_probe(0);
    let actual = cast_probe(1);
    assert_eq!(
        actual.inputs, baseline.inputs,
        "native roll/crit/Mastery/Vers/recipient inputs must match"
    );
    let expected = baseline.damage * (1.0 + actual.retained_bonus);
    assert!(
        (actual.damage - expected).abs() <= 2.0,
        "real retained MID2 average{}: damage{}, expected{expected}; baseline{}",
        actual.retained_bonus,
        actual.damage,
        baseline.damage
    );
}
