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
fn spell_override_animation_and_actionbar_values_replicate_until_expiry() {
    let mut h = harness(Vec::new(), &[1719, 85288, 335097, 406732], 1).unwrap();
    apply(&mut h, 1719);
    apply(&mut h, 406732);
    h.advance(3);
    let view = h
        .app
        .world()
        .get::<shared::components::UnitAuras>(h.caster)
        .unwrap();
    let json = serde_json::to_value(view).unwrap();
    let auras = json["auras"].as_array().unwrap();
    let recklessness = auras.iter().find(|a| a["spell_id"] == 1719).unwrap();
    assert!(
        recklessness["overrides"].as_array().is_some_and(|list| list
            .contains(&serde_json::json!({"ActionBar": {"spell_id":85288,"replacement":335097}}))),
        "{json}"
    );
    let paradox = auras.iter().find(|a| a["spell_id"] == 406732).unwrap();
    assert_eq!(
        paradox["overrides"],
        serde_json::json!([{"Animation":1013}])
    );
    h.advance(500);
    let view = h
        .app
        .world()
        .get::<shared::components::UnitAuras>(h.caster)
        .unwrap();
    assert!(
        view.auras
            .iter()
            .all(|a| a.spell_id != 1719 && a.spell_id != 406732)
    );
    assert_eq!(
        super::overrides::resolve_spell(h.app.world(), h.caster, 85288),
        85288
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
