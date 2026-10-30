//! After the Wildfire retains paid Rage across its native threshold.
use super::*;
use shared::{
    components::{Health, PowerType},
    protocol::CombatLogKind,
};

#[test]
#[ignore = "requires Guardian selected talent and native world data"]
fn guardian_after_the_wildfire_heals_on_300_rage_and_retains_overflow() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), actor, 371905));
    let max = run.app.world().get::<Health>(actor).unwrap().max;
    run.app
        .world_mut()
        .get_mut::<Health>(actor)
        .unwrap()
        .current = max * 0.5;
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    let spend = |run: &mut Run| {
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            PowerType::Rage,
            1000.0,
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 192081, Some(actor), false)
            .unwrap();
        run.app.update();
    };
    let heals = |run: &Run, spell| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Heal
                    && event.target == Some(actor.to_bits())
                    && event.amount > 0
            })
            .count()
    };
    for _ in 0..7 {
        spend(&mut run);
    }
    assert_eq!(
        heals(&run, 371982),
        0,
        "280 Rage is below the native threshold"
    );
    spend(&mut run);
    assert_eq!(heals(&run, 371982), 1, "320 Rage triggers the first heal");
    assert_eq!(heals(&run, 227034), 1, "mastery follows the native heal");
    for _ in 0..7 {
        spend(&mut run);
    }
    assert_eq!(
        heals(&run, 371982),
        2,
        "the retained 20 Rage triggers again at 600 total"
    );
}
