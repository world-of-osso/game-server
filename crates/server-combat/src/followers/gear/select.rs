//! Select real, class-appropriate items from the source dungeon tier.
#[cfg(test)]
#[path = "select_tests.rs"]
mod tests;
use std::collections::BTreeSet;

use shared::item_data::ItemDefinitionSource;
use shared::protocol::EquipmentSlot;

use super::JournalGearCandidate;
use crate::class_progression::KnownSpells;
use crate::item_templates::{ItemTemplate, ItemTemplates};
use crate::stats::PrimaryStat;

/// Selected Retail item identities and slots for which no eligible item exists.
pub struct GearSelection {
    pub items: Vec<(EquipmentSlot, ItemDefinitionSource, u32)>,
    pub missing: Vec<EquipmentSlot>,
}

fn has_primary(item: &ItemTemplate, primary: PrimaryStat) -> bool {
    item.scaling.stats.iter().any(|stat| {
        stat.allocation > 0
            && match primary {
                PrimaryStat::Strength => matches!(stat.stat, 4 | 71 | 72 | 74),
                PrimaryStat::Agility => matches!(stat.stat, 3 | 71 | 72 | 73),
                PrimaryStat::Intellect => matches!(stat.stat, 5 | 71 | 73 | 74),
            }
    })
}

fn class_allowed(item: &ItemTemplate, class: u8) -> bool {
    item.allowable_class == -1 || item.allowable_class as u64 & (1_u64 << (class - 1)) != 0
}

fn equipment_eligible(
    item: &ItemTemplate,
    class: u8,
    primary: PrimaryStat,
    known: &KnownSpells,
    templates: &ItemTemplates,
) -> bool {
    if !class_allowed(item, class) {
        return false;
    }
    let has_any_primary = [
        PrimaryStat::Strength,
        PrimaryStat::Agility,
        PrimaryStat::Intellect,
    ]
    .into_iter()
    .any(|stat| has_primary(item, stat));
    if has_any_primary && !has_primary(item, primary) {
        return false;
    }
    let subclass = 1_u32 << item.scaling.subclass_id;
    match item.scaling.class_id {
        4 if item.inventory_type != 16 => {
            let Some(mask) = templates.loot_need.armor_masks.get(&class) else {
                return false;
            };
            if mask & subclass == 0 {
                return false;
            }
            !matches!(item.inventory_type, 1 | 3 | 5 | 6 | 7 | 8 | 9 | 10 | 20)
                || has_primary(item, primary)
        }
        2 => {
            known.iter().any(|spell| {
                templates
                    .loot_need
                    .weapon_proficiencies
                    .get(&spell)
                    .is_some_and(|mask| mask & subclass != 0)
            }) && (has_primary(item, primary)
                || item.scaling.stats.iter().all(|stat| stat.stat <= 0))
        }
        4 => true,
        _ => false,
    }
}

fn inventory_types(slot: EquipmentSlot) -> &'static [u8] {
    use EquipmentSlot::*;
    match slot {
        Head => &[1],
        Neck => &[2],
        Shoulder => &[3],
        Back => &[16],
        Chest => &[5, 20],
        Wrist => &[9],
        Hands => &[10],
        Waist => &[6],
        Legs => &[7],
        Feet => &[8],
        Finger1 | Finger2 => &[11],
        Trinket1 | Trinket2 => &[12],
        MainHand => &[13, 21, 17, 15, 26],
        OffHand => &[14, 23, 13, 22],
        _ => &[],
    }
}

fn is_two_handed(inventory_type: u8) -> bool {
    matches!(inventory_type, 17 | 15 | 26)
}

