//! Shared player profile inputs for parity and follower actors.
use bevy::prelude::*;
use serde::Deserialize;
use shared::protocol::EquipmentSlot;
use std::collections::BTreeMap;

use crate::item_instance::ItemInstanceModifiers;

#[derive(Deserialize)]
pub struct Profile {
    pub spec_id: u32,
    pub level: String,
    pub race: String,
    pub equipment: Vec<Equipment>,
    pub consumables: BTreeMap<String, String>,
    pub reference_gap: Option<String>,
    pub omnium_talents: Option<String>,
    #[serde(flatten)]
    pub options: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
pub struct Equipment {
    pub slot: String,
    pub item_id: u32,
    pub bonus_list: Vec<u32>,
    pub item_level_override: Option<u16>,
    pub enchant: u32,
    pub gems: Vec<u32>,
    pub crafted_stats: Vec<i8>,
    pub options: BTreeMap<String, String>,
}

/// SimC76f8118 sc_hunter.cpp1052 defaults `summon_pet` to duck.
pub fn simc_hunter_pet(profile: &Profile) -> Result<Option<u32>, String> {
    if !matches!(profile.spec_id, 253..=255) {
        return Ok(None);
    }
    let Some(value) = profile.options.get("summon_pet") else {
        return Ok(Some(174884));
    };
    let alias = value.as_str().ok_or("SimC summon_pet must be a string")?;
    match alias {
        "duck" => Ok(Some(174884)),
        "none" => Ok(None),
        _ => Err(format!("unsupported SimC summon_pet alias {alias:?}")),
    }
}

/// Resolve a SimC warlock's configured pet through its native summon effect.
pub fn simc_warlock_pet(world: &World, profile: &Profile) -> Result<Option<u32>, String> {
    if !matches!(profile.spec_id, 265..=267) {
        return Ok(None);
    }
    let Some(value) = profile.options.get("warlock.default_pet") else {
        return Ok(None);
    };
    let name = value
        .as_str()
        .ok_or("warlock.default_pet must be a string")?;
    let spell_id = match name {
        "imp" => 688,
        "felhunter" => 691,
        "voidwalker" => 697,
        "sayaad" => 366222,
        "felguard" => 30146,
        "none" => return Ok(None),
        _ => return Err(format!("unsupported warlock.default_pet {name:?}")),
    };
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .ok_or_else(|| format!("native summon spell{spell_id} missing"))?;
    let entries: Vec<_> = spell
        .effects
        .iter()
        .filter(|effect| effect.effect == crate::hunter_pet::EFFECT_SUMMON_PET)
        .filter_map(|effect| u32::try_from(effect.misc_values[0]).ok())
        .filter(|entry| *entry != 0)
        .collect();
    match entries.as_slice() {
        [entry] => Ok(Some(*entry)),
        _ => Err(format!(
            "native summon spell{spell_id} has no unique pet entry"
        )),
    }
}

// SimC76f8118 item.cpp2090–2111 resolves explicit names through the pinned
// embellishment table; native ItemBonus then resolves the equipped effect.
pub fn embellishment_bonus(name: &str) -> Option<u32> {
    include_str!("embellishments_76f8118.csv")
        .lines()
        .skip(2)
        .filter_map(|line| line.split_once(','))
        .find(|(alias, _)| alias.eq_ignore_ascii_case(name))
        .and_then(|(_, bonus)| bonus.parse().ok())
}

impl Equipment {
    fn stat_redirect(&self, world: &World) -> Result<Option<(EquipmentSlot, u32)>, String> {
        let Some(value) = self.options.get("redirected_base_stats") else {
            return Ok(None);
        };
        let id: u32 = value.parse().map_err(|error| {
            format!(
                "reference {} item{} redirected_base_stats {value:?}: {error}",
                self.slot, self.item_id
            )
        })?;
        // Pinned item.cpp95/1314: zero is the explicit no-redirect value.
        if id == 0 {
            return Ok(None);
        }
        let templates = world
            .get_resource::<crate::item_templates::ItemTemplates>()
            .ok_or("redirected_base_stats requires native item catalogs")?;
        if templates
            .get(shared::item_data::ItemDefinitionSource::Retail, id)
            .is_none()
        {
            return Err(format!(
                "reference {} item{}: native redirected_base_stats item{id} absent",
                self.slot, self.item_id
            ));
        }
        Ok(Some((slot(&self.slot)?, id)))
    }

