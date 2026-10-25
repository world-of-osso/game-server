use super::{Bundle, Result};
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};
use std::path::Path;

pub fn label(db: &Connection, table: &str, column: &str, id: u32) -> Result<Value> {
    let name: Option<String> = db
        .query_row(
            &format!("SELECT {column} FROM {table} WHERE ID=?1"),
            [id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(json!({"id":id,"name":name}))
}

pub fn race(db: &Connection, id: u8) -> Result<(Value, Value)> {
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='chr_races')",
        [],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok((json!({"id":id,"name":null}), Value::Null));
    }
    let row: Option<(String, u32)> = db
        .query_row(
            "SELECT Name_lang,FactionID FROM chr_races WHERE ID=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((name, template)) = row else {
        return Ok((json!({"id":id,"name":null}), Value::Null));
    };
    let group: Option<u32> = db
        .query_row(
            "SELECT FactionGroup FROM faction_template WHERE ID=?1",
            [template],
            |r| r.get(0),
        )
        .optional()?;
    let faction = match group.map(|group| group & 6) {
        Some(2) => json!({"id":1,"name":"Alliance","template_id":template}),
        Some(4) => json!({"id":2,"name":"Horde","template_id":template}),
        Some(0) => json!({"id":0,"name":"Neutral","template_id":template}),
        _ => Value::Null,
    };
    Ok((json!({"id":id,"name":name}), faction))
}
pub fn item(db: &Connection, id: u32, source: u8) -> Result<(Option<u32>, Option<u32>)> {
    let prefix = match source {
        0 => "",
        1 => "skyborne_",
        _ => return Err(format!("unknown item definition source {source}").into()),
    };
    let level = db
        .query_row(
            &format!("SELECT ItemLevel FROM {prefix}item_sparse WHERE ID=?1"),
            [id],
            |r| r.get(0),
        )
        .optional()?;
    let icon = db
        .query_row(
            &format!("SELECT IconFileDataID FROM {prefix}item WHERE ID=?1"),
            [id],
            |r| r.get::<_, u32>(0),
        )
        .optional()?
        .filter(|id| *id != 0);
    Ok((level, icon))
}

pub fn icon(bundle: &mut Bundle, textures: &Path, fdid: Option<u32>) -> Result<Option<String>> {
    let Some(id) = fdid else {
        return Ok(None);
    };
    let path = format!("icons/{id}.png");
    if bundle.contains_key(&path) {
        return Ok(Some(path));
    }
    let source = textures.join(format!("{id}.png"));
    if source.is_file() {
        // Decode before copying: malformed local assets must not become broken web URLs.
        let image = image::open(&source)?;
        let mut png = std::io::Cursor::new(Vec::new());
        image.write_to(&mut png, image::ImageFormat::Png)?;
        bundle.insert(path.clone(), png.into_inner());
        return Ok(Some(path));
    }
    let source = textures.join(format!("{id}.blp"));
    if !source.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(source)?;
    let blp =
        image_blp::parser::load_blp_from_buf(&bytes).map_err(|e| format!("icon {id}: {e}"))?;
    let image = image_blp::convert::blp_to_image(&blp, 0).map_err(|e| format!("icon {id}: {e}"))?;
    let mut png = std::io::Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png)?;
    bundle.insert(path.clone(), png.into_inner());
    Ok(Some(path))
}

pub fn raid(db: &Connection, lock: &super::store::Lock) -> Result<Option<Value>> {
    let raid: Option<String> = db
        .query_row(
            "SELECT MapName_lang FROM map WHERE ID=?1 AND InstanceType=2",
            [lock.map_id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(name) = raid else {
        return Ok(None);
    };
    let mut query=db.prepare("SELECT ID,Name_lang,Bit FROM dungeon_encounter WHERE MapID=?1 AND (DifficultyID<=0 OR DifficultyID=?2) ORDER BY Bit,DifficultyID DESC,ID")?;
    let rows = query.query_map([lock.map_id, lock.difficulty_id], |r| {
        Ok((
            r.get::<_, u32>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, u32>(2)?,
        ))
    })?;
    let mut bits = std::collections::BTreeSet::new();
    let mut encounters = Vec::new();
    for row in rows {
        let (id, name, bit) = row?;
        if bit >= 32 {
            return Err(format!("encounter {id}: invalid mask bit {bit}").into());
        }
        if bits.insert(bit) {
            encounters.push(json!({"id":id,"name":name,"killed":lock.data.completed_encounters & (1<<bit)!=0,"first_kill_at":null}));
        }
    }
    if encounters.is_empty() {
        return Ok(None);
    }
    let kills = encounters.iter().filter(|e| e["killed"] == true).count();
    Ok(Some(
        json!({"raid":{"id":lock.map_id,"name":name},"difficulty_id":lock.difficulty_id,"scope":"saved_lock","kills":kills,"total":encounters.len(),"encounters":encounters,"first_kill_at":null,"lock_expires_at":lock.expiry,"extended":lock.extended}),
    ))
}
