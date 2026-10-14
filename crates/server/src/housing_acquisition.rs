//! Qualified Retail item/effect324/decor collection; acquisition remains inventory-owned.
use crate::persistence::{GameDatabase, housing_store::RedemptionRule};
use bevy::prelude::*;
use rusqlite::Connection;
use shared::item_data::ItemDefinitionSource;
use shared::protocol::{InventoryError, InventoryErrorReason, ItemLocation};
use std::collections::BTreeMap;

const COLLECT_DECOR_EFFECT: u32 = 324;

#[derive(Resource)]
pub(crate) struct HousingCollectionRules(pub Result<BTreeMap<u32, RedemptionRule>, String>);

pub(crate) fn read_collection_rules(
    db: &Connection,
) -> Result<BTreeMap<u32, RedemptionRule>, String> {
    let build_key = crate::housing_catalog::read_build_key(db)?;
    let mut query = db.prepare("SELECT CAST(json_extract(h.fields_json,'$.ItemID') AS INTEGER),h.record_id
        FROM housing_catalog_record h JOIN item_x_item_effect x ON x.ItemID=CAST(json_extract(h.fields_json,'$.ItemID') AS INTEGER)
        JOIN item_effect i ON i.ID=x.ItemEffectID JOIN spell_effect e ON e.SpellID=i.SpellID
        WHERE h.build_key=?1 AND h.table_name='HouseDecor' AND i.TriggerType=0 AND i.PlayerConditionID=0
        AND e.EffectIndex=0 AND e.Effect=?2 AND e.EffectMiscValue_0=h.record_id AND e.EffectBasePointsF=0")
        .map_err(|error| format!("Read authenticated decor collect chains: {error}"))?;
    let rows = query
        .query_map(rusqlite::params![build_key, COLLECT_DECOR_EFFECT], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
        })
        .map_err(|error| format!("Query decor collection: {error}"))?;
    let mut rules: BTreeMap<u32, RedemptionRule> = BTreeMap::new();
    for row in rows {
        let (item_id, decor_id) =
            row.map_err(|error| format!("Decode decor collection: {error}"))?;
        if let Some(previous) = rules.get(&item_id)
            && previous.decor_id != decor_id
        {
            return Err(format!(
                "Retail item {item_id} has conflicting decor collect targets."
            ));
        }
        rules.insert(item_id, RedemptionRule { item_id, decor_id });
    }
    Ok(rules)
}

/// Recognize the actual Retail on-use collect effect even if housing catalog loading
/// failed, so a broken housing rule cannot turn into an ordinary unsupported spell cast.
fn load_collectible_item_guid(
    world: &World,
    character_id: u64,
    location: ItemLocation,
) -> Result<Option<u64>, InventoryError> {
    let (db, templates) = crate::inventory::use_store(world)?;
    let model = crate::inventory::load_model(db, templates, character_id)?;
    let item = model.existing(location)?;
    if model.definition_source(item.item_guid) != ItemDefinitionSource::Retail {
        return Ok(None);
    }
    let spell = templates
        .get(ItemDefinitionSource::Retail, item.item_id)
        .and_then(|template| template.on_use)
        .and_then(|on_use| {
            world
                .resource::<crate::spell_info::SpellStore>()
                .get(on_use.spell_id)
        });
    let collects = spell.is_some_and(|spell| {
        spell
            .effects
            .iter()
            .any(|effect| effect.effect == COLLECT_DECOR_EFFECT)
    });
    Ok(collects.then_some(item.item_guid))
}

/// Normal bag UseItem invokes this before the non-collection spell path.
pub(crate) fn collect_item(
    world: &mut World,
    character_id: u64,
    player: Entity,
    location: ItemLocation,
) -> Result<bool, InventoryError> {
    let Some(guid) = load_collectible_item_guid(world, character_id, location)? else {
        return Ok(false);
    };
    let fail = |detail: String| InventoryError {
        reason: InventoryErrorReason::CantDoRightNow,
        detail: Some(detail),
    };
    crate::housing::collect_authenticated_item(world, character_id, player, guid).map_err(fail)?;
    Ok(true)
}

fn require_item_eligibility(world: &World, player: Entity, item_id: u32) -> Result<(), String> {
    let info = world
        .get::<shared::components::Player>(player)
        .ok_or("Housing collector has no player identity.")?;
    let level = world
        .get::<shared::components::Level>(player)
        .ok_or("Housing collector has no level.")?
        .0;
    let template = world
        .resource::<crate::item_templates::ItemTemplates>()
        .get(ItemDefinitionSource::Retail, item_id)
        .ok_or_else(|| format!("Retail decor item {item_id} definition is unavailable."))?;
    if !template.allows(info.class, info.race) || u16::from(level) < template.required_level {
        return Err("You do not meet this decor item's requirements.".into());
    }
    Ok(())
}

pub(crate) fn redeem(
    world: &mut World,
    actor: crate::persistence::housing_store::Actor,
    player: Entity,
    guid: u64,
) -> Result<(), String> {
    let inventory = crate::persistence::load_inventory(
        &world.resource::<GameDatabase>().db,
        actor.character_id,
    );
    let item = inventory
        .items
        .iter()
        .find(|item| item.item_guid == guid)
        .ok_or("You do not own this decor item.")?;
    let rule = world
        .resource::<HousingCollectionRules>()
        .0
        .as_ref()
        .map_err(Clone::clone)?
        .get(&item.item_id)
        .ok_or("This item has no verified Retail decor collect chain.")?;
    require_item_eligibility(world, player, rule.item_id)?;
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
