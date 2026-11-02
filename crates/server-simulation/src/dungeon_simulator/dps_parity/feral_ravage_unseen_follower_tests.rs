//! Ravage inherits Bite's selected Unseen attack proc.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_ravage_earns_delayed_unseen_attacks() {
    let _seed = crate::random::seed_scope(1031263657);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
    assert!(crate::auras::has_aura(app.world(), actor, 1263657));
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    for _ in 0..20 {
        // Isolate each paid impact from the rotation and its GCD; no ordinary Bite executes.
        app.world_mut()
            .entity_mut(actor)
            .remove::<crate::spell_cast::SpellHistory>();
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 441585, Some(actor), 1);
        crate::spell_cast::powers::change_power(
            app.world_mut(),
            actor,
            PowerType::ComboPoints,
            5.0,
        );
        crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
        let spell =
            crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 22568).unwrap();
        crate::spell_cast::creature_cast(app.world_mut(), actor, spell, Some(target), false)
            .unwrap();
    }
    for _ in 0..15 {
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let count = |kind, spell| {
        events
            .iter()
            .filter(|event| {
                event.kind == kind
                    && event.spell_id == Some(spell)
                    && event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
            })
            .count()
    };
    assert_eq!(
        count(CombatLogKind::Damage, 441591),
        20,
        "twenty landed Ravages"
    );
    let unseen = count(CombatLogKind::Damage, 1263890);
    assert!(
        unseen > 0 && unseen < 20,
        "Ravage must sometimes earn an Unseen Slash, not one per cast: {unseen}"
    );
}
