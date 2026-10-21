//! TrinityCore `CreatureTextMgr` (game/Texts/CreatureTextMgr.cpp): what creatures
//! say, yell and emote, by `content_creature_text` group.
//!
//! `talk` is `SendChat(source, textGroup)`: one text of the group not said since
//! the group was last exhausted (repeat ids), weighted by `Probability`; the text
//! is its `BroadcastText` (female text for a female or genderless speaker when it
//! has one, `DB2Manager::GetBroadcastTextValue`), else `creature_text.Text`. The
//! chat type is `Type` (12 say, 14 yell, 16 emote, 41 boss emote) and it reaches
//! `TextRange` 3 (map: every player of the speaker's map copy) or 0 (the chat
//! type's listen range: say and emotes 25 yards, yell 300). Normal-play sounds share
//! that audience; gender-specific BroadcastText SoundKit overrides the text's Sound.
//! Emotes and area, zone, world and personal ranges are not used by the ported scripts.

use crate::random as rand;
use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::components::{ModelDisplay, Npc, Player, Position};
use shared::protocol::{ChatChannel, ChatMessage, ChatType, CreatureTextSound};

use crate::maps::InMap;
use crate::networking::PlayerLink;

/// `CHAT_MSG_MONSTER_SAY` .. `CHAT_MSG_RAID_BOSS_EMOTE` (`ChatMsg`).
const CHAT_MSG_MONSTER_SAY: u8 = 12;
const CHAT_MSG_MONSTER_YELL: u8 = 14;
const CHAT_MSG_MONSTER_EMOTE: u8 = 16;
const CHAT_MSG_RAID_BOSS_EMOTE: u8 = 41;
/// `CreatureTextRange`.
const TEXT_RANGE_NORMAL: u8 = 0;
const TEXT_RANGE_MAP: u8 = 3;
/// `CONFIG_LISTEN_RANGE_SAY` / `_TEXTEMOTE` / `_YELL` defaults.
const LISTEN_RANGE_SAY: f32 = 25.0;
const LISTEN_RANGE_YELL: f32 = 300.0;
/// `GENDER_FEMALE` and `GENDER_NONE` (CreatureDisplayInfo.Gender; -1 is unknown).
const GENDER_FEMALE: i8 = 1;
const GENDER_NONE: i8 = 2;

/// One `creature_text` row with its texts resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatureTextEntry {
    pub id: u8,
    pub chat_type: u8,
    pub text_range: u8,
    pub probability: f32,
    pub male_text: String,
    /// BroadcastText `Text1`; empty when the text has no female form.
    pub female_text: String,
    pub sound: u32,
    pub sound_play_type: u8,
    pub broadcast_sounds: [u32; 2],
}

#[derive(Resource, Debug, Default)]
pub struct CreatureTexts {
    /// By creature entry, then group.
    pub groups: HashMap<u32, HashMap<u8, Vec<CreatureTextEntry>>>,
    /// `CreatureDisplayInfo.Gender` by display id (`Creature::InitEntry` → `SetGender`).
    pub display_gender: HashMap<u32, i8>,
}

/// Text ids of each group said since the group was last exhausted
/// (`Creature::GetTextRepeatGroup`).
#[derive(Component, Debug, Default)]
pub struct TextRepeatGroups(HashMap<u8, Vec<u8>>);

