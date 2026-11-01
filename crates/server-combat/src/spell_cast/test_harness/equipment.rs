//! Equip test casters with source items that satisfy their supplied weapon spells.
use bevy::prelude::*;
use shared::item_data::ItemDefinitionSource::Retail;
use shared::protocol::EquipmentSlot;

use super::WorldData;
use crate::item_templates::ItemTemplates;
use crate::spell_info::SpellInfo;
use crate::stats::EquippedItems;

pub(super) struct FixtureEquipment {
    items: EquippedItems,
    templates: ItemTemplates,
}

pub(super) fn select(data: &WorldData, class: u8, spells: &[SpellInfo]) -> FixtureEquipment {
    let requirements: Vec<_> = spells
        .iter()
        .filter_map(|spell| {
            spell
                .equipped_items
                .as_deref()
                .filter(|item| item.item_class == 2)
                .map(|item| (item, super::super::attributes(spell, 3)))
        })
        .collect();
    let mut selected = EquippedItems::default();
    let mut templates = ItemTemplates::default();
    if requirements.is_empty() {
        return FixtureEquipment {
            items: selected,
            templates,
        };
    }
    let Some(class_info) = data
        .classes
        .classes
        .iter()
        .find(|info| info.class_id == class)
    else {
        return FixtureEquipment {
            items: selected,
            templates,
        };
    };
    let Some(spec) = class_info.specs.first() else {
        return FixtureEquipment {
            items: selected,
            templates,
        };
    };
    let (mut known, _) =
        crate::class_progression::evaluate_npc_starter(&data.classes, class, spec.spec_id, 10)
            .expect("fixture class starter skills");
    let race = if class == 12 { 4 } else { 1 };
    known.extend(crate::class_progression::default_skill_spells(
        class_info, race, 10,
    ));
    let proficiency = known
        .iter()
        .filter_map(|spell| {
            data.items
                .loot_need
                .weapon_proficiencies
                .get(&spell.spell_id)
        })
        .fold(0, |mask, value| mask | value);
    let offhand_required = requirements.iter().any(|(_, flags)| flags & (1 << 24) != 0);
    let can_dual_wield = known.iter().any(|spell| {
        data.store
            .get(spell.spell_id)
            .is_some_and(|info| info.effects.iter().any(|effect| effect.effect == 40))
    });
    let choose = |offhand: bool| {
        data.items
            .retail
            .iter()
            .filter(|(_, item)| {
                item.scaling.class_id == 2
                    && item.required_level <= 10
                    && item.allows(class, race)
                    && proficiency & (1_u32 << item.scaling.subclass_id) != 0
                    && if offhand {
                        can_dual_wield && matches!(item.inventory_type, 13 | 22)
                    } else {
                        matches!(item.inventory_type, 13 | 15 | 17 | 21 | 26)
                            && (!offhand_required || !matches!(item.inventory_type, 15 | 17 | 26))
                    }
            })
            .filter_map(|(&id, item)| {
                let score = requirements
                    .iter()
                    .filter(|(requirement, flags)| {
                        let relevant = if offhand {
                            flags & (1 << 24) != 0
                        } else {
                            flags & (1 << 10) != 0 || flags & (1 << 24) == 0
                        };
                        relevant
                            && (requirement.item_subclass == 0
                                || requirement.item_subclass & (1_u32 << item.scaling.subclass_id)
                                    != 0)
                    })
                    .count();
                (score > 0).then_some((score, id, item))
            })
            .max_by_key(|(score, id, _)| (*score, std::cmp::Reverse(*id)))
    };
    for (slot, chosen) in [
        (EquipmentSlot::MainHand, choose(false)),
        (EquipmentSlot::OffHand, choose(true)),
    ] {
        if let Some((_, id, item)) = chosen {
            selected.items.push((slot, Retail, id));
            templates.retail.insert(id, item.clone());
        }
    }
    FixtureEquipment {
        items: selected,
        templates,
    }
}

pub(super) fn install(app: &mut App, caster: Entity, equipment: FixtureEquipment) {
    if equipment.items.items.is_empty() {
        return;
    }
    app.insert_resource(equipment.templates);
    app.world_mut().entity_mut(caster).insert(equipment.items);
}
