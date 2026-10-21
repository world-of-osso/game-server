//! Equipped native item/enchant spells follow the ordinary aura lifecycle.
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::spell_info::{SpellInfo, SpellStore};
use crate::stats::{EquippedItemModifiers, EquippedItems, NativeItemInstances};
use bevy::prelude::*;
use server::item_instance::{ItemInstanceModifiers, bonuses, effects};
use shared::components::Level;
use shared::item_data::ItemDefinitionSource;
use shared::protocol::EquipmentSlot;

const ON_EQUIP: u8 = 1;

type ChangedEquipment = QueryState<
    Entity,
    (
        With<EquippedItems>,
        With<EquippedItemModifiers>,
        Or<(
            Changed<EquippedItems>,
            Changed<EquippedItemModifiers>,
            Changed<Level>,
        )>,
    ),
>;

pub(crate) fn sync_equipment_effects(world: &mut World, changed: &mut ChangedEquipment) {
    let units: Vec<Entity> = changed.iter(world).collect();
    for unit in units {
        let desired = equipped_spells(world, unit);
        crate::auras::remove_auras(world, unit, RemoveReason::Unlearned, |aura| {
            let AuraSource::Equipment(slot) = aura.source else {
                return false;
            };
            !desired
                .iter()
                .any(|(key, spell)| *key == slot && spell.id == aura.spell_id)
        });
        for (slot, spell) in desired {
            apply_missing_equipment_aura(world, unit, slot, &spell);
        }
    }
}

fn equipped_spells(world: &World, unit: Entity) -> Vec<(EquipmentSlot, SpellInfo)> {
    let equipped = world
        .get::<EquippedItems>(unit)
        .expect("equipped catalog identities");
    let modifiers = world
        .get::<EquippedItemModifiers>(unit)
        .expect("equipped instance projection");
    let level = world.get::<Level>(unit).expect("equipment owner level").0;
    equipped
        .items
        .iter()
        .filter(|(_, source, _)| *source == ItemDefinitionSource::Retail)
        .flat_map(|&(slot, _, item_id)| {
            let properties = modifiers
                .items
                .iter()
                .find(|(key, _)| *key == slot)
                .map(|(_, state)| state.clone())
                .unwrap_or_default();
            resolve_slot_spells(world, slot, item_id, &properties, level)
        })
        .collect()
}

fn resolve_slot_spells(
    world: &World,
    slot: EquipmentSlot,
    item_id: u32,
    properties: &ItemInstanceModifiers,
    level: u8,
) -> Vec<(EquipmentSlot, SpellInfo)> {
    let (item_level, bonus_effects) = resolve_item_level_and_effects(world, item_id, properties);
    let ids = equipped_spell_ids(world, item_id, &bonus_effects, properties, level);
    ids.into_iter()
        .map(|id| {
            let mut spell = world
                .resource::<SpellStore>()
                .get(id)
                .cloned()
                .unwrap_or_else(|| panic!("equipped item {item_id} spell {id} absent"));
            spell.cast_item_level = Some(item_level);
            (slot, spell)
        })
        .collect()
}

fn resolve_item_level_and_effects(
    world: &World,
    item_id: u32,
    properties: &ItemInstanceModifiers,
) -> (u16, Vec<u32>) {
    let native = world.resource::<NativeItemInstances>();
    let templates = world.resource::<crate::item_templates::ItemTemplates>();
    let template = templates
        .get(ItemDefinitionSource::Retail, item_id)
        .unwrap_or_else(|| panic!("native equipped item {item_id} absent"));
    let modified =
        bonuses::apply_item_bonuses(&template.scaling, &properties.bonus_list, &native.bonuses)
            .unwrap_or_else(|error| panic!("equipped item {item_id} bonuses: {error}"));
    let item_level = server::item_instance::item_instance_stats(
        &modified.scaling,
        template.inventory_type,
        properties,
        world.resource::<crate::item_stats::ItemScalingTables>(),
    )
    .unwrap_or_else(|error| panic!("equipped item {item_id} level: {error}"))
    .item_level;
    (item_level, modified.effect_ids)
}

fn equipped_spell_ids(
    world: &World,
    item_id: u32,
    bonus_effects: &[u32],
    properties: &ItemInstanceModifiers,
    level: u8,
) -> Vec<u32> {
    let native = world.resource::<NativeItemInstances>();
    let mut ids: Vec<u32> = effects::equipped_item_effects(item_id, bonus_effects, &native.effects)
        .unwrap_or_else(|error| panic!("{error}"))
        .into_iter()
        .filter(|effect| effect.trigger == ON_EQUIP)
        .map(|effect| effect.spell_id)
        .collect();
    ids.extend(
        crate::stats::instance_enchant_contributions(properties, level, &native.enchants)
            .into_iter()
            .flat_map(|contribution| contribution.equip_spells),
    );
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn apply_missing_equipment_aura(
    world: &mut World,
    unit: Entity,
    slot: EquipmentSlot,
    spell: &SpellInfo,
) {
    let source = AuraSource::Equipment(slot);
    let applied = world.get::<ActiveAuras>(unit).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == spell.id && aura.source == source)
    });
    if applied {
        return;
    }
    let effect_mask = crate::auras::aura_effect_mask(spell);
    assert_ne!(
        effect_mask, 0,
        "native equip spell {} has no applicable aura effects",
        spell.id
    );
    crate::auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: spell.id,
            effect_mask,
            source,
        },
        spell,
    );
}

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
