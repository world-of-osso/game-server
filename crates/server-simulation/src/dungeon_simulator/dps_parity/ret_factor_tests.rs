//! Native Verdict factors from real casts and the ordinary full-fight rotation.
use super::single_spell::{SpellProbe, capture_cast};
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "native Ret FV factor comparison needs exact private profile/traits"]
fn dps_parity_ret_factor_verdict_records_base_and_wrath_source_stages() {
    for wings in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare(70, 1).expect("actual Ret follower profile");
        assert!(
            run.app
                .world()
                .get::<crate::followers::Follower>(run.actor)
                .is_some()
        );
        engage(&mut run);
        if wings {
            ret_tier_tests::cast_finisher(&mut run, 31884);
        }
        capture_cast(
            &mut run,
            &SpellProbe {
                spec: 70,
                spell: 184575,
                damage_spell: 184575,
                action: "blade_of_justice",
                ticks: 1,
            },
        )
        .expect("actual native Blade priming");
        let mut receipt = capture_cast(
            &mut run,
            &SpellProbe {
                spec: 70,
                spell: 383328,
                damage_spell: 383328,
                action: "final_verdict",
                ticks: 1,
            },
        )
        .expect("actual naturally funded Final Verdict");
        receipt["priming"] = json!({"blade_of_justice": 1, "ordinary_avenging_wrath": wings});
        assert_eq!(receipt["casts"], 1);
        assert_eq!(receipt["captured_hits"], 1);
        let formula = receipt["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 383328)
            .expect("actual native Final Verdict formula");
        assert!(formula["factors"]["a_power"].as_f64().unwrap() > 0.0);
        assert!(formula["factors"]["a_mod"].as_f64().unwrap() > 0.0);
        let mode = if wings { "wrath" } else { "base" };
        write_verdict_receipt(mode, &receipt);
    }
}

#[test]
#[ignore = "native Ret300s FV multiplier timeline needs exact private profile/traits"]
fn dps_parity_ret_factor_verdict_records_full_fight_multiplier_timeline() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(70, 1).expect("actual Ret follower measurement");
    let before = report::stats(run.app.world(), run.actor);
    run.app.world_mut().insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        383328,
        "final_verdict",
        1000,
    ));
    engage(&mut run);
    let mut cursor = 0;
    let mut contexts = Vec::new();
    let (seconds, uptimes, _) = advance_combat_observed(&mut run, |run| {
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        let verdict_landed = events[cursor..].iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(383328)
                && event.kind == CombatLogKind::Damage
        });
        cursor = events.len();
        if verdict_landed {
            contexts.push(snapshot_verdict_frame(run));
        }
    })
    .expect("ordinary300s native rotation");
    let combat = report::combat(run.app.world(), &run.damage_targets, seconds);
    let verdict = combat["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["spell_id"] == 383328)
        .expect("ordinary rotation casts Final Verdict");
    let mut receipt = run.app.world().resource::<FactorCapture>().export();
    assert!((seconds - 300.0).abs() < 0.001);
    assert_eq!(receipt["captured_hits"], verdict["hits"]);
    assert!(!contexts.is_empty());
    receipt["simulated_seconds"] = json!(seconds);
    receipt["seed"] = json!(70448);
    receipt["stats_before"] = before;
    receipt["stats_after"] = report::stats(run.app.world(), run.actor);
    receipt["buff_uptime_pct"] = json!(uptimes);
    receipt["post_impact_frame_contexts"] = json!(contexts);
    receipt["combat"] = combat;
    write_verdict_receipt("fight", &receipt);
}

fn snapshot_verdict_frame(run: &Run) -> Value {
    let world = run.app.world();
    let auras = world
        .get::<ActiveAuras>(run.actor)
        .expect("native Ret actor auras")
        .iter()
        .map(|aura| {
            json!({"spell_id": aura.spell_id, "stacks": aura.stacks,
                "effects": aura.effects.iter().map(|effect| json!({"index": effect.index,
                    "aura_type": effect.aura_type, "amount": effect.amount})).collect::<Vec<_>>()})
        })
        .collect::<Vec<_>>();
    json!({"observed_frame_time": crate::spell_cast::now(world),
        "stats": report::stats(world, run.actor), "auras": auras})
}

fn write_verdict_receipt(mode: &str, receipt: &Value) {
    let path = inputs::directory()
        .unwrap()
        .join(format!("70.final_verdict.{mode}.native.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(receipt).unwrap()).unwrap();
    eprintln!("RET_FV_FACTORS mode={mode} path={}", path.display());
}
