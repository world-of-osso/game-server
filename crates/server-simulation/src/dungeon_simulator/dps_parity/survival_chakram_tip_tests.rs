//! Actual Moonlight Chakram spends earned Survival Tip and supplies its native buff.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_chakram_spends_earned_tip_and_grants_its_native_buff() {
    // SimC76f8118 sc_hunter.cpp4456–4476: actual Chakram spends one
    // earned Tip, grants native1280140 and does not roll a Sentinel mark.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    engage(&mut run);
    cast(&mut run, 1250646, false);
    let before = owned_tip(&run);
    assert!(before > 0, "actual selected Takedown must earn Tip");
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1264946),
        "actual Takedown must arm Chakram"
    );
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    cast(&mut run, 1264949, false);
    assert_eq!(
        owned_tip(&run),
        before - 1,
        "actual Chakram must spend one earned Tip"
    );
    let amount = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == 1280140)
        .map(|aura| aura.effects[0].amount);
    assert!(
        amount.is_some_and(|amount| amount > 0.0),
        "native Chakram Tip modifier must be live during bounces: {amount:?}"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.actor.to_bits())
                && event.spell_id == Some(1280140)
                && event.kind == CombatLogKind::AuraApplied),
        "actual tipped Chakram must earn its native buff"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1266081)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "actual Chakram must deal native bounce damage"
    );
}

fn owned_tip(run: &Run) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == 260286)
        .map_or(0, |aura| aura.stacks)
}
