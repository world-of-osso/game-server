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
pub fn select_items(
    candidates: &[JournalGearCandidate],
    map: u32,
    class: u8,
    primary: PrimaryStat,
    spec_id: u32,
    known: &KnownSpells,
    spells: &crate::spell_info::SpellStore,
    templates: &ItemTemplates,
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
    let mut two_handed = false;
    // Inventory admission uses native SPELL_EFFECT_DUAL_WIELD40, not one spell ID.
    let can_dual_wield = known.iter().any(|spell| {
        spells
            .get(spell)
            .is_some_and(|info| info.effects.iter().any(|effect| effect.effect == 40))
    });
    let pick = |slot: EquipmentSlot, used: &BTreeSet<u32>, two_hand_only: bool| {
        candidates.iter().find_map(|row| {
            let item = templates.get(ItemDefinitionSource::Retail, row.item_id)?;
            let ranged_hunter =
                class == 3 && slot == MainHand && !matches!(item.inventory_type, 15 | 26);
            let shield_tank = matches!(spec_id, 66 | 73)
                && ((slot == MainHand && !matches!(item.inventory_type, 13 | 21))
                    || (slot == OffHand && item.inventory_type != 14));
            let no_dual_wield =
                slot == OffHand && matches!(item.inventory_type, 13 | 22) && !can_dual_wield;
            let monk_two_hand =
                spec_id == 269 && !can_dual_wield && slot == MainHand && item.inventory_type != 17;
            if used.contains(&row.item_id)
                || !inventory_types(slot).contains(&item.inventory_type)
                || (two_hand_only && !is_two_handed(item.inventory_type))
                || ranged_hunter
                || shield_tank
                || no_dual_wield
                || monk_two_hand
                || !equipment_eligible(item, class, primary, known, templates)
            {
                return None;
            }
            Some((row.item_id, item.inventory_type))
        })
    };
    for slot in [
        Head, Neck, Shoulder, Back, Chest, Wrist, Hands, Waist, Legs, Feet, Finger1, Finger2,
        Trinket1, Trinket2, MainHand, OffHand,
    ] {
        if slot == OffHand && two_handed {
            continue;
        }
        if let Some((item, inventory)) = pick(slot, &used, false) {
            used.insert(item);
            if slot == MainHand {
                two_handed = is_two_handed(inventory);
            }
            chosen.push((slot, ItemDefinitionSource::Retail, item));
        } else if let Some(main) = (slot == OffHand && !matches!(spec_id, 66 | 73))
            .then(|| chosen.iter().position(|(slot, _, _)| *slot == MainHand))
            .flatten()
        {
            // A one-hander with no usable off-hand (a druid: no shield, no dual
            // wield): wield an eligible two-hander instead. Without one, the
            // one-hander alone is a valid loadout; only shield tanks need the slot.
            if let Some((item, _)) = pick(MainHand, &used, true) {
                used.remove(&chosen[main].2);
                used.insert(item);
                chosen[main].2 = item;
            }
        } else {
            missing.push(slot);
        }
    }
    GearSelection {
        items: chosen,
        missing,
    }
}
