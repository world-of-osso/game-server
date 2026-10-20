//! Pinned local DB2 catalog; unknown semantics stay unavailable rather than inferred.
use crate::trading_post_store::Grant;
use bevy::prelude::*;
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use shared::protocol::{TradingPostCategory, TradingPostItem};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Resource)]
pub(crate) struct Catalog(pub Result<CatalogData, String>);
impl Default for Catalog {
    fn default() -> Self {
        Self(Err("Trading Post catalog not loaded".into()))
    }
}

pub(crate) struct CatalogData {
    pub categories: Vec<TradingPostCategory>,
    pub items: BTreeMap<u32, (TradingPostItem, Result<Grant, String>)>,
    pub monthly: Arc<super::monthly::Catalog>,
}

#[derive(Deserialize)]
struct Category {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "DisplayName_lang")]
    name: String,
    #[serde(rename = "DefaultUIModelSceneID")]
    scene: u32,
}
#[derive(Deserialize)]
struct Vendor {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "PerksVendorCategoryID")]
    category: u32,
    #[serde(rename = "ItemID")]
    item: u32,
    #[serde(rename = "CreatureDisplayInfoID")]
    display: u32,
    #[serde(rename = "Cost")]
    cost: u32,
    #[serde(rename = "UiModelSceneID")]
    scene: u32,
    #[serde(rename = "UiGroupInfo")]
    group: u32,
}
#[derive(Deserialize)]
struct Group {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "Name_lang")]
    name: String,
    #[serde(rename = "Priority")]
    priority: i32,
}
fn read_rows<T: serde::de::DeserializeOwned>(
    conn: &Connection,
    table: &str,
) -> Result<Vec<T>, String> {
    let mut query = conn
        .prepare(&format!("SELECT payload FROM {table} ORDER BY ID"))
        .map_err(|error| format!("Trading Post {table}: {error}"))?;
    let rows = query
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    rows.map(|row| {
        serde_json::from_str(&row.map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    })
    .collect()
}

pub(crate) fn load(conn: &Connection, monthly_path: &Path) -> Result<CatalogData, String> {
    let build = crate::db2_source_build::read_source_build(conn, "trading_post_source")?;
    let categories = read_rows::<Category>(conn, "perks_vendor_category")?
        .into_iter()
        .map(|row| TradingPostCategory {
            id: row.id,
            name: row.name,
            model_scene_id: row.scene,
        })
        .collect();
    let groups: BTreeMap<_, _> = read_rows::<Group>(conn, "perks_vendor_item_ui_group")?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    let items: BTreeMap<_, _> = read_rows::<Vendor>(conn, "perks_vendor_item")?
        .into_iter()
        .map(|vendor| {
            let (row, grant) = read_item(conn, &vendor, groups.get(&vendor.group));
            (vendor.id, (row, grant))
        })
        .collect();
    let monthly = super::monthly::load(monthly_path, &build, &items.keys().copied().collect())?;
    Ok(CatalogData {
        categories,
        items,
        monthly: Arc::new(monthly),
    })
}

fn read_item(
    conn: &Connection,
    vendor: &Vendor,
    group: Option<&Group>,
) -> (TradingPostItem, Result<Grant, String>) {
    let metadata = conn.query_row(r#"SELECT s.Display_lang, s.Description_lang,
        CASE WHEN a.ID IS NULL THEN i.IconFileDataID ELSE a.DefaultIconFileDataID END,
        s.OverallQualityID
        FROM item_sparse s JOIN item i ON i.ID=s.ID
        LEFT JOIN item_appearance a ON a.ID=(
            SELECT ma.ItemAppearanceID
            FROM item_modified_appearance ma JOIN item_appearance base ON base.ID=ma.ItemAppearanceID
            WHERE ma.ItemID=s.ID AND base.DefaultIconFileDataID<>0
            ORDER BY ma.OrderIndex, ma.ID LIMIT 1
        ) WHERE s.ID=?1"#, [vendor.item], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,u32>(2)?,row.get::<_,u8>(3)?))).optional();
    let mut row = TradingPostItem {
        id: vendor.id,
        category_id: vendor.category,
        item_id: vendor.item,
        name: String::new(),
        description: String::new(),
        icon_fdid: 0,
        quality: 0,
        usable: None,
        price: vendor.cost,
        purchased: false,
        collected: false,
        refund_seconds: 0,
        ui_group_name: group.map_or_else(String::new, |group| group.name.clone()),
        ui_group_priority: group.map_or(0, |group| group.priority),
        model_scene_id: vendor.scene,
        creature_display_id: vendor.display,
        preview_item_ids: vec![vendor.item],
        grant_unavailable: None,
    };
    let grant = match metadata {
        Ok(Some((name, description, icon, quality))) => {
            row.name = name;
            row.description = description;
            row.icon_fdid = icon;
            row.quality = quality;
            read_grant(conn, vendor, &mut row)
                .map_err(|error| format!("Vendor{} item{}: {error}", vendor.id, vendor.item))
        }
        Ok(None) => Err(format!("ItemSparse/Item row{} unavailable", vendor.item)),
        Err(error) => Err(format!("Trading Post item metadata: {error}")),
    };
    row.grant_unavailable = grant.as_ref().err().cloned();
    (row, grant)
}

fn read_grant(
    conn: &Connection,
    vendor: &Vendor,
    row: &mut TradingPostItem,
) -> Result<Grant, String> {
    match vendor.category {
        1 => {
            let appearance=conn.query_row("SELECT ItemAppearanceID FROM item_modified_appearance WHERE ItemID=?1 AND ItemAppearanceModifierID=0 ORDER BY OrderIndex,ID LIMIT 1",[vendor.item],|row|row.get(0)).map_err(|error|error.to_string())?;
            Ok(Grant::Appearances(vec![appearance]))
        }
        2 => {
            let mount=conn.query_row("SELECT m.ID,COALESCE((SELECT d.CreatureDisplayInfoID FROM mount_x_display d WHERE d.MountID=m.ID AND d.PlayerConditionID=0 ORDER BY d.ID LIMIT 1),0) FROM item_x_item_effect x JOIN item_effect e ON e.ID=x.ItemEffectID JOIN mount m ON m.SourceSpellID=e.SpellID WHERE x.ItemID=?1 AND e.TriggerType=6 ORDER BY e.ID LIMIT 1",[vendor.item],|row|Ok((row.get::<_,u32>(0)?,row.get::<_,u32>(1)?))).map_err(|error|error.to_string())?;
            if row.creature_display_id == 0 {
                row.creature_display_id = mount.1;
            }
            row.preview_item_ids.clear();
            Ok(Grant::Mount(mount.0))
        }
        3 => {
            let species=conn.query_row("SELECT s.species_id,s.display_id FROM item_x_item_effect x JOIN item_effect e ON e.ID=x.ItemEffectID JOIN battle_pet_species s ON s.summon_spell_id=e.SpellID WHERE x.ItemID=?1 AND e.TriggerType=6 ORDER BY e.ID LIMIT 1",[vendor.item],|row|Ok((row.get::<_,u32>(0)?,row.get::<_,u32>(1)?))).map_err(|error|error.to_string())?;
            if row.creature_display_id == 0 {
                row.creature_display_id = species.1;
            }
            row.preview_item_ids.clear();
            Ok(Grant::Pet(species.0))
        }
        5 => {
            conn.query_row(
                "SELECT ItemID FROM toy WHERE ItemID=?1",
                [vendor.item],
                |row| row.get::<_, u32>(0),
            )
            .map_err(|error| error.to_string())?;
            Ok(Grant::Toy(vendor.item))
        }
        8 => read_set_grant(conn, vendor.item, row),
        category => Err(format!(
            "Perks category{category} collection binding unavailable"
        )),
    }
}

fn read_set_grant(
    conn: &Connection,
    item: u32,
    row: &mut TradingPostItem,
) -> Result<Grant, String> {
    // Local ensemble ItemEffect -> SpellEffect255 -> exact embedded TransmogSet members.
    const LEARN_TRANSMOG_SET: u32 = 255;
    let set=conn.query_row("SELECT s.EffectMiscValue_0 FROM item_x_item_effect x JOIN item_effect e ON e.ID=x.ItemEffectID JOIN spell_effect s ON s.SpellID=e.SpellID WHERE x.ItemID=?1 AND s.DifficultyID=0 AND s.Effect=?2 ORDER BY e.ID LIMIT 1",rusqlite::params![item,LEARN_TRANSMOG_SET],|row|row.get::<_,u32>(0)).map_err(|error|error.to_string())?;
    let sources = crate::transmog::authored_set_sources(set)?;
    let mut query = conn
        .prepare("SELECT ItemID,ItemAppearanceID FROM item_modified_appearance WHERE ID=?1")
        .map_err(|error| error.to_string())?;
    let members: Vec<(u32, u32)> = sources
        .into_iter()
        .map(|id| {
            query
                .query_row([id], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(|error| format!("Set{set} source{id}: {error}"))
        })
        .collect::<Result<_, _>>()?;
    row.preview_item_ids = members.iter().map(|(item, _)| *item).collect();
    Ok(Grant::Appearances(
        members
            .into_iter()
            .map(|(_, appearance)| appearance)
            .collect(),
    ))
}
