//! Native self-target trigger effects must retain the request's hostile primary.
use crate::spell_cast::test_harness::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
fn trigger_primary_native_annihilation_children_hit_selected_enemy() {
    let mut h = harness(Vec::new(), &[201427, 227518, 201428], 12).unwrap();
    let caster = h.caster;
    let target = h.npc;
    h.set_power(PowerType::Fury, 100.0);
    h.cast_and_deliver(201427, Some(target));
    h.advance(20);
    for child in [227518, 201428] {
        assert!(
            h.inbox.log.iter().any(|event| {
                event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(child)
                    && event.source == Some(caster.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.amount > 0
            }),
            "native Annihilation child{child} must hit its selected enemy: {:?}",
            h.inbox.log
        );
    }
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.target == Some(caster.to_bits()))
    );
}
