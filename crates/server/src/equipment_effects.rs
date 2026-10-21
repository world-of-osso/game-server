//! Equipped native item/enchant spells follow the ordinary aura lifecycle.
use bevy::prelude::*;

pub(crate) fn sync_equipment_effects(_world: &mut World) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell_cast::test_harness::{Harness, harness, world_db};
    use crate::stats::{EquippedItemModifiers, EquippedItems, NativeItemInstances};
    use server::item_instance::{ItemInstanceModifiers, load};
    use shared::components::Level;
    use shared::item_data::ItemDefinitionSource::Retail;
    use shared::protocol::EquipmentSlot;

    fn native_harness() -> Harness {
        let path = std::env::var_os("DPS_PARITY_WORLD_DB")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(crate::test_data::world_db_path);
        let conn = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut h = harness(vec![], &[], 2).unwrap();
        h.app.insert_resource(world_db().unwrap().store.clone());
        h.app.insert_resource(NativeItemInstances {
            bonuses: load::load_bonus_tables(&conn).unwrap(),
            enchants: load::load_enchantment_tables(&conn).unwrap(),
            effects: load::load_item_effect_tables(&conn).unwrap(),
        });
        h.app
            .insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
        h.app
            .insert_resource(crate::item_stats::load_item_scaling_tables(&path).unwrap());
        h.app.world_mut().get_mut::<Level>(h.caster).unwrap().0 = 90;
        h
    }

    fn equip(h: &mut Harness, slot: EquipmentSlot, item: u32, modifiers: ItemInstanceModifiers) {
        h.app.world_mut().entity_mut(h.caster).insert((
            EquippedItems {
                items: vec![(slot, Retail, item)],
            },
            EquippedItemModifiers {
                items: vec![(slot, modifiers)],
            },
        ));
        h.advance(2);
    }

    #[test]
    fn item_instance_equipment_base_spell_is_removed_on_unequip() {
        let mut h = native_harness();
        // Native268209 -> ItemEffect232799 -> equip spell1291718.
        equip(
            &mut h,
            EquipmentSlot::MainHand,
            268209,
            ItemInstanceModifiers {
                item_level_override: Some(344),
                ..Default::default()
            },
        );
        assert!(crate::auras::has_aura(h.app.world(), h.caster, 1291718));
        h.app
            .world_mut()
            .get_mut::<EquippedItems>(h.caster)
            .unwrap()
            .items
            .clear();
        h.advance(2);
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, 1291718));
    }

    #[test]
    fn item_instance_equipment_enchant_removal_keeps_base_aura_and_removes_linked_child() {
        let mut h = native_harness();
        // Native8039 -> linked parent1236742 -> actual child1241710.
        equip(
            &mut h,
            EquipmentSlot::MainHand,
            268209,
            ItemInstanceModifiers {
                item_level_override: Some(344),
                enchant: 8039,
                ..Default::default()
            },
        );
        assert!(crate::auras::has_aura(h.app.world(), h.caster, 1236742));
        assert!(crate::auras::has_aura(h.app.world(), h.caster, 1241710));
        h.app
            .world_mut()
            .get_mut::<EquippedItemModifiers>(h.caster)
            .unwrap()
            .items[0]
            .1
            .enchant = 0;
        h.advance(2);
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, 1236742));
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, 1241710));
        assert!(crate::auras::has_aura(h.app.world(), h.caster, 1291718));
    }

    #[test]
    fn item_instance_equipment_mid2_bonus_spell_is_real_aura_not_fake_flat_stats() {
        let mut h = native_harness();
        // Shipped MID2 Fire offhand245769/list13771 -> effect234561/spell1297382.
        equip(
            &mut h,
            EquipmentSlot::OffHand,
            245769,
            ItemInstanceModifiers {
                item_level_override: Some(331),
                bonus_list: vec![8960, 13751, 13771],
                crafted_stats: vec![36, 40],
                ..Default::default()
            },
        );
        assert!(crate::auras::has_aura(h.app.world(), h.caster, 1297382));
        h.app
            .world_mut()
            .get_mut::<EquippedItemModifiers>(h.caster)
            .unwrap()
            .items[0]
            .1
            .bonus_list
            .clear();
        h.advance(2);
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, 1297382));
    }
}
