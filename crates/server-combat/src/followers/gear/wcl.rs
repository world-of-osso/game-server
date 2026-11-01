//! Qualified Warcraft Logs profiles for keystone follower equipment and talents.
#[cfg(test)]
#[path = "wcl_tests.rs"]
mod tests;
use std::collections::HashMap;
use std::sync::OnceLock;

use bevy::prelude::*;
use serde::Deserialize;
use shared::protocol::EquipmentSlot;

use crate::item_instance::ItemInstanceModifiers;

#[derive(Deserialize)]
struct Catalog {
    profiles: HashMap<String, Profile>,
}

#[derive(Deserialize)]
pub(super) struct Profile {
    spec_id: u32,
    class_id: u8,
    key_band: String,
    race: Race,
    talents: Vec<Talent>,
    gear: Vec<Item>,
}

#[derive(Deserialize)]
struct Race {
    id: u8,
    source: String,
}

#[derive(Deserialize)]
struct Talent {
    #[serde(rename = "talentID")]
    entry: u32,
    points: u32,
}

#[derive(Deserialize)]
struct Item {
    id: u32,
    #[serde(rename = "bonusIDs", default)]
    bonuses: Vec<String>,
    #[serde(rename = "permanentEnchant", default)]
    enchant: String,
    #[serde(default)]
    gems: Vec<Gem>,
}

#[derive(Deserialize)]
struct Gem {
    id: String,
}

fn catalog() -> Result<&'static Catalog, String> {
    static CATALOG: OnceLock<Result<Catalog, String>> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../../../scripts/reference/follower-wcl-candidates.json"
            ))
            .map_err(|error| format!("decode qualified keystone follower profiles: {error}"))
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn band_contains(band: &str, level: u32) -> Result<bool, String> {
    if let Some(lower) = band.strip_suffix('+') {
        let lower: u32 = lower
            .parse()
            .map_err(|_| format!("invalid key band {band}"))?;
        return Ok(level >= lower);
    }
    let (lower, upper) = band
        .split_once('-')
        .ok_or_else(|| format!("invalid key band {band}"))?;
    let lower: u32 = lower
        .parse()
        .map_err(|_| format!("invalid key band {band}"))?;
    let upper: u32 = upper
        .parse()
        .map_err(|_| format!("invalid key band {band}"))?;
    Ok((lower..=upper).contains(&level))
}

pub(super) fn find_profile(spec: u32, level: u32) -> Result<&'static Profile, String> {
    let mut matching = Vec::new();
    for profile in catalog()?
        .profiles
        .values()
        .filter(|profile| profile.spec_id == spec)
    {
        if band_contains(&profile.key_band, level)? {
            matching.push(profile);
        }
    }
    match matching.as_slice() {
        [profile] => Ok(profile),
        _ => Err(format!(
            "expected one qualified keystone profile for spec{spec} key{level}, found {}",
            matching.len()
        )),
    }
}

pub(in crate::followers) fn class_profiles(
    classes: &crate::spell_info::ClassSpells,
    level: u32,
) -> Result<Vec<super::super::class_profiles::ClassProfile>, String> {
    let mut profiles = Vec::new();
    for profile in catalog()?.profiles.values() {
        if !band_contains(&profile.key_band, level)? {
            continue;
        }
        let spec = classes
            .classes
            .iter()
            .find(|class| class.class_id == profile.class_id)
            .and_then(|class| {
                class
                    .specs
                    .iter()
                    .find(|spec| spec.spec_id == profile.spec_id)
            })
            .ok_or_else(|| {
                format!(
                    "keystone profile spec{} lacks native class metadata",
                    profile.spec_id
                )
            })?;
        let role =
            u8::try_from(spec.role).map_err(|_| format!("invalid spec{} role", profile.spec_id))?;
        profiles.push(super::super::class_profiles::ClassProfile {
            spec_id: profile.spec_id,
            class_id: profile.class_id,
            role,
            race: profile.race.id,
            talents: "TALENTS_FROM_WCL".into(),
            race_source: profile.race.source.clone(),
            talents_source: Some("TALENTS_FROM_WCL".into()),
            talent_entries: Some(
                profile
                    .talents
                    .iter()
                    .map(|talent| (talent.entry, talent.points))
                    .collect(),
            ),
            rejection: None,
        });
    }
    profiles.sort_by_key(|profile| profile.spec_id);
    if profiles
        .windows(2)
        .any(|pair| pair[0].spec_id == pair[1].spec_id)
    {
        return Err(format!(
            "overlapping qualified follower profiles for key{level}"
        ));
    }
    Ok(profiles)
}

