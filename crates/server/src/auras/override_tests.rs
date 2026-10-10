use crate::spell_cast::test_harness::*;
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

fn apply(h: &mut Harness, id: u32) {
    let effect_mask =
        super::aura_effect_mask(h.app.world().resource::<SpellStore>().get(id).unwrap());
    super::apply_aura(
        h.app.world_mut(),
        &super::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: id,
            effect_mask,
            source: super::AuraSource::Cast,
        },
    );
}

#[test]
fn spell_override_recklessness_replaces_raging_blow_and_restores_after_removal() {
    // Actual matrix1719 rows: MiscValue85288 -> amount335097.
    let mut h = harness(Vec::new(), &[1719, 85288, 335097], 1).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            1719, 85288,
        ]));
    apply(&mut h, 1719);
    h.cast(85288, Some(h.npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::CastSuccess && e.spell_id == Some(335097)),
        "{:?}",
        h.inbox.log
    );
    super::remove_auras(
        h.app.world_mut(),
        h.caster,
        super::RemoveReason::Cancelled,
        |a| a.spell_id == 1719,
    );
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::spell_cast::SpellHistory::default());
    h.inbox.log.clear();
    h.cast(85288, Some(h.npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::CastSuccess && e.spell_id == Some(85288))
    );
}
