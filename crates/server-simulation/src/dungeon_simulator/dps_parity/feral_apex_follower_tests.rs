//! Apex Predator's Craving is a low-chance Rip-tick proc, not one per periodic tick.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_earns_apex_only_from_rare_rip_ticks() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
    let owner = app.world().get::<crate::followers::Follower>(actor).unwrap().owner;
    for _ in 0..5 {
        app.update();
    }
    start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
    for frame in 0..FIXED_FIGHT_FRAMES {
        let mut health = app.world_mut().get_mut::<shared::components::Health>(target).unwrap();
        health.current = health.max * (FIXED_FIGHT_FRAMES - frame) as f32 / FIXED_FIGHT_FRAMES as f32;
        app.update();
    }
    let events = &app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    let count = |kind: CombatLogKind, spell: u32, source: Entity| {
        events.iter().filter(|event| event.kind == kind && event.spell_id == Some(spell)
            && event.source == Some(source.to_bits())).count()
    };
    let rip_ticks = count(CombatLogKind::Damage, 1079, actor);
    let apex = count(CombatLogKind::AuraApplied, 391882, actor);
    let bites = count(CombatLogKind::CastSuccess, 22568, actor);
    assert!(rip_ticks >= 40, "the real rotation must maintain Rip: {rip_ticks} ticks");
    // SimC76f8118 rolls 4% per single-target Rip tick.
    assert!(apex * 5 <= rip_ticks, "Apex must stay rare: {apex} grants from {rip_ticks} Rip ticks");
    assert!(bites <= 30, "free Bites cannot dominate the rotation: {bites} casts");
}
