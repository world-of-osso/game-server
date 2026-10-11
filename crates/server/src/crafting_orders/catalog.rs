use bevy::prelude::Resource;
use rusqlite::Connection;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct Recipe {
    pub spell_id: u32,
    pub skill_line: u32,
    pub item_id: u32,
    pub count: u32,
}

#[derive(Resource, Default)]
pub(crate) struct OrderCatalog(pub HashMap<u32, Recipe>);

pub(crate) fn load_catalog(conn: &Connection) -> Result<OrderCatalog, String> {
    if !conn
        .table_exists(None, "crafting_order")
        .map_err(|e| e.to_string())?
    {
        // Feature explicitly unavailable until the copy-only import is supplied.
        return Ok(OrderCatalog::default());
    }
    let mut query=conn.prepare("SELECT a.Spell,a.SkillLine,e.EffectItemType,MAX(1,CAST(ROUND(e.EffectBasePointsF) AS INTEGER)) FROM crafting_order o JOIN skill_line_ability a ON a.ID=o.SkillLineAbilityID JOIN spell_effect e ON e.SpellID=a.Spell WHERE e.Effect=24 AND e.DifficultyID=0 AND e.EffectItemType>0").map_err(|e|e.to_string())?;
    let rows = query
        .query_map([], |row| {
            Ok(Recipe {
                spell_id: row.get(0)?,
                skill_line: row.get(1)?,
                item_id: row.get(2)?,
                count: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut catalog = OrderCatalog::default();
    for row in rows {
        let recipe = row.map_err(|e| e.to_string())?;
        if catalog.0.insert(recipe.spell_id, recipe).is_some() {
            return Err("duplicate fixed-output crafting order spell".into());
        }
    }
    Ok(catalog)
}
