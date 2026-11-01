//! Native curse classification and removal.
use super::{kit_tests::level32, utility_tests};
use crate::spell_cast::test_harness::Harness;
use shared::protocol::CombatLogKind;

fn cast_debuff(h: &mut Harness, spell: u32, spec: u32) {
    let enemy = h.npc;
    let patient = h.caster;
    utility_tests::prepare_enemy_spec(h, enemy, spec);
    crate::spell_cast::creature_cast(h.app.world_mut(), enemy, spell, Some(patient), false)
        .unwrap();
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), patient, spell));
}

#[test]
fn native_curse702_harmful_classification_gap() {
    let (mut h, _) = level32(209059);
    cast_debuff(&mut h, 702, 266);
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 702)
        .unwrap();
    assert!(
        aura.harmful,
        "native342/290 curse positivity is not a follower-picker decision"
    );
}

#[test]
fn native_curse702_real_player_remove_curse_removes_native_application() {
    let (mut h, _) = level32(209059);
    cast_debuff(&mut h, 702, 266);
    let harmful_before = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 702)
        .unwrap()
        .harmful;
    // Explicit native player spellbook input; no follower AI executes this cast.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(
            [475].into_iter().collect(),
        ));
    h.cast_and_deliver(475, Some(h.caster));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| { go.caster == h.caster.to_bits() && go.spell_id == 475 })
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), h.caster, 702),
        "real player Remove Curse left native702; harmful_before={harmful_before}"
    );
    assert!(h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::Dispel
            && event.source == Some(h.caster.to_bits())
            && event.target == Some(h.caster.to_bits())
            && event.spell_id == Some(475)
            && event.extra_spell_id == Some(702)
    }));
}

#[test]
fn native_curse702_signed_haste_counterparts_preserve_positive_and_passive_rows() {
    let data = crate::spell_cast::test_harness::world_db().unwrap();
    // All values/effect indices come from unchanged native70448 rows.
    for (spell, negative) in [(702, true), (442804, true), (5171, false), (404542, false)] {
        let info = data.store.get(spell).unwrap();
        let haste = info
            .effects
            .iter()
            .find(|effect| effect.aura == 342)
            .unwrap();
        let negative_mask = crate::spell_info::positivity::negative_effects(&data.store, info);
        assert_eq!(
            negative_mask & (1 << haste.index) != 0,
            negative,
            "spell{spell}"
        );
    }
}
