//! Read exact external reference inputs; unsupported options remain reportable data.
use bevy::prelude::*;
use serde::Deserialize;
use server::item_instance::{ItemInstanceModifiers, load};
use shared::protocol::EquipmentSlot;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
pub(super) struct Profile {
    pub spec_id: u32,
    pub level: String,
    pub race: String,
    pub equipment: Vec<Equipment>,
    pub consumables: BTreeMap<String, String>,
    pub reference_gap: Option<String>,
    pub omnium_talents: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Equipment {
    pub slot: String,
    pub item_id: u32,
    pub bonus_list: Vec<u32>,
    pub item_level_override: Option<u16>,
    pub enchant: u32,
    pub gems: Vec<u32>,
    pub crafted_stats: Vec<i8>,
    pub options: BTreeMap<String, String>,
}

impl Equipment {
    pub fn modifiers(&self) -> ItemInstanceModifiers {
        ItemInstanceModifiers {
            bonus_list: self.bonus_list.clone(),
            item_level_override: self.item_level_override,
            enchant: self.enchant,
            gems: self.gems.clone(),
            crafted_stats: self.crafted_stats.clone(),
        }
    }
}

#[derive(Deserialize)]
pub(super) struct AllocatedTrait {
    pub node_id: u32,
    pub entry_id: u32,
    pub ranks: u32,
    pub active_ranks: u32,
}

pub(super) fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

pub(super) fn directory() -> Result<PathBuf, String> {
    std::env::var_os("DPS_PARITY_OUTPUT")
        .map(PathBuf::from)
        .ok_or_else(|| "DPS_PARITY_OUTPUT must name the prepared profile/evidence directory".into())
}

pub(super) fn slot(name: &str) -> Result<EquipmentSlot, String> {
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

pub(super) fn install_reference_catalogs(app: &mut App) -> Result<(), String> {
    let path = std::env::var_os("DPS_PARITY_WORLD_DB")
        .map(PathBuf::from)
        .ok_or("DPS_PARITY_WORLD_DB must name the private native database")?;
    let db =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| format!("open private database {}: {error}", path.display()))?;
    let followers = crate::followers::simulator::load_parity_catalog(&db)
        .map_err(|error| format!("load private follower catalog: {error}"))?;
    app.insert_resource(followers);
    let native = load_native_catalogs(&db)?;
    app.insert_resource(native);
    app.insert_resource(
        crate::item_templates::load_item_templates(&path).map_err(|error| error.to_string())?,
    );
    app.insert_resource(
        crate::item_stats::load_item_scaling_tables(&path).map_err(|error| error.to_string())?,
    );
    Ok(())
}

fn load_native_catalogs(
    db: &rusqlite::Connection,
) -> Result<crate::stats::NativeItemInstances, String> {
    Ok(crate::stats::NativeItemInstances {
        bonuses: load::load_bonus_tables(db).map_err(|error| error.to_string())?,
        enchants: load::load_enchantment_tables(db).map_err(|error| error.to_string())?,
        effects: load::load_item_effect_tables(db).map_err(|error| error.to_string())?,
    })
}

pub(super) fn equip(world: &mut World, unit: Entity, profile: &Profile) -> Result<(), String> {
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
    world.entity_mut(unit).insert((
        crate::stats::EquippedItems { items },
        crate::stats::EquippedItemModifiers { items: modifiers },
    ));
    Ok(())
}

pub(super) fn install_traits(
    world: &mut World,
    unit: Entity,
    allocated: &[AllocatedTrait],
) -> Result<(), String> {
    let follower = world.get::<crate::followers::Follower>(unit).unwrap();
    let level = world.get::<shared::components::Level>(unit).unwrap().0;
    if allocated
        .iter()
        .any(|entry| entry.active_ranks > entry.ranks)
    {
        return Err("active trait ranks exceed exported allocations".into());
    }
    let purchases: Vec<_> = allocated
        .iter()
        .filter(|entry| entry.active_ranks > 0)
        .map(|entry| crate::trait_config::Purchase {
            node_id: entry.node_id,
            entry_id: entry.entry_id,
            ranks: entry.active_ranks,
        })
        .collect();
    let (known, traits) = crate::class_progression::evaluate_reference_traits(
        world.resource::<crate::spell_info::ClassSpells>(),
        &follower.profile,
        level,
        &purchases,
    )
    .map_err(|reason| format!("NATIVE_REFERENCE_TRAIT_CONFIG_REJECTED: {reason}"))?;
    world.entity_mut(unit).insert((known, traits));
    Ok(())
}
