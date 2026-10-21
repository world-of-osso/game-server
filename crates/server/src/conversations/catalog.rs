//! TC a352b1fa ConversationDataStore.cpp71–278 and Conversation.cpp187–234.
use std::collections::{HashMap, HashSet};

use super::*;
use rusqlite::Connection;

#[derive(Clone)]
pub(super) struct Definition {
    pub texture_kit: u32,
    pub flags: u8,
    pub actors: Vec<ActorPayload>,
    pub lines: Vec<LinePayload>,
    pub last_end: i64,
    pub maximum_end: i64,
}

#[derive(Resource, Clone)]
pub(crate) struct Catalog(pub(super) HashMap<u32, Result<Definition, String>>);

pub(crate) fn load_catalog(connection: &Connection) -> rusqlite::Result<Catalog> {
    let mut query = connection.prepare("SELECT Id,FirstLineId,TextureKitId,Flags,ScriptName FROM tdb_conversation_template ORDER BY Id")?;
    let rows = query.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            row.get::<_, u32>(2)?,
            row.get::<_, u8>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;
    let mut definitions = HashMap::new();
    for row in rows {
        let (entry, first, texture_kit, flags, script) = row?;
        let definition = if script.is_empty() {
            load_definition(connection, entry, first, texture_kit, flags)
        } else {
            Err(format!(
                "conversation{entry}: unsupported source script {script}"
            ))
        };
        definitions.insert(entry, definition);
    }
    Ok(Catalog(definitions))
}

fn load_definition(
    db: &Connection,
    entry: u32,
    first: u32,
    texture_kit: u32,
    flags: u8,
) -> Result<Definition, String> {
    let actors =
        load_actors(db, entry).map_err(|error| format!("conversation{entry} actors: {error}"))?;
    let mut lines = Vec::new();
    let mut seen = HashSet::new();
    let mut current = first;
    // Common.h71–89/WowLocaleToCascLocaleBit: represented locales, excluding none.
    const LOCALES: &[u8] = &[1, 2, 4, 5, 6, 8, 7, 12, 13, 14, 15];
    let mut ends = HashMap::<u8, i64>::new();
    while current != 0 {
        if !seen.insert(current) {
            return Err(format!("conversation{entry}: cyclic line{current}"));
        }
        let (mut line, next) = load_line(db, current)
            .map_err(|error| format!("conversation{entry} line{current}: {error}"))?;
        line.start_ms = *ends.get(&1).unwrap_or(&0);
        for &locale in LOCALES {
            let duration=db.query_row("SELECT DurationMS FROM broadcast_text_duration WHERE BroadcastTextID=?1 AND Locale=?2 LIMIT 1",rusqlite::params![line.broadcast_text_id,locale],|row|row.get::<_,i32>(0)).optional().map_err(|error|error.to_string())?;
            if locale == 1 {
                line.source_duration_ms = duration;
            }
            *ends.entry(locale).or_default() +=
                i64::from(duration.unwrap_or(0)) + i64::from(line.additional_duration_ms);
        }
        lines.push(line);
        current = next;
    }
    if lines.is_empty() {
        return Err(format!("conversation{entry}: no readable native lines"));
    }
    Ok(Definition {
        texture_kit,
        flags,
        actors,
        lines,
        last_end: *ends.get(&1).unwrap_or(&0),
        maximum_end: ends.values().copied().max().unwrap_or(0),
    })
}

use rusqlite::OptionalExtension;
fn load_actors(db: &Connection, entry: u32) -> rusqlite::Result<Vec<ActorPayload>> {
    let mut query=db.prepare("SELECT ConversationActorId,ConversationActorGuid,Idx,CreatureId,CreatureDisplayInfoId,NoActorObject,ActivePlayerObject FROM tdb_conversation_actors WHERE ConversationId=?1 ORDER BY Idx")?;
    query
        .query_map([entry], |row| {
            let guid: u64 = row.get(1)?;
            let creature_id: u32 = row.get(3)?;
            let display_id: u32 = row.get(4)?;
            let no_object: bool = row.get(5)?;
            let active_player: bool = row.get(6)?;
            let object = if active_player {
                ActorObject::ActivePlayer
            } else if no_object {
                ActorObject::NoObject {
                    creature_id,
                    display_id,
                }
            } else if guid != 0 || creature_id == 0 {
                ActorObject::WorldObject(guid)
            } else {
                ActorObject::TalkingHead {
                    creature_id,
                    display_id,
                }
            };
            let name = db
                .query_row(
                    "SELECT name FROM content_creature_template WHERE entry=?1",
                    [creature_id],
                    |row| row.get(0),
                )
                .optional()?;
            Ok(ActorPayload {
                id: row.get(0)?,
                index: row.get(2)?,
                name,
                object,
            })
        })?
        .collect()
}

fn load_line(db: &Connection, id: u32) -> rusqlite::Result<(LinePayload, u32)> {
    let conditions: u32 = db.query_row(
        "SELECT count(*) FROM tdb_conditions WHERE SourceTypeOrReferenceId=29 AND SourceEntry=?1",
        [id],
        |row| row.get(0),
    )?;
    if conditions != 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    db.query_row("SELECT c.BroadcastTextID,t.UiCameraID,t.ActorIdx,t.Flags,t.ChatType,c.AdditionalDuration,c.NextConversationLineID,c.SpellVisualKitID,c.AnimKitID,c.StartAnimation,c.EndAnimation FROM conversation_line c JOIN tdb_conversation_line_template t ON t.Id=c.ID WHERE c.ID=?1",[id],|row| {
        let broadcast:u32=row.get(0)?;
        let text=db.query_row("SELECT MaleText,FemaleText,SoundEntriesId,LanguageID FROM content_broadcast_text WHERE ID=?1",[broadcast],|text| Ok(BroadcastTextPayload {male_text:text.get(0)?,female_text:text.get(1)?,sound_kit_id:text.get(2)?,language_id:text.get(3)?})).optional()?;
        Ok((LinePayload {id,broadcast_text_id:broadcast,ui_camera_id:row.get(1)?,actor_index:row.get(2)?,flags:row.get(3)?,chat_type:row.get(4)?,start_ms:0,source_duration_ms:None,additional_duration_ms:row.get(5)?,visual_kit_id:row.get(7)?,animation_kit_id:row.get(8)?,start_animation:row.get(9)?,end_animation:row.get(10)?,text},row.get(6)?))
    })
}
