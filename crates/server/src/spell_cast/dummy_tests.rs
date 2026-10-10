//! Scripted class spells through the player cast pipeline, using real world.db IDs.

use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

use super::test_harness::*;

#[test]
fn dummy_warrior_shield_block_applies_six_second_block_aura() {
    let mut h = harness(Vec::new(), &[2565, 132404], 1).expect("real world.db required");
    h.set_power(PowerType::Rage, 1000.0);
    h.cast_and_deliver(2565, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 132404));
    let auras = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap();
    let block = auras.iter().find(|aura| aura.spell_id == 132404).unwrap();
    assert_eq!(block.duration_ms, 6000);
    assert_eq!(
        block
            .effects
            .iter()
            .find(|effect| effect.aura_type == 51)
            .unwrap()
            .amount,
        100.0
    );
    h.advance(125);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 132404));
}

#[test]
fn dummy_warrior_bloodthirst_damages_generates_rage_and_heals_three_percent() {
    let mut h = harness(Vec::new(), &[23881, 117313, 184362], 1).expect("real world.db required");
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 500.0;
    h.set_power(PowerType::Rage, 0.0);
    h.cast_and_deliver(23881, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(424.203).contains(&(5000.0 - h.health(h.npc))));
    assert_eq!(h.power(PowerType::Rage), 80.0);
    assert_eq!(h.health(h.caster), 530.0);
    h.advance(3);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(117313)
                && event.amount == 30)
    );
}

#[test]
fn dummy_warrior_bloodthirst_enrage_uses_effect_two_chance() {
    // Controlled chance endpoints on a real-ID catalog clone, not Retail's
    // 30% random roll. The unmodified-data test above proves the base cast.
    let original = world_db().unwrap().store.get(23881).unwrap();
    assert_eq!(
        original
            .effects
            .iter()
            .find(|effect| effect.index == 2)
            .unwrap()
            .base_points,
        30.0
    );
    for (chance, expected) in [(0.0, false), (100.0, true)] {
        let mut spell = original.clone();
        spell
            .effects
            .iter_mut()
            .find(|effect| effect.index == 2)
            .unwrap()
            .base_points = chance;
        let mut h = harness(vec![spell], &[117313, 184362], 1).unwrap();
        h.cast_and_deliver(23881, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert_eq!(
            crate::auras::has_aura(h.app.world(), h.caster, 184362),
            expected
        );
    }
}

#[test]
fn dummy_warrior_rampage_spends_rage_hits_and_enrages() {
    let ids = [184367, 184707, 184709, 201364, 201363, 184362];
    let mut h = harness(Vec::new(), &ids, 1).expect("real world.db required");
    h.set_power(PowerType::Rage, 1000.0);
    h.cast_and_deliver(184367, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Rage), 200.0);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 184362));
    h.advance(3);
    for id in [184707, 184709, 201364, 201363] {
        assert!(
            h.inbox
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(id)
                    && event.target == Some(h.npc.to_bits())),
            "missing {id}"
        );
    }
}
