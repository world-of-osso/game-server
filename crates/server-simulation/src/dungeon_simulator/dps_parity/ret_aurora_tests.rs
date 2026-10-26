//! SimC76f8118 Ret878–881: real Wake guarantees the selected Aurora Purpose grant.
use super::ret_tier_tests::{cast_finisher, has, native_ret};
use super::*;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const AURORA: u32 = 439760;

pub(super) fn unselect_aurora(run: &mut Run) {
    let (_, mut traits) = read_reference(70).unwrap();
    let classes = run.app.world().resource::<crate::spell_info::ClassSpells>();
    let tree = &classes
        .classes
        .iter()
        .find(|class| class.class_id == 2)
        .unwrap()
        .traits;
    traits.retain(|allocated| {
        tree.node(allocated.node_id)
            .unwrap()
            .entry(allocated.entry_id)
            .unwrap()
            .spell_id
            != AURORA
    });
    inputs::install_traits(run.app.world_mut(), run.actor, &traits).unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert!(!has(run, AURORA), "native reconciliation removes unselected Aurora");
}

#[test]
#[ignore = "legal unspent Aurora retains ordinary Wake without a guaranteed Purpose"]
fn ret_aurora_native_unselected_wake_does_not_grant_purpose() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    unselect_aurora(&mut run);
    cast_finisher(&mut run, 255937);
    assert!(ret_purpose_tests::current_holy_power(&run) >= 3.0);
    assert!(!has(&run, 408458));
}

#[test]
#[ignore = "native selected Aurora and MID2 Wake→Purpose→Arbiter supply"]
fn ret_aurora_native_wake_grants_purpose_and_supplies_real_arbiter_payout() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert!(
        run.app
            .world()
            .get::<TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|spell| spell.spell_id == 439760)
    );
    for spell in [408458, 1306161, 1306162, 1310461] {
        assert!(
            !has(&run, spell),
            "native setup must not start with Purpose/Arbiter uses: {spell}"
        );
    }
    cast_finisher(&mut run, 255937);
    assert!(
        has(&run, 408458),
        "selected Aurora must grant native Divine Purpose after an ordinary Wake, without a random spender roll"
    );
    let funded = ret_purpose_tests::current_holy_power(&run);
    assert!(funded >= 3.0, "actual Wake supplied native Holy Power");
    cast_finisher(&mut run, 383328);
    assert_eq!(
        ret_purpose_tests::current_holy_power(&run),
        funded,
        "the guaranteed Purpose makes the actual Verdict free"
    );
    assert!(
        has(&run, 1306161),
        "the actual Purpose consumer arms native Verdict Arbiter"
    );
    cast_finisher(&mut run, 53385);
    assert!(
        !has(&run, 1306161),
        "actual opposite-kind Storm consumes the armed use"
    );
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1306923)
                && event.kind == CombatLogKind::Damage
        })
        .count();
    assert_eq!(
        hits, 1,
        "the armed native use produces a real Arbiter primary hit"
    );
}
