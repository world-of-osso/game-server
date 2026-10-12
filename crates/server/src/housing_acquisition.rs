//! One verified Retail collect-item chain; no generic catalog/default grants.
use crate::persistence::{
    GameDatabase,
    housing_store::{BUILD_KEY, RedemptionRule},
};
use bevy::prelude::*;
use rusqlite::Connection;
use shared::item_data::ItemDefinitionSource;
use shared::protocol::{InventoryError, InventoryErrorReason, ItemLocation};

const BOOKCASE_ITEM: u32 = 245259;
const BOOKCASE_DECOR: u32 = 1693;
const PAWDO: u32 = 252312;
const COLLECT_BOOKCASE: u32 = 1238340;
const COLLECT_DECOR_EFFECT: u32 = 324;

#[derive(Resource)]
pub(crate) struct HousingCollectionRule(pub Result<RedemptionRule, String>);

pub(crate) fn read_bookcase_rule(db: &Connection) -> Result<RedemptionRule, String> {
    let build: String = db
        .query_row(
            "SELECT build FROM housing_catalog_build WHERE build_key=?",
            [BUILD_KEY],
            |row| row.get(0),
        )
        .map_err(|error| format!("Read housing collection build: {error}"))?;
    if build != "12.1.0.69933" {
        return Err(format!(
            "Housing collection requires build 12.1.0.69933, found {build}."
        ));
    }
    require_decor_link(db)?;
    require_collect_chain(db)?;
    Ok(RedemptionRule {
        item_id: BOOKCASE_ITEM,
        decor_id: BOOKCASE_DECOR,
    })
}

fn require_decor_link(db: &Connection) -> Result<(), String> {
    let json: String = db.query_row("SELECT fields_json FROM housing_catalog_record WHERE build_key=? AND table_name='HouseDecor' AND record_id=?", rusqlite::params![BUILD_KEY, BOOKCASE_DECOR], |row| row.get(0)).map_err(|error| format!("Read authenticated bookcase decor: {error}"))?;
    let fields: std::collections::BTreeMap<String, String> =
        serde_json::from_str(&json).map_err(|error| format!("Decode bookcase decor: {error}"))?;
    if fields
        .get("ItemID")
        .and_then(|value| value.parse::<u32>().ok())
        != Some(BOOKCASE_ITEM)
    {
        return Err("Bookcase decor does not reference the verified Retail item.".into());
    }
    Ok(())
}

fn require_collect_chain(db: &Connection) -> Result<(), String> {
    let verified: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM item_x_item_effect x JOIN item_effect i ON i.ID=x.ItemEffectID JOIN spell_effect e ON e.SpellID=i.SpellID WHERE x.ItemID=?1 AND i.SpellID=?2 AND i.TriggerType=0 AND i.PlayerConditionID=0 AND e.EffectIndex=0 AND e.Effect=?3 AND e.EffectMiscValue_0=?4 AND e.EffectBasePointsF=0) AND EXISTS(SELECT 1 FROM content_npc_vendor WHERE entry=?5 AND item=?1 AND ExtendedCost=0)",
        rusqlite::params![BOOKCASE_ITEM, COLLECT_BOOKCASE, COLLECT_DECOR_EFFECT, BOOKCASE_DECOR, PAWDO], |row| row.get(0)
    ).map_err(|error| format!("Verify bookcase vendor/collect effect: {error}"))?;
    if !verified {
        return Err("The verified bookcase vendor/collect-effect chain is unavailable.".into());
    }
    Ok(())
}

fn bookcase_guid(
    world: &World,
    character_id: u64,
    location: ItemLocation,
) -> Result<Option<u64>, InventoryError> {
    let (db, templates) = crate::inventory::use_store(world)?;
    let model = crate::inventory::load_model(db, templates, character_id)?;
    let item = model.existing(location)?;
    let retail = model.definition_source(item.item_guid) == ItemDefinitionSource::Retail;
    Ok((retail && item.item_id == BOOKCASE_ITEM).then_some(item.item_guid))
}

/// Normal bag UseItem invokes this before the non-collection spell path.
pub(crate) fn collect_item(
    world: &mut World,
    character_id: u64,
    player: Entity,
    location: ItemLocation,
) -> Result<bool, InventoryError> {
    let Some(guid) = bookcase_guid(world, character_id, location)? else {
        return Ok(false);
    };
    let fail = |detail: String| InventoryError {
        reason: InventoryErrorReason::CantDoRightNow,
        detail: Some(detail),
    };
    crate::housing::collect_authenticated_item(world, character_id, player, guid).map_err(fail)?;
    Ok(true)
}

pub(crate) fn require_item_eligibility(world: &World, player: Entity) -> Result<(), String> {
    let info = world
        .get::<shared::components::Player>(player)
        .ok_or("Housing collector has no player identity.")?;
    let level = world
        .get::<shared::components::Level>(player)
        .ok_or("Housing collector has no level.")?
        .0;
    let template = world
        .resource::<crate::item_templates::ItemTemplates>()
        .get(ItemDefinitionSource::Retail, BOOKCASE_ITEM)
        .ok_or("Retail bookcase item definition is unavailable.")?;
    if !template.allows(info.class, info.race) || u16::from(level) < template.required_level {
        return Err("You do not meet this decor item's requirements.".into());
    }
    Ok(())
}

pub(crate) fn redeem(
    world: &mut World,
    actor: crate::persistence::housing_store::Actor,
    guid: u64,
) -> Result<(), String> {
    let rule = world
        .resource::<HousingCollectionRule>()
        .0
        .as_ref()
        .map_err(Clone::clone)?;
    crate::persistence::housing_store::redeem(
        &world.resource::<GameDatabase>().db,
        actor,
        guid,
        rule,
    )?;
    world
        .resource_mut::<crate::gold::GoldUpdateQueue>()
        .record_items(actor.character_id);
    Ok(())
}

#[cfg(test)]
#[path = "housing_acquisition_tests.rs"]
pub(crate) mod tests;
