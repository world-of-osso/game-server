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

#[test]
#[ignore = "requires native 70 player profile; selected Wake three-target damage"]
fn ret_wake_seething_flames_reduces_actual_secondary_impacts() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(70, 3).unwrap();
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(run.actor)
            .is_some()
    );
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 405355));
    super::channel_timing_tests::remove_equipped_proc_drivers(&mut run);
    engage(&mut run);
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 0.0;
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        WAKE,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    for _ in 0..60 {
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .crit_pct = 0.0;
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    for spell in [255937, 405345, 405350] {
        let hit = |target: Entity| {
            events
                .iter()
                .find(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(spell)
                        && event.kind == CombatLogKind::Damage
                })
                .unwrap_or_else(|| panic!("spell={spell} target={target:?} missing impact"))
        };
        let primary = hit(run.targets[0]);
        for &target in &run.targets[1..] {
            let secondary = hit(target);
            assert_eq!(
                primary.crit, secondary.crit,
                "compare matching real hit outcomes"
            );
            let ratio = secondary.amount as f64 / primary.amount as f64;
            assert!(
                (ratio - 0.6).abs() < 0.02,
                "spell={spell} secondary ratio {ratio}, primary={} secondary={}",
                primary.amount,
                secondary.amount
            );
        }
    }
}
