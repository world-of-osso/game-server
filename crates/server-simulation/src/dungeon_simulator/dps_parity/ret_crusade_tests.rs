//! SimC76f8118 Crusade: real paid/free spenders, unchanged base Wrath effects.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::effects::factor_capture::FactorCapture;

const WRATH: u32 = 31884;
const VERDICT: u32 = 383328;

fn wrath(run: &Run) -> &crate::auras::AuraInstance {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == WRATH && aura.caster == run.actor)
        .unwrap()
}

#[test]
#[ignore = "native Ret Crusade paid/free spender and derived-stat boundary"]
fn ret_crusade_paid_and_free_spenders_reach_ten_without_scaling_wrath_base_effects() {
    let (profile, traits) = read_reference(70).unwrap();
    let mut run = prepare_literal_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    engage(&mut run);
    ret_purpose_tests::earn_finisher_power(&mut run, VERDICT);
    // Isolate Crusade from the generator-only Rush of Light haste buff.
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 407065,
    );
    for _ in 0..4 {
        run.app.update();
    }
    let base_haste = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .haste_pct;
    let base_crit = run
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(run.actor)
        .unwrap()
        .npc_crit_pct;
    ret_tier_tests::cast_finisher(&mut run, WRATH);
    assert_eq!((wrath(&run).stacks, wrath(&run).max_stacks), (1, 10));
    let initial_expiry = wrath(&run).expires_at;
    let base_wrath_crit = wrath(&run)
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .unwrap()
        .amount;
    let base_wrath_damage = wrath(&run)
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    let per_stack = 2.0;
    // cpp3904–3911 also enables native Wrath effect11's flat haste for Crusade.
    let base_wrath_haste = wrath(&run)
        .effects
        .iter()
        .find(|effect| effect.index == 10)
        .unwrap()
        .amount;
    let hasted = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .haste_pct;
    let expected =
        ((1.0 + (base_haste + base_wrath_haste) / 100.0) * (1.0 + per_stack / 100.0) - 1.0) * 100.0;
    assert!(
        (hasted - expected).abs() < 0.01,
        "initial Crusade haste must multiply the existing speed: {hasted} vs{expected}"
    );

    ret_tier_tests::cast_finisher(&mut run, VERDICT);
    assert_eq!(
        wrath(&run).stacks,
        4,
        "paid3HP gives3 stacks; Legacy background Storm gives none"
    );
    for expected in [7, 10] {
        ret_tier_tests::grant_native_purpose(&mut run);
        let before = ret_purpose_tests::current_holy_power(&run);
        ret_tier_tests::cast_finisher(&mut run, VERDICT);
        assert_eq!(
            ret_purpose_tests::current_holy_power(&run),
            before,
            "free spender must not invent or spend HP"
        );
        assert_eq!(
            wrath(&run).stacks,
            expected,
            "free3HP-equivalent spender must count toward Crusade"
        );
    }
    assert_eq!(
        wrath(&run).expires_at,
        initial_expiry,
        "stack gains must not refresh Wrath"
    );
    let crit = run
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(run.actor)
        .unwrap()
        .npc_crit_pct;
    assert!(
        (crit - base_crit - base_wrath_crit).abs() < 0.01,
        "base Wrath crit must apply its native amount once; before={base_crit} after={crit}, native={base_wrath_crit}"
    );

    run.app.world_mut().insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        VERDICT,
        "final_verdict",
        1,
    ));
    ret_tier_tests::grant_native_purpose(&mut run);
    ret_tier_tests::cast_finisher(&mut run, VERDICT);
    assert_eq!(wrath(&run).stacks, 10, "Crusade cap is10");
    let capture = run.app.world().resource::<FactorCapture>().export();
    let wrath_damage = capture["modifiers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source_spell_id"] == WRATH && row["op"] == 0)
        .expect("actual Wrath damage modifier");
    assert_eq!(
        wrath_damage["value"], base_wrath_damage,
        "base Wrath damage must ignore Crusade stack count"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, WRATH, Some(run.actor), 1);
    assert_eq!(wrath(&run).stacks, 10);
    assert_eq!(
        wrath(&run).expires_at,
        initial_expiry,
        "active Crusade cannot be refreshed by another Wrath application"
    );
}
