//! Native target45 chain-heal selection on Merithra's Blessing.
use crate::spell_cast::test_harness::{harness, spawn_player};
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native Merithra's Blessing spell data"]
fn target45_heals_friendly_primary_and_jumps_to_greatest_deficits() {
    let mut h = harness(Vec::new(), &[1256581], 13).unwrap();
    let primary = spawn_player(&mut h.app, 13, 1.0);
    let close_low_deficit = spawn_player(&mut h.app, 13, 2.0);
    let second = spawn_player(&mut h.app, 13, 3.0);
    let third = spawn_player(&mut h.app, 13, 4.0);
    let fourth = spawn_player(&mut h.app, 13, 5.0);
    let far_high_deficit = spawn_player(&mut h.app, 13, 12.0);
    for (unit, current) in [
        (primary, 500.0),
        (close_low_deficit, 900.0),
        (second, 800.0),
        (third, 700.0),
        (fourth, 600.0),
        (far_high_deficit, 300.0),
    ] {
        h.app.world_mut().get_mut::<Health>(unit).unwrap().current = current;
    }
    h.cast_and_deliver(1256581, Some(primary));
    // Native projectile speed delays the farthest jump beyond the cast frame.
    h.advance(40);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let targets: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| event.spell_id == Some(1256581) && event.kind == CombatLogKind::Heal)
        .filter_map(|event| event.target)
        .collect();
    assert_eq!(targets.len(), 5, "{targets:?}");
    for unit in [primary, second, third, fourth, far_high_deficit] {
        assert!(
            targets.contains(&unit.to_bits()),
            "missing {unit:?}: {targets:?}"
        );
    }
    assert!(!targets.contains(&close_low_deficit.to_bits()));
}