pub(in crate::followers) fn class_profile(
    base: &super::super::class_profiles::ClassProfile,
    level: u32,
) -> Result<super::super::class_profiles::ClassProfile, String> {
    let profile = find_profile(base.spec_id, level)?;
    if profile.class_id != base.class_id {
        return Err(format!(
            "keystone profile spec{} has wrong class",
            base.spec_id
        ));
    }
    let mut selected = base.clone();
    selected.race = profile.race.id;
    selected.race_source.clone_from(&profile.race.source);
    selected.talents_source = Some("TALENTS_FROM_WCL".into());
    selected.talent_entries = Some(
        profile
            .talents
            .iter()
            .map(|talent| (talent.entry, talent.points))
            .collect(),
    );
    Ok(selected)
}

// WCL Character.gear uses Blizzard equipment slot order, including shirt and tabard.
const SLOTS: [Option<EquipmentSlot>; 18] = [
    Some(EquipmentSlot::Head),
    Some(EquipmentSlot::Neck),
    Some(EquipmentSlot::Shoulder),
    None,
    Some(EquipmentSlot::Chest),
    Some(EquipmentSlot::Waist),
    Some(EquipmentSlot::Legs),
    Some(EquipmentSlot::Feet),
    Some(EquipmentSlot::Wrist),
    Some(EquipmentSlot::Hands),
    Some(EquipmentSlot::Finger1),
    Some(EquipmentSlot::Finger2),
    Some(EquipmentSlot::Trinket1),
    Some(EquipmentSlot::Trinket2),
    Some(EquipmentSlot::Back),
    Some(EquipmentSlot::MainHand),
    Some(EquipmentSlot::OffHand),
    None,
];

fn number(value: &str, context: &str) -> Result<u32, String> {
    value
        .parse()
        .map_err(|_| format!("invalid {context}: {value:?}"))
}

pub(in crate::followers) fn equip(
    world: &mut World,
    actor: Entity,
    spec: u32,
    level: u32,
) -> Result<(), String> {
    let profile = find_profile(spec, level)?;
    if profile.gear.len() != SLOTS.len() {
        return Err(format!(
            "spec{spec} has {} gear slots, expected {}",
            profile.gear.len(),
            SLOTS.len()
        ));
    }
    let mut items = Vec::new();
    let mut modifiers = Vec::new();
    for (slot, item) in SLOTS.into_iter().zip(&profile.gear) {
        let Some(slot) = slot else { continue };
        if item.id == 0 {
            continue;
        }
        let bonus_list = item
            .bonuses
            .iter()
            .map(|id| number(id, "bonus ID"))
            .collect::<Result<Vec<_>, _>>()?;
        let gems = item
            .gems
            .iter()
            .map(|gem| number(&gem.id, "gem ID"))
            .collect::<Result<Vec<_>, _>>()?;
        let enchant = if item.enchant.is_empty() {
            0
        } else {
            number(&item.enchant, "enchantment ID")?
        };
        items.push((
            slot,
            shared::item_data::ItemDefinitionSource::Retail,
            item.id,
        ));
        modifiers.push((
            slot,
            ItemInstanceModifiers {
                bonus_list,
                enchant,
                gems,
                ..Default::default()
            },
        ));
    }
    super::super::parity_profile::equip_instances(
        world,
        actor,
        items,
        crate::stats::EquippedItemModifiers {
            items: modifiers,
            ..Default::default()
        },
    );
    Ok(())
}