pub fn load_creature_texts(conn: &Connection) -> rusqlite::Result<CreatureTexts> {
    let mut texts = CreatureTexts::default();
    let mut statement = conn.prepare(
        "SELECT t.CreatureID, t.GroupID, t.ID, t.Type, t.TextRange, t.Probability, t.Text,
                COALESCE(b.MaleText, ''), COALESCE(b.FemaleText, ''), b.ID IS NOT NULL,
                t.Sound, t.SoundPlayType, COALESCE(r.SoundKitID1, 0), COALESCE(r.SoundKitID2, 0)
         FROM content_creature_text t
         LEFT JOIN content_broadcast_text b ON b.ID = t.BroadcastTextId AND t.BroadcastTextId <> 0
         LEFT JOIN tdb_broadcast_text r ON r.ID = t.BroadcastTextId
           AND r.VerifiedBuild = (SELECT MAX(latest.VerifiedBuild) FROM tdb_broadcast_text latest WHERE latest.ID = r.ID)
         ORDER BY t.CreatureID, t.GroupID, t.ID",
    )?;
    let rows = statement.query_map([], |row| {
        let broadcast: bool = row.get(9)?;
        let own_text: String = row.get(6)?;
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u8>(1)?,
            CreatureTextEntry {
                id: row.get(2)?,
                chat_type: row.get(3)?,
                text_range: row.get(4)?,
                probability: row.get::<_, f64>(5)? as f32,
                sound: row.get(10)?,
                sound_play_type: row.get(11)?,
                broadcast_sounds: [row.get(12)?, row.get(13)?],
                male_text: if broadcast { row.get(7)? } else { own_text },
                female_text: if broadcast {
                    row.get(8)?
                } else {
                    String::new()
                },
            },
        ))
    })?;
    for row in rows {
        let (entry, group, text) = row?;
        texts
            .groups
            .entry(entry)
            .or_default()
            .entry(group)
            .or_default()
            .push(text);
    }
    let mut statement = conn.prepare("SELECT ID, Gender FROM creature_display_info")?;
    texts.display_gender = statement
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, i8>(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(texts)
}

