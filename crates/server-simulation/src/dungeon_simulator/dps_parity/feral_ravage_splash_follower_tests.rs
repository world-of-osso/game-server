//! Ravage's secondary coefficient cannot hit the primary target twice.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_ravage_hits_each_target_once() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, primary, _) = prepare_class_follower(app, copy, 103, 90);
    let secondary = spawn_level_dummy(&mut app, actor, 90, 1).unwrap();
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 441585, Some(actor), 1);
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::ComboPoints, 5.0);
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
    let spell =
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 22568).unwrap();
    crate::spell_cast::creature_cast(app.world_mut(), actor, spell, Some(primary), false).unwrap();
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let hits = |target: Entity| {
        events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(441591)
                    && event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.amount > 0
            })
            .count()
    };
    assert_eq!(hits(primary), 1, "primary receives only its direct hit");
    assert_eq!(
        hits(secondary),
        1,
        "nearby secondary receives the splash hit"
    );
}
