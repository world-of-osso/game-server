//! Actual Judgment impacts produce native target uses, not an unconditional amplifier.
use super::single_spell::{SpellProbe, capture_cast};
use super::*;
use crate::auras::ActiveAuras;

const JUDGMENT: u32 = 20271;
const MARK: u32 = 197277;
const VERDICT: u32 = 383328;

fn mark_stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.targets[0])
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == MARK && aura.caster == run.actor)
        })
        .map_or(0, |aura| aura.stacks)
}

fn capture_verdict(run: &mut Run) -> f64 {
    let receipt = capture_cast(
        run,
        &SpellProbe {
            spec: 70,
            spell: VERDICT,
            damage_spell: VERDICT,
            action: "final_verdict",
            ticks: 1,
        },
    )
    .expect("actual naturally funded Verdict");
    receipt["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "result" && event["spell_id"] == VERDICT)
        .expect("native Verdict result")
        ["factors"]["target_product"]
        .as_f64()
        .unwrap()
}

#[test]
#[ignore = "native Ret Judgment target uses need exact private profile/traits"]
fn ret_judgment_native_impacts_grant_uses_with_one_flat_damage_bonus() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(70, 1).expect("native Ret profile");
    engage(&mut run);
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
    .expect("actual native Blade funds initial Verdict");
    // Establish actual unmarked damage before the Judgment producers.
    let unmarked = capture_verdict(&mut run);
    assert_eq!(mark_stacks(&run), 0);
    for expected in [1, 2] {
        ret_tier_tests::cast_finisher(&mut run, JUDGMENT);
        for _ in 0..20 {
            run.app.update();
        }
        assert_eq!(
            mark_stacks(&run),
            expected,
            "successful ordinary Judgment must grant a native owned target use"
        );
        assert!(
            !crate::auras::has_aura(run.app.world(), run.actor, MARK),
            "the target debuff never belongs on its caster"
        );
    }
    let native_bonus = crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        run.actor,
        MARK,
        0,
    ) as f64
        / 100.0;
    for remaining in [1, 0] {
        let marked = capture_verdict(&mut run);
        assert!(
            (marked / unmarked - (1.0 + native_bonus)).abs() < 0.001,
            "each marked Verdict gets one native bonus regardless of stored uses: {marked}/{unmarked}, native{native_bonus}"
        );
        assert_eq!(
            mark_stacks(&run),
            remaining,
            "each successful affected Verdict spends exactly one target use"
        );
    }
}