/// Select own-map loot first, then other same-tier loot; never repeat an item.
/// Unknown Journal difficulty/faction masks are excluded rather than guessed.
pub(super) fn select_items(
    candidates: &[JournalGearCandidate],
    map: u32,
    class: u8,
    primary: PrimaryStat,
    spec_id: u32,
    level: u8,
    known: &KnownSpells,
    spells: &crate::spell_info::SpellStore,
    templates: &ItemTemplates,
    item_specs: &super::item_spec::ItemSpecs,
    actions: &[&crate::spell_info::SpellInfo],
) -> GearSelection {
    use EquipmentSlot::*;
    let mut candidates: Vec<_> = candidates
        .iter()
        .filter(|row| row.difficulty_mask == -1 && row.faction_mask == -1)
        .collect();
    candidates.sort_by_key(|row| (row.source_map != map, row.item_id));
    let mut chosen = Vec::new();
    let mut missing = Vec::new();
    let mut used = BTreeSet::new();
    // Inventory admission uses native SPELL_EFFECT_DUAL_WIELD40, not one spell ID.
    let can_dual_wield = known.iter().any(|spell| {
        spells
            .get(spell)
            .is_some_and(|info| info.effects.iter().any(|effect| effect.effect == 40))
    });
    let titan_grip = crate::inventory::inventory_model::find_titan_grip(Some(known), Some(spells));
    let eligible = |slot: EquipmentSlot, used: &BTreeSet<u32>| {
        candidates
            .iter()
            .filter_map(|row| {
                let item = templates.get(ItemDefinitionSource::Retail, row.item_id)?;
                let ranged_hunter = matches!(spec_id, 253 | 254)
                    && slot == MainHand
                    && !matches!(item.inventory_type, 15 | 26);
                let shield_tank = matches!(spec_id, 66 | 73)
                    && ((slot == MainHand && !matches!(item.inventory_type, 13 | 21))
                        || (slot == OffHand && item.inventory_type != 14));
                let grip_allows = can_dual_wield
                    && item.inventory_type == 17
                    && titan_grip.as_ref().is_some_and(|grip| grip.allows(item));
                let valid_slot = inventory_types(slot).contains(&item.inventory_type)
                    || (slot == OffHand && grip_allows);
                let no_dual_wield =
                    slot == OffHand && matches!(item.inventory_type, 13 | 22) && !can_dual_wield;
                let monk_two_hand = spec_id == 269
                    && !can_dual_wield
                    && slot == MainHand
                    && item.inventory_type != 17;
                if used.contains(&row.item_id)
                    || !valid_slot
                    || ranged_hunter
                    || shield_tank
                    || no_dual_wield
                    || monk_two_hand
                    || (item.scaling.class_id == 2
                        && !item_specs.allows(row.item_id, item, spec_id, level))
                    || !equipment_eligible(item, class, primary, known, templates)
                {
                    return None;
                }
                Some((row.item_id, item.inventory_type, grip_allows))
            })
            .collect::<Vec<_>>()
    };
    for slot in [
        Head, Neck, Shoulder, Back, Chest, Wrist, Hands, Waist, Legs, Feet, Finger1, Finger2,
        Trinket1, Trinket2,
    ] {
        if let Some((item, _, _)) = eligible(slot, &used).first() {
            used.insert(*item);
            chosen.push((slot, ItemDefinitionSource::Retail, *item));
        } else {
            missing.push(slot);
        }
    }
    let main_candidates = eligible(MainHand, &used);
    let body: Vec<_> = chosen
        .iter()
        .filter_map(|&(slot, source, id)| templates.get(source, id).map(|item| (slot, item)))
        .collect();
    let mut pair = None;
    let mut bare_one_hand = None;
    for &(main, inventory, grip_allows) in &main_candidates {
        let mut occupied = used.clone();
        occupied.insert(main);
        let blocks_offhand = is_two_handed(inventory) && !grip_allows;
        let offhands = if blocks_offhand {
            Vec::new()
        } else {
            eligible(OffHand, &occupied)
        };
        let options = offhands
            .iter()
            .map(|&(id, _, _)| Some(id))
            .chain(std::iter::once(None));
        for offhand in options {
            if offhand.is_none() && matches!(spec_id, 66 | 73) {
                continue;
            }
            let mut equipped = body.clone();
            equipped.push((
                MainHand,
                templates.get(ItemDefinitionSource::Retail, main).unwrap(),
            ));
            if let Some(offhand) = offhand {
                equipped.push((
                    OffHand,
                    templates
                        .get(ItemDefinitionSource::Retail, offhand)
                        .unwrap(),
                ));
            }
            if actions.iter().all(|spell| {
                crate::spell_cast::equipped_items::check_items(&equipped, spell).is_ok()
            }) {
                if offhand.is_none() && !is_two_handed(inventory) {
                    bare_one_hand.get_or_insert((main, offhand));
                } else {
                    pair = Some((main, offhand));
                }
                break;
            }
        }
        if pair.is_some() {
            break;
        }
    }
    if let Some((main, offhand)) = pair.or(bare_one_hand) {
        chosen.push((MainHand, ItemDefinitionSource::Retail, main));
        if let Some(offhand) = offhand {
            chosen.push((OffHand, ItemDefinitionSource::Retail, offhand));
        }
    } else {
        missing.push(MainHand);
        missing.push(OffHand);
    }
    GearSelection {
        items: chosen,
        missing,
    }
}
