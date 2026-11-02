//! The follower spends Ravage through the ordinary Ferocious Bite action.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_bite_consumes_ravage_with_paid_resources() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 441585, Some(actor), 1);
    assert!(crate::auras::has_aura(app.world(), actor, 441585));
    let spell = crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 22568)
        .expect("Bite action resolves with Ravage");
    assert_eq!(spell, 441591, "Ravage replaces the ordinary Bite action");
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::ComboPoints, 5.0);
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
    let energy = crate::spell_cast::powers::power_value(app.world(), actor, PowerType::Energy)
        .unwrap()
        .0;
    crate::spell_cast::creature_cast(app.world_mut(), actor, spell, Some(target), false).unwrap();
    assert!(
        !crate::auras::has_aura(app.world(), actor, 441585),
        "Ravage use consumes its replacement aura"
    );
    let remaining = crate::spell_cast::powers::power_value(app.world(), actor, PowerType::Energy)
        .unwrap()
        .0;
    let points = crate::spell_cast::powers::power_value(app.world(), actor, PowerType::ComboPoints)
        .unwrap()
        .0;
    assert!(
        (energy - remaining - 50.0).abs() < 0.01,
        "Ravage spends Bite's base and extra energy: {energy} -> {remaining}"
    );
    assert_eq!(points, 0.0, "Ravage spends five combo points");
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.spell_id == Some(441591)
                && event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.amount > 0),
        "Ravage deals damage to its actual target"
    );
    assert!(
        crate::auras::has_aura(app.world(), target, 441812),
        "Ravage applies the selected Dreadful Wound"
    );
}
