//! Native70448 Wake of Ashes255937: real target damage and effect1 resource.
use super::ret_purpose_tests::current_holy_power;
use super::ret_tier_tests::native_ret;
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const WAKE: u32 = 255937;

#[test]
#[ignore = "native selected Wake normal cast must damage31146 and grant native effect1 HolyPower"]
fn dps_parity_ret_wake_native_damage_and_resource_share_real_cast() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert_eq!(
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, WAKE).unwrap(),
        Some(WAKE)
    );
    assert_eq!(current_holy_power(&run), 0.0);
    let native = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(WAKE)
        .unwrap();
    let resource = native
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .base_points;
    assert_eq!(
        resource, 3.0,
        "native effect1 is the source resource amount"
    );
    // SimC76f8118 Ret846–876: observe Wake independently of the
    // Crusading Strike resource producer (sc_paladin.cpp888–916).
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        WAKE,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    for _ in 0..100 {
        run.app.update();
        if run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(WAKE)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
        {
            break;
        }
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(WAKE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "ordinary Wake must produce its own native direct damage"
    );
    assert_eq!(current_holy_power(&run), resource);
}

#[test]
#[ignore = "native granted-only Ret must not acquire purchased Wake in literal knowledge"]
fn dps_parity_ret_wake_native_unselected_knowledge_does_not_admit_action() {
    let mut run = native_ret(4);
    inputs::install_traits(run.app.world_mut(), run.actor, &[]).unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert_eq!(
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, WAKE).unwrap(),
        None
    );
    assert_eq!(current_holy_power(&run), 0.0);
}
