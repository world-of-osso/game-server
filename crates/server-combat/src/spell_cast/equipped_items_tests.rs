use super::test_harness::harness;
use super::{CastFailure, creature_cast};
use crate::stats::EquippedItems;
use shared::item_data::ItemDefinitionSource::Retail;
use shared::protocol::EquipmentSlot::{MainHand, OffHand};

#[test]
fn weapon_fixture_uses_native_class_proficiency_for_casts() {
    for (class, spell) in [(3, 19_434), (12, 263_642)] {
        let mut h = harness(Vec::new(), &[spell], class).unwrap();
        let result = creature_cast(h.app.world_mut(), h.caster, spell, Some(h.npc), false);
        let data = super::test_harness::world_db().unwrap();
        let info = data
            .classes
            .classes
            .iter()
            .find(|info| info.class_id == class)
            .unwrap();
        let (known, _) = crate::class_progression::evaluate_npc_starter(
            &data.classes,
            class,
            info.specs[0].spec_id,
            10,
        )
        .unwrap();
        eprintln!(
            "class {class} proficiency {:?}, dual {}",
            known
                .iter()
                .filter_map(|s| data
                    .items
                    .loot_need
                    .weapon_proficiencies
                    .get(&s.spell_id)
                    .map(|mask| (s.spell_id, mask)))
                .collect::<Vec<_>>(),
            known.iter().any(|s| s.spell_id == 674)
        );
        assert_ne!(
            result,
            Err(CastFailure::EquippedItemClass),
            "class {class}: {result:?}; equipped {:?}",
            h.app.world().get::<EquippedItems>(h.caster)
        );
    }
}

#[test]
fn vengeance_weapon_requirements_reject_dagger_main_hand() {
    let mut h = harness(Vec::new(), &[263_642, 228_477], 12).unwrap();
    let path = crate::test_data::world_db_path();
    h.app
        .world_mut()
        .insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(EquippedItems {
            items: vec![(MainHand, Retail, 251_128), (OffHand, Retail, 258_045)],
        });
    for spell in [263_642, 228_477] {
        assert_eq!(
            creature_cast(h.app.world_mut(), h.caster, spell, Some(h.npc), false),
            Err(CastFailure::EquippedItemClass)
        );
    }
    h.app
        .world_mut()
        .get_mut::<EquippedItems>(h.caster)
        .unwrap()
        .items[0]
        .2 = 258_045;
    for spell in [263_642, 228_477] {
        let info = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(spell)
            .unwrap();
        assert_eq!(
            super::equipped_items::check_cast(h.app.world(), h.caster, info),
            Ok(())
        );
    }
}
