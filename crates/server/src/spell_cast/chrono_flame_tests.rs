//! Native431442 ->431443 override and declared Living-Flame/Chrono effect rows.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{ActiveSpec, Level, UnitLevel};
use shared::protocol::CombatLogKind;

#[test]
fn class_completeness_native_chrono_flames_resolves_fire_and_arcane_repeat() {
    let mut h = harness(
        Vec::new(),
        &[361469, 361500, 361509, 431442, 431443, 431483, 431583],
        13,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), UnitLevel(80), ActiveSpec(1467)));
    h.advance(3);
    h.cast_and_deliver(361469, Some(h.npc));
    h.advance(50);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 431443));
    let fire = h
        .inbox
        .log
        .iter()
        .find(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(361500))
        .expect("native Chrono replacement must retain Living Flame's fire branch");
    let arcane = h
        .inbox
        .log
        .iter()
        .find(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(431583))
        .expect("native Chrono Flame repeat must land as the Arcane child");
    let percent = world_db()
        .unwrap()
        .store
        .get(431442)
        .unwrap()
        .effects
        .iter()
        .find(|e| e.index == 2)
        .unwrap()
        .base_points;
    assert_eq!(arcane.school_mask, 64);
    assert_eq!(arcane.target, Some(h.npc.to_bits()));
    assert_eq!(
        arcane.amount,
        (fire.amount as f32 * percent / 100.0).round() as i32
    );
}