pub(super) fn load_creature_texts_startup(mut commands: Commands) {
    let path = Path::new(crate::spell_info::WORLD_DB_PATH);
    if !path.exists() {
        warn!("world.db not found, creature texts empty");
        commands.insert_resource(CreatureTexts::default());
        return;
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Failed to open world.db for creature texts");
    let texts = load_creature_texts(&conn)
        .expect("Failed to load content_creature_text; run scripts/import-creature-text.sh");
    info!("Loaded creature texts for {} creatures", texts.groups.len());
    commands.insert_resource(texts);
}

/// `CreatureTextMgr::SendChat(source, textGroup)`.
pub fn talk(world: &mut World, speaker: Entity, group: u8) {
    talk_with_target(world, speaker, group, None);
}

/// The target supplies `$n`; Say still reaches the speaker's nearby listeners.
pub fn talk_with_target(world: &mut World, speaker: Entity, group: u8, target: Option<Entity>) {
    let Some(npc) = world.get::<Npc>(speaker).cloned() else {
        return;
    };
    let Some(candidates) = world
        .get_resource::<CreatureTexts>()
        .and_then(|texts| texts.groups.get(&npc.template_id))
        .and_then(|groups| groups.get(&group))
        .cloned()
    else {
        error!(
            "CreatureTextMgr: Could not find TextGroup {group} for Creature {} ({})",
            npc.name, npc.template_id
        );
        return;
    };
    let Some(text) = choose_text(world, speaker, group, &candidates) else {
        return;
    };
    let female = matches!(
        speaker_gender(world, speaker),
        Some(GENDER_FEMALE | GENDER_NONE)
    );
    let content = if female && !text.female_text.is_empty() {
        text.female_text.clone()
    } else {
        text.male_text.clone()
    };
    let content = if let Some(target) = target {
        let name = world
            .get::<Player>(target)
            .map(|player| player.name.as_str())
            .or_else(|| world.get::<Npc>(target).map(|npc| npc.name.as_str()));
        let Some(name) = name else {
            error!("CreatureTextMgr: speech target {target:?} has no unit name");
            return;
        };
        content.replace("$n", name)
    } else {
        content
    };
    let speaker_bits = speaker.to_bits();
    let (channel, listen_range) = match text.chat_type {
        CHAT_MSG_MONSTER_SAY => (ChatType::MonsterSay(speaker_bits), LISTEN_RANGE_SAY),
        CHAT_MSG_MONSTER_YELL => (ChatType::MonsterYell(speaker_bits), LISTEN_RANGE_YELL),
        CHAT_MSG_MONSTER_EMOTE => (ChatType::MonsterEmote(speaker_bits), LISTEN_RANGE_SAY),
        CHAT_MSG_RAID_BOSS_EMOTE => (ChatType::RaidBossEmote(speaker_bits), LISTEN_RANGE_SAY),
        other => {
            warn!(
                "creature_text Type {other} of {} is not a ported chat type",
                npc.template_id
            );
            return;
        }
    };
    let range = match text.text_range {
        TEXT_RANGE_MAP => None,
        TEXT_RANGE_NORMAL => Some(listen_range),
        other => {
            warn!(
                "creature_text TextRange {other} of {} is not ported",
                npc.template_id
            );
            return;
        }
    };
    let message = ChatMessage {
        sender: npc.name,
        content,
        channel,
    };
    info!(
        "[{:?}] {}: {}",
        message.channel, message.sender, message.content
    );
    let gender = speaker_gender(world, speaker);
    let broadcast_sound = text.broadcast_sounds[usize::from(gender == Some(GENDER_FEMALE))];
    let sound_kit = if broadcast_sound != 0 {
        broadcast_sound
    } else {
        text.sound
    };
    for player in hearers(world, speaker, range) {
        send_chat(world, player, message.clone());
        // Native voice coverage is currently Naminze only; other creature sounds
        // remain outside this arena change instead of emitting unsupported kits.
        if npc.template_id == crate::pvp::arena_cage::NAMINZE && sound_kit != 0 {
            if text.sound_play_type == 0 {
                send_sound(
                    world,
                    player,
                    CreatureTextSound {
                        speaker: speaker_bits,
                        sound_kit,
                    },
                );
            } else {
                warn!(
                    "creature_text SoundPlayType {} of {} is not ported",
                    text.sound_play_type, speaker_bits
                );
            }
        }
    }
}

fn send_sound(world: &mut World, player: Entity, message: CreatureTextSound) {
    let mut connections = world.query::<(&PlayerLink, &mut MessageSender<CreatureTextSound>)>();
    for (link, mut sender) in connections.iter_mut(world) {
        if link.0 == player {
            sender.send::<ChatChannel>(message.clone());
        }
    }
}

/// A text of the group not said since the group was exhausted, by probability
/// (`SelectRandomWeightedContainerElement`); records it as said.
fn choose_text(
    world: &mut World,
    speaker: Entity,
    group: u8,
    candidates: &[CreatureTextEntry],
) -> Option<CreatureTextEntry> {
    let mut entity = world.entity_mut(speaker);
    let mut repeats = entity.entry::<TextRepeatGroups>().or_default();
    let mut repeats = repeats.get_mut();
    let said = repeats.0.entry(group).or_default();
    let mut unsaid: Vec<&CreatureTextEntry> = candidates
        .iter()
        .filter(|text| !said.contains(&text.id))
        .collect();
    if unsaid.is_empty() {
        said.clear();
        unsaid = candidates.iter().collect();
    }
    let index = match rand::distributions::WeightedIndex::new(
        unsaid
            .iter()
            .map(|text| f64::from(text.probability.max(0.0))),
    ) {
        Ok(weights) => rand::Rng::sample(&mut rand::thread_rng(), weights),
        Err(_) => rand::Rng::gen_range(&mut rand::thread_rng(), 0..unsaid.len()),
    };
    let chosen = unsaid.get(index).map(|text| (*text).clone())?;
    said.push(chosen.id);
    Some(chosen)
}

fn speaker_gender(world: &World, speaker: Entity) -> Option<i8> {
    let display = world.get::<ModelDisplay>(speaker)?;
    world
        .get_resource::<CreatureTexts>()?
        .display_gender
        .get(&display.display_id)
        .copied()
}

/// Players of the speaker's map copy, within `range` yards when given.
fn hearers(world: &mut World, speaker: Entity, range: Option<f32>) -> Vec<Entity> {
    let Some(map) = world.get::<InMap>(speaker).copied() else {
        return Vec::new();
    };
    let origin = world.get::<Position>(speaker).copied();
    world
        .query_filtered::<(Entity, &InMap, &Position), With<Player>>()
        .iter(world)
        .filter(|(_, player_map, position)| {
            **player_map == map
                && range.is_none_or(|range| {
                    origin.is_some_and(|origin| {
                        let (dx, dy, dz) = (
                            origin.x - position.x,
                            origin.y - position.y,
                            origin.z - position.z,
                        );
                        dx * dx + dy * dy + dz * dz <= range * range
                    })
                })
        })
        .map(|(player, ..)| player)
        .collect()
}

pub(crate) fn send_chat(world: &mut World, player: Entity, message: ChatMessage) {
    let mut connections = world.query::<(&PlayerLink, &mut MessageSender<ChatMessage>)>();
    for (link, mut sender) in connections.iter_mut(world) {
        if link.0 == player {
            sender.send::<ChatChannel>(message.clone());
        }
    }
}
