use super::test_harness::*;
use shared::protocol::CombatLogKind;

#[test]
fn spell_override_queue_resolves_after_aura_expiry_not_at_admission() {
    let mut h = harness(Vec::new(), &[1719, 85288, 335097], 1).unwrap();
    let effect_mask = crate::auras::aura_effect_mask(
        h.app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(1719)
            .unwrap(),
    );
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1719,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.advance(235);
    let now = super::history_now(h.app.world());
    let mut history = super::SpellHistory::default();
    history.gcd_ready_at.insert(133, now + 0.4);
    h.app.world_mut().entity_mut(h.caster).insert(history);
    h.cast(85288, Some(h.npc));
    h.advance(15);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::CastSuccess && e.spell_id == Some(85288)),
        "{:?}",
        h.inbox.log
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::CastSuccess && e.spell_id == Some(335097))
    );
}
