//! Actual imbues must supply Elemental Weapons' native dynamic direct modifier.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use std::time::Duration;

const ELEMENTAL_WEAPONS: u32 = 384355;
const BOLT: u32 = 188196;
const IMBUES: &[(u32, u32)] = &[(33757, 319773), (318038, 319778)];

struct Probe {
    damage: f64,
    inputs: Value,
    bonus_per_hand: f64,
}

fn prepare_imbued_actor(count: usize) -> Run {
    let mut run = prepare_literal(263, 1).unwrap();
    let actor = run.actor;
    for &(spell, _) in IMBUES.iter().take(count) {
        enhancement_generation_tests::cast(&mut run, spell, actor);
    }
    advance_idle_clock(&mut run);
    for (index, &(_, aura)) in IMBUES.iter().enumerate() {
        assert_eq!(
            crate::auras::has_aura(run.app.world(), actor, aura),
            index < count
        );
    }
    engage(&mut run);
    run.app
        .world_mut()
        .entity_mut(actor)
        .insert(crate::combat::AutoAttack {
            timer: Duration::from_secs(60),
            off_hand_timer: Duration::from_secs(60),
            victim: run.targets[0],
        });
    run
}

fn advance_idle_clock(run: &mut Run) {
    // Equal native clock time, without injecting resources or changing proc chances.
    for _ in 0..600 {
        if crate::spell_cast::now(run.app.world()) >= 10.0 {
            return;
        }
        run.app.update();
    }
    panic!("native idle clock did not reach the control's ten seconds");
}

fn read_bolt_result(run: &Run) -> Option<Value> {
    let capture = run.app.world().resource::<FactorCapture>().export();
    capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "result" && event["spell_id"] == BOLT)
        .cloned()
}

fn cast_and_capture_bolt(run: &mut Run) -> Value {
    let actor = run.actor;
    let target = run.targets[0];
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 187881),
        "the control must not use Maelstrom Weapon stacks"
    );
    run.app.insert_resource(FactorCapture::new(
        actor,
        target,
        BOLT,
        "elemental_weapons",
        2,
    ));
    let _damage_seed = crate::random::seed_scope(818181);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, BOLT, Some(target), 1);
    for _ in 0..40 {
        if let Some(hit) = read_bolt_result(run) {
            return hit;
        }
        run.app.update();
    }
    panic!("actual native Lightning Bolt must land");
}

fn native_bonus_per_hand(run: &Run) -> f64 {
    // SimC76f8118 sc_shaman.cpp13212–13220: talent effect1.percent()*0.1 per imbued hand.
    let talent = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(ELEMENTAL_WEAPONS)
        .unwrap();
    let amount = talent
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    f64::from(amount) / 100.0 * 0.1
}

fn observed_bolt_inputs(hit: &Value) -> Value {
    let factors = &hit["factors"];
    let mastery = hit["factor_sources"]["source_product"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| source["source_spell_id"] == 77223 && source["native_effect_index"] == 0)
        .expect("actual Enhanced Elements modifier")["value"]
        .clone();
    json!({"roll": factors["native_input_amount"], "crit": factors["crit_bonus"],
        "versatility": factors["versatility"], "mastery": mastery, "target": factors["target_product"]})
}

fn cast_imbued_bolt(count: usize) -> Probe {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_imbued_actor(count);
    assert!(crate::auras::has_aura(
        run.app.world(),
        run.actor,
        ELEMENTAL_WEAPONS
    ));
    let bonus_per_hand = native_bonus_per_hand(&run);
    let hit = cast_and_capture_bolt(&mut run);
    let inputs = observed_bolt_inputs(&hit);
    let damage = hit["factors"]["actual_amount"].as_f64().unwrap();
    assert!(damage > 0.0, "native Bolt must deal actual damage");
    println!(
        "ELEMENTAL_WEAPONS hands={count} damage={damage} per_hand={bonus_per_hand} inputs={inputs}"
    );
    Probe {
        damage,
        inputs,
        bonus_per_hand,
    }
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_elemental_weapons_real_imbues_increase_actual_bolt_damage_per_hand() {
    let baseline = cast_imbued_bolt(0);
    for hands in [1, 2] {
        let actual = cast_imbued_bolt(hands);
        assert_eq!(
            actual.inputs, baseline.inputs,
            "native roll/crit/Mastery/Vers/recipient inputs must match before comparing damage"
        );
        let expected = baseline.damage * (1.0 + actual.bonus_per_hand * hands as f64);
        assert!(
            (actual.damage - expected).abs() <= 2.0,
            "native imbued hands{hands}: damage{}, expected{expected}; base{}",
            actual.damage,
            baseline.damage
        );
    }
}
