//! Owned pet periodic damage uses the owner's native spell haste.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use shared::protocol::CombatLogKind;

fn pet_periodic_hits(spell: u32, haste: f32) -> usize {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage(&mut run);
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .haste_pct = haste;
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        pet,
        spell,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..60 {
        run.app.update();
    }
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..]
        .iter()
        .filter(|event| {
            event.source == Some(pet.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.periodic
        })
        .count()
}

#[test]
#[ignore = "requires native Survival255 profile and world data"]
fn survival_pet_sic_em_ticks_follow_live_owner_haste() {
    // SimC76f8118 pet.cpp217–228 and action.cpp671: pet spell haste is
    // inherited from the owner; the native spell flag selects hasted ticks.
    let ordinary = pet_periodic_hits(1253138, 0.0);
    let hasted = pet_periodic_hits(1253138, 100.0);
    assert!((2..=3).contains(&ordinary), "native 1s Sic Em: {ordinary}");
    assert!((5..=6).contains(&hasted), "native 0.5s Sic Em: {hasted}");
    let unflagged = pet_periodic_hits(1253171, 0.0);
    let unflagged_hasted = pet_periodic_hits(1253171, 100.0);
    assert!(unflagged > 0, "control must really tick");
    assert_eq!(
        unflagged, unflagged_hasted,
        "unflagged periodic cadence is unchanged"
    );
}
