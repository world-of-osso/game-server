//! Native Chaos damage and a pure Physical control share the real target armor intervention.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn damage_total(run: &Run, spell: u32) -> i32 {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| event.amount)
        .sum()
}

fn native_chaos_and_physical_hits(remove_armor: bool) -> (i32, i32) {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 1).unwrap();
    enhancement_armor_tests::apply_armor_control(&mut run, remove_armor);
    let target = run.targets[0];
    run.app.world_mut().insert_resource(FactorCapture::new(
        run.actor,
        target,
        393054,
        "armor_control",
        4,
    ));
    let _damage_seed = crate::random::seed_scope(818181);
    for spell in [393054, 198813] {
        crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, spell, Some(target), 1);
    }
    let hits = (damage_total(&run, 393054), damage_total(&run, 198813));
    enhancement_armor_tests::assert_armor_response(&run, &[393054], false);
    enhancement_armor_tests::assert_armor_response(&run, &[198813], !remove_armor);
    println!("HAVOC_ARMOR armorless={remove_armor} chaos_physical={hits:?}");
    assert!(
        hits.0 > 0 && hits.1 > 0,
        "both native school payloads must really land: {hits:?}"
    );
    hits
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn havoc_chaos_native_child_ignores_armor_but_physical_damage_retains_it() {
    let armored = native_chaos_and_physical_hits(false);
    let unarmored = native_chaos_and_physical_hits(true);
    // SimC76f8118 action.cpp1686–1690: each actual Chaos hit retains its
    // own native pre-mitigation amount, even if the control cast procs gear.
    assert!(
        armored.1 < unarmored.1,
        "native pure Physical198813 must still take armor mitigation"
    );
}