    pub fn modifiers(&self) -> ItemInstanceModifiers {
        let mut bonus_list = self.bonus_list.clone();
        if let Some(bonus) = self
            .options
            .get("embellishment")
            .and_then(|name| embellishment_bonus(name))
            && !bonus_list.contains(&bonus)
        {
            bonus_list.push(bonus);
        }
        ItemInstanceModifiers {
            bonus_list,
            item_level_override: self.item_level_override,
            enchant: self.enchant,
            gems: self.gems.clone(),
            crafted_stats: self.crafted_stats.clone(),
        }
    }
}

pub fn slot(name: &str) -> Result<EquipmentSlot, String> {
    use EquipmentSlot::*;
    match name {
        "head" => Ok(Head),
        "neck" => Ok(Neck),
        "shoulders" => Ok(Shoulder),
        "back" => Ok(Back),
        "chest" => Ok(Chest),
        "wrists" => Ok(Wrist),
        "hands" => Ok(Hands),
        "waist" => Ok(Waist),
        "legs" => Ok(Legs),
        "feet" => Ok(Feet),
        "finger1" => Ok(Finger1),
        "finger2" => Ok(Finger2),
        "trinket1" => Ok(Trinket1),
        "trinket2" => Ok(Trinket2),
        "main_hand" => Ok(MainHand),
        "off_hand" => Ok(OffHand),
        _ => Err(format!("unsupported reference equipment slot {name}")),
    }
}

#[derive(Clone, Debug)]
pub struct ItemInstances {
    pub items: Vec<(EquipmentSlot, shared::item_data::ItemDefinitionSource, u32)>,
    pub modifiers: crate::stats::EquippedItemModifiers,
}

pub fn profile_instances(world: &World, profile: &Profile) -> Result<ItemInstances, String> {
    let items = profile
        .equipment
        .iter()
        .map(|item| {
            Ok((
                slot(&item.slot)?,
                shared::item_data::ItemDefinitionSource::Retail,
                item.item_id,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let modifiers = profile
        .equipment
        .iter()
        .map(|item| Ok((slot(&item.slot)?, item.modifiers())))
        .collect::<Result<Vec<_>, String>>()?;
    let base_stat_redirects = profile
        .equipment
        .iter()
        .filter_map(|item| item.stat_redirect(world).transpose())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ItemInstances {
        items,
        modifiers: crate::stats::EquippedItemModifiers {
            items: modifiers,
            base_stat_redirects,
            ..Default::default()
        },
    })
}

pub fn equip(world: &mut World, unit: Entity, profile: &Profile) -> Result<(), String> {
    let instances = profile_instances(world, profile)?;
    equip_instances(world, unit, instances.items, instances.modifiers);
    Ok(())
}

/// Install physical item identities and instance modifiers through the common stat path.
pub fn equip_instances(
    world: &mut World,
    unit: Entity,
    items: Vec<(EquipmentSlot, shared::item_data::ItemDefinitionSource, u32)>,
    modifiers: crate::stats::EquippedItemModifiers,
) {
    world
        .entity_mut(unit)
        .insert((crate::stats::EquippedItems { items }, modifiers));
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::item_data::ItemDefinitionSource;

    #[test]
    fn simc_warlock_pet_preserves_each_profile_selection() {
        use crate::spell_info::{SpellEffect, SpellInfo, SpellStore};

        let mut world = World::new();
        world.insert_resource(SpellStore::fixture(
            [(688, 416), (691, 417), (30146, 17252), (366222, 1863)]
                .into_iter()
                .map(|(id, entry)| SpellInfo {
                    id,
                    effects: vec![SpellEffect {
                        effect: crate::hunter_pet::EFFECT_SUMMON_PET,
                        misc_values: [entry, 0],
                        ..Default::default()
                    }],
                    ..Default::default()
                })
                .collect(),
        ));
        for (spec, name, entry) in [
            (265, "sayaad", 1863),
            (266, "felguard", 17252),
            (267, "imp", 416),
            (267, "felhunter", 417),
        ] {
            let profile: Profile = serde_json::from_value(serde_json::json!({
                "spec_id": spec, "level": "90", "race": "human",
                "equipment": [], "consumables": {}, "warlock.default_pet": name
            }))
            .unwrap();
            assert_eq!(simc_warlock_pet(&world, &profile).unwrap(), Some(entry));
        }
    }

    #[test]
    fn simc_hunter_pet_resolves_explicit_and_default_aliases() {
        let mut profile: Profile = serde_json::from_str(
            r#"{ "spec_id": 253, "level": "90", "race": "human", "equipment": [], "consumables": {} }"#,
        ).unwrap();
        assert_eq!(simc_hunter_pet(&profile).unwrap(), Some(174884));
        profile
            .options
            .insert("summon_pet".into(), serde_json::json!("duck"));
        assert_eq!(simc_hunter_pet(&profile).unwrap(), Some(174884));
        profile
            .options
            .insert("summon_pet".into(), serde_json::json!("none"));
        assert_eq!(simc_hunter_pet(&profile).unwrap(), None);
        profile
            .options
            .insert("summon_pet".into(), serde_json::json!("wolf"));
        assert_eq!(
            simc_hunter_pet(&profile).unwrap_err(),
            "unsupported SimC summon_pet alias \"wolf\""
        );
    }

    #[test]
    fn class_follower_profile_equips_real_item_and_modifiers() {
        let profile: Profile = serde_json::from_str(
            r#"{
            "spec_id": 70, "level": "80", "race": "human",
            "consumables": {}, "equipment": [{"slot": "main_hand", "item_id": 268213,
            "bonus_list": [6652, 13334], "item_level_override": 334, "enchant": 8689,
            "gems": [240892], "crafted_stats": [], "options": {}}]
        }"#,
        )
        .unwrap();
        let mut world = World::new();
        let unit = world.spawn_empty().id();
        equip(&mut world, unit, &profile).unwrap();
        let gear = world.get::<crate::stats::EquippedItems>(unit).unwrap();
        assert_eq!(
            gear.items,
            vec![(
                EquipmentSlot::MainHand,
                ItemDefinitionSource::Retail,
                268213
            )]
        );
        let modifiers = &world
            .get::<crate::stats::EquippedItemModifiers>(unit)
            .unwrap()
            .items[0]
            .1;
        assert_eq!(modifiers.bonus_list, vec![6652, 13334]);
        assert_eq!(modifiers.item_level_override, Some(334));
        assert_eq!(modifiers.enchant, 8689);
        assert_eq!(modifiers.gems, vec![240892]);
    }
}
