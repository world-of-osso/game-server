//! Native Windlord impacts and subsequent follower melee swings produce Thunderfist.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn follower_windlord_earns_and_spends_thunderfist() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 269, 70);
    let owner = app.world().get::<crate::followers::Follower>(actor).unwrap().owner;
    for _ in 0..5 { app.update(); }
    start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
    for frame in 0..FIXED_FIGHT_FRAMES {
        let mut health = app.world_mut().get_mut::<shared::components::Health>(target).unwrap();
        health.current = health.max * (FIXED_FIGHT_FRAMES - frame) as f32 / FIXED_FIGHT_FRAMES as f32;
        app.update();
    }
    let events = &app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    let first_windlord = events.iter().position(|event| event.source == Some(actor.to_bits())
        && event.target == Some(target.to_bits()) && event.kind == CombatLogKind::Damage
        && event.spell_id == Some(395521)).expect("real off-hand Windlord impact");
    let hits: Vec<_> = events.iter().enumerate().filter(|(_, event)| event.source == Some(actor.to_bits())
        && event.target == Some(target.to_bits()) && event.kind == CombatLogKind::Damage
        && event.spell_id == Some(393566)).collect();
    assert!(!hits.is_empty(), "earned Thunderfist must damage the target");
    assert!(hits.iter().all(|(index, event)| *index > first_windlord && event.amount > 0));
    let windlord_hits = events.iter().filter(|event| event.source == Some(actor.to_bits())
        && event.target == Some(target.to_bits()) && event.kind == CombatLogKind::Damage
        && event.spell_id == Some(395521)).count();
    assert!(hits.len() <= windlord_hits * 4, "each native Windlord hit grants only four stacks");
}
