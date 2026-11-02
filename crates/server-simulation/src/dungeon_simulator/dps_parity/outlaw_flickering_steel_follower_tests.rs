//! Opportunity earned in a real Outlaw follower fight restores Flickering Steel energy.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn follower_opportunity_restores_flickering_steel_energy() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 260, 70);
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    for _ in 0..5 {
        app.update();
    }
    assert!(crate::auras::has_aura(app.world(), actor, 1259492));
    start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
    for frame in 0..FIXED_FIGHT_FRAMES {
        let mut health = app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current =
            health.max * (FIXED_FIGHT_FRAMES - frame) as f32 / FIXED_FIGHT_FRAMES as f32;
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let procs = events
        .iter()
        .filter(|e| {
            e.source == Some(actor.to_bits())
                && e.target == Some(actor.to_bits())
                && e.spell_id == Some(195627)
                && matches!(
                    e.kind,
                    CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                )
        })
        .count();
    let energy: Vec<_> = events
        .iter()
        .filter(|e| {
            e.source == Some(actor.to_bits())
                && e.target == Some(actor.to_bits())
                && e.spell_id == Some(1259493)
                && e.kind == CombatLogKind::Energize
        })
        .collect();
    assert!(procs > 0, "the fight must earn Opportunity");
    assert_eq!(
        energy.len(),
        procs,
        "each earned Opportunity triggers Flickering Steel"
    );
    assert!(
        energy.iter().any(|e| e.amount == 5),
        "native energize gives five energy"
    );
}
