use redb::{Database, ReadableDatabase, ReadableTable};
use rusqlite::{Connection, OpenFlags, params};
use shared::components::{CharacterAppearance, GuildMembership};
use std::collections::BTreeSet;

const MAX_CHARACTERS_PER_ACCOUNT: usize = 10;

use crate::persistence::{
    ACCOUNT_CHARACTERS, CHARACTER_GUILDS, CHARACTER_NAMES, CHARACTERS, GUILD_MEMBER_NOTES,
    GUILD_PROFILES, PENDING_GUILD_INVITES,
};
use crate::persistence::{next_id, save_inventory};
use crate::persistence_types::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildMemberRecord {
    pub character_id: u64,
    pub name: String,
    pub level: u16,
    pub class_id: u8,
    pub guild_name: String,
}

/// Save a single character to the database.
pub fn save_character(db: &Database, character_id: u64, data: &CharacterData) {
    let bytes = bitcode::encode(data);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(CHARACTERS)
            .expect("Failed to open characters table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert character");
    }
    txn.commit().expect("Failed to commit character save");
}

/// Load a character from the database, if one exists for this character_id.
pub fn load_character(db: &Database, character_id: u64) -> Option<CharacterData> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(CHARACTERS)
        .expect("Failed to open characters table");
    table
        .get(character_id)
        .expect("Failed to read character")
        .map(|guard| {
            CharacterData::decode_compatible(guard.value()).expect("Failed to decode character")
        })
}

pub fn load_character_guild(db: &Database, character_id: u64) -> Option<GuildMembership> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(CHARACTER_GUILDS)
        .expect("Failed to open character_guilds table");
    let guard = table
        .get(character_id)
        .expect("Failed to read character guild")?;
    let data = bitcode::decode::<GuildMembershipData>(guard.value()).expect("decode guild");
    Some(GuildMembership {
        guild_id: data.guild_id,
        guild_name: data.guild_name,
    })
}

pub fn set_character_guild_by_name(
    db: &Database,
    character_name: &str,
    guild_id: u32,
    guild_name: &str,
) -> Result<u64, String> {
    let character_id = find_character_id_by_name(db, character_name)
        .ok_or_else(|| format!("Character '{character_name}' not found"))?;
    set_character_guild_by_id(db, character_id, guild_id, guild_name)?;
    Ok(character_id)
}

pub fn set_character_guild_by_id(
    db: &Database,
    character_id: u64,
    guild_id: u32,
    guild_name: &str,
) -> Result<(), String> {
    let guild = GuildMembershipData {
        guild_id,
        guild_name: guild_name.to_string(),
    };
    let encoded = bitcode::encode(&guild);
    let write = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = write
            .open_table(CHARACTER_GUILDS)
            .expect("open character_guilds");
        table
            .insert(character_id, encoded.as_slice())
            .expect("insert character guild");
    }
    write.commit().expect("Failed to commit character guild");
    Ok(())
}

pub fn set_pending_guild_invite(
    db: &Database,
    character_id: u64,
    guild_id: u32,
    guild_name: &str,
    inviter_name: &str,
) {
    let invite = PendingGuildInviteData {
        guild_id,
        guild_name: guild_name.to_string(),
        inviter_name: inviter_name.to_string(),
    };
    let encoded = bitcode::encode(&invite);
    let write = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = write
            .open_table(PENDING_GUILD_INVITES)
            .expect("open pending_guild_invites");
        table
            .insert(character_id, encoded.as_slice())
            .expect("insert pending guild invite");
    }
    write
        .commit()
        .expect("Failed to commit pending guild invite");
}

pub fn take_pending_guild_invite(
    db: &Database,
    character_id: u64,
) -> Option<PendingGuildInviteData> {
    let write = db.begin_write().expect("Failed to begin write txn");
    let invite = {
        let mut table = write
            .open_table(PENDING_GUILD_INVITES)
            .expect("open pending_guild_invites");
        let raw = table
            .get(character_id)
            .expect("read pending guild invite")
            .map(|g| g.value().to_vec())?;
        let decoded = bitcode::decode::<PendingGuildInviteData>(&raw).expect("decode invite");
        table
            .remove(character_id)
            .expect("remove pending guild invite");
        decoded
    };
    write
        .commit()
        .expect("Failed to commit pending guild invite removal");
    Some(invite)
}

pub fn load_guild_profile(db: &Database, guild_id: u32) -> GuildProfileData {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let Ok(table) = txn.open_table(GUILD_PROFILES) else {
        return GuildProfileData::default();
    };
    table
        .get(guild_id)
        .expect("Failed to read guild profile")
        .map(|guard| {
            bitcode::decode::<GuildProfileData>(guard.value()).expect("decode guild profile")
        })
        .unwrap_or_default()
}

pub fn save_guild_profile(db: &Database, guild_id: u32, profile: &GuildProfileData) {
    let encoded = bitcode::encode(profile);
    let write = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = write
            .open_table(GUILD_PROFILES)
            .expect("open guild_profiles");
        table
            .insert(guild_id, encoded.as_slice())
            .expect("insert guild profile");
    }
    write.commit().expect("Failed to commit guild profile");
}

pub fn load_guild_member_note(db: &Database, character_id: u64) -> GuildMemberNoteData {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let Ok(table) = txn.open_table(GUILD_MEMBER_NOTES) else {
        return GuildMemberNoteData::default();
    };
    table
        .get(character_id)
        .expect("Failed to read guild member note")
        .map(|guard| {
            bitcode::decode::<GuildMemberNoteData>(guard.value()).expect("decode guild member note")
        })
        .unwrap_or_default()
}

pub fn save_guild_member_note(db: &Database, character_id: u64, note: &GuildMemberNoteData) {
    let encoded = bitcode::encode(note);
    let write = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = write
            .open_table(GUILD_MEMBER_NOTES)
            .expect("open guild_member_notes");
        table
            .insert(character_id, encoded.as_slice())
            .expect("insert guild member note");
    }
    write.commit().expect("Failed to commit guild member note");
}

pub fn list_guild_members(db: &Database, guild_id: u32) -> Vec<GuildMemberRecord> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let guilds = txn
        .open_table(CHARACTER_GUILDS)
        .expect("Failed to open character_guilds table");
    let characters = txn
        .open_table(CHARACTERS)
        .expect("Failed to open characters table");
    let mut members = Vec::new();
    for row in guilds.iter().expect("Failed to iterate character_guilds") {
        let (character_id, encoded_guild) = row.expect("Failed to read character_guilds row");
        let guild =
            bitcode::decode::<GuildMembershipData>(encoded_guild.value()).expect("decode guild");
        if guild.guild_id != guild_id {
            continue;
        }
        let Some(encoded_character) = characters
            .get(character_id.value())
            .expect("Failed to read character")
        else {
            continue;
        };
        let character = CharacterData::decode_compatible(encoded_character.value())
            .expect("Failed to decode character");
        members.push(GuildMemberRecord {
            character_id: character_id.value(),
            name: character.name,
            level: character.level,
            class_id: character.class,
            guild_name: guild.guild_name,
        });
    }
    members.sort_by(|left, right| left.name.cmp(&right.name));
    members
}

pub fn find_character_id_by_name(db: &Database, character_name: &str) -> Option<u64> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let names = txn
        .open_table(CHARACTER_NAMES)
        .expect("Failed to open character_names");
    names
        .get(character_name)
        .expect("Failed to read character name")
        .map(|v| v.value())
}

/// List all characters for an account.
pub fn list_characters_for_account(db: &Database, account_id: u64) -> Vec<(u64, CharacterData)> {
    let char_ids = load_account_char_ids(db, account_id);
    let mut result = Vec::new();
    for id in char_ids {
        if let Some(data) = load_character(db, id) {
            result.push((id, data));
        }
    }
    result
}

pub fn find_account_id_for_character(db: &Database, character_id: u64) -> Option<u64> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(ACCOUNT_CHARACTERS)
        .expect("Failed to open account_characters");
    for row in table.iter().expect("Failed to iterate account_characters") {
        let (account_id, encoded_ids) = row.expect("Failed to read account_characters row");
        let ids =
            bitcode::decode::<Vec<u64>>(encoded_ids.value()).expect("decode account character ids");
        if ids.contains(&character_id) {
            return Some(account_id.value());
        }
    }
    None
}

/// Move all characters from one account to another. Returns the number of moved characters.
pub fn move_characters_between_accounts(
    db: &Database,
    from_account_id: u64,
    to_account_id: u64,
) -> Result<usize, String> {
    if from_account_id == to_account_id {
        return Ok(0);
    }

    let from_ids = load_account_char_ids(db, from_account_id);
    if from_ids.is_empty() {
        return Ok(0);
    }
    let mut to_ids = load_account_char_ids(db, to_account_id);
    // Keep deterministic ordering while avoiding repeated linear contains checks.
    let mut seen: BTreeSet<u64> = to_ids.iter().copied().collect();
    for id in from_ids.iter().copied() {
        if seen.insert(id) {
            to_ids.push(id);
        }
    }
    to_ids.sort_unstable();

    let from_encoded = bitcode::encode(&Vec::<u64>::new());
    let to_encoded = bitcode::encode(&to_ids);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut acct = txn.open_table(ACCOUNT_CHARACTERS).expect("open acct chars");
        acct.insert(from_account_id, from_encoded.as_slice())
            .expect("clear source acct chars");
        acct.insert(to_account_id, to_encoded.as_slice())
            .expect("update target acct chars");
    }
    txn.commit()
        .map_err(|e| format!("Failed to commit character move: {e}"))?;
    Ok(from_ids.len())
}

/// Load the character ID list for an account.
pub(crate) fn load_account_char_ids(db: &Database, account_id: u64) -> Vec<u64> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(ACCOUNT_CHARACTERS)
        .expect("Failed to open account_characters");
    match table.get(account_id).expect("Failed to read account chars") {
        Some(guard) => {
            bitcode::decode::<Vec<u64>>(guard.value()).expect("Failed to decode char ids")
        }
        None => Vec::new(),
    }
}

/// Create a new character for an account. Returns the character or an error.
#[cfg(test)]
pub fn create_character(
    db: &Database,
    account_id: u64,
    name: &str,
    race: u8,
    class: u8,
) -> Result<(u64, CharacterData), String> {
    create_character_with_appearance(
        db,
        account_id,
        name,
        race,
        class,
        CharacterAppearance::default(),
    )
}

pub fn create_character_with_appearance(
    db: &Database,
    account_id: u64,
    name: &str,
    race: u8,
    class: u8,
    appearance: CharacterAppearance,
) -> Result<(u64, CharacterData), String> {
    let trimmed = name.trim();
    if trimmed.len() < 2 {
        return Err("Character name must be at least 2 characters".to_string());
    }
    if trimmed.len() > 12 {
        return Err("Character name must be at most 12 characters".to_string());
    }
    if !trimmed.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err("Character name must contain only letters".to_string());
    }
    if list_characters_for_account(db, account_id).len() >= MAX_CHARACTERS_PER_ACCOUNT {
        return Err("Maximum characters per account reached".to_string());
    }
    check_name_available(db, name)?;
    validate_playable_race(race)?;
    let character_id = next_id(db, "character");
    let data = default_character_data(name, race, class, appearance);
    save_new_character(db, account_id, character_id, name, &data);
    save_inventory(db, character_id, &default_inventory(db));
    Ok((character_id, data))
}

pub(crate) fn validate_playable_race(race: u8) -> Result<(), String> {
    if let Ok(conn) = Connection::open_with_flags("data/world.db", OpenFlags::SQLITE_OPEN_READ_ONLY)
        && let Ok(mut stmt) = conn.prepare("SELECT 1 FROM chr_races WHERE id = ?1 LIMIT 1")
    {
        let mut rows = stmt
            .query(params![i64::from(race)])
            .map_err(|e| format!("Failed to execute race query: {e}"))?;
        return if rows
            .next()
            .map_err(|e| format!("Failed to read race query result: {e}"))?
            .is_some()
        {
            Ok(())
        } else {
            Err(format!(
                "Invalid race id {race}. See data/world.db table chr_races"
            ))
        };
    }

    const FALLBACK_PLAYABLE_RACES: &[u8] = &[
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 22, 24, 25, 26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37,
        52, 70, 84, 85,
    ];
    if FALLBACK_PLAYABLE_RACES.contains(&race) {
        Ok(())
    } else {
        Err(format!("Invalid race id {race}"))
    }
}

/// Check that a character name is not already taken.
fn check_name_available(db: &Database, name: &str) -> Result<(), String> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(CHARACTER_NAMES)
        .expect("Failed to open character_names");
    if table.get(name).expect("Failed to read name").is_some() {
        return Err(format!("Name '{name}' is already taken"));
    }
    Ok(())
}

/// Build default CharacterData for a new character at Northshire spawn.
/// Starting position per race (WoW coordinates).
fn starting_position(race: u8) -> (f32, f32, f32) {
    match race {
        1 => (-8949.0, -132.0, 83.0),     // Human — Northshire
        2 => (-618.0, -4251.0, 38.0),     // Orc — Valley of Trials
        3 => (-6240.0, 331.0, 383.0),     // Dwarf — Coldridge Valley
        4 => (10311.0, 832.0, 1326.0),    // Night Elf — Shadowglen
        5 => (1676.0, 1677.0, 121.0),     // Undead — Deathknell
        6 => (-2917.0, -257.0, 53.0),     // Tauren — Red Cloud Mesa
        7 => (-6240.0, 331.0, 383.0),     // Gnome — Coldridge Valley (shared)
        8 => (-618.0, -4251.0, 38.0),     // Troll — Valley of Trials (shared)
        10 => (10349.0, -6357.0, 33.0),   // Blood Elf — Sunstrider Isle
        11 => (-3961.0, -13931.0, 100.0), // Draenei — Crash Site
        22 => (-1451.0, 1403.0, 35.0),    // Worgen — Gilneas
        _ => (-8949.0, -132.0, 83.0),     // Fallback to Human start
    }
}

fn default_character_data(
    name: &str,
    race: u8,
    class: u8,
    appearance: CharacterAppearance,
) -> CharacterData {
    let (x, y, z) = starting_position(race);
    CharacterData {
        name: name.to_string(),
        x,
        y,
        z,
        health: 100.0,
        max_health: 100.0,
        race,
        class,
        level: 1,
        appearance,
        equipment_appearance: Default::default(),
        durability: Vec::new(),
    }
}

/// Persist a new character across all tables in a single transaction.
pub(crate) fn save_new_character(
    db: &Database,
    account_id: u64,
    character_id: u64,
    name: &str,
    data: &CharacterData,
) {
    let bytes = bitcode::encode(data);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut chars = txn.open_table(CHARACTERS).expect("open characters");
        chars
            .insert(character_id, bytes.as_slice())
            .expect("insert character");
        let mut names = txn.open_table(CHARACTER_NAMES).expect("open names");
        names
            .insert(name, character_id)
            .expect("insert character name");
        let mut acct = txn.open_table(ACCOUNT_CHARACTERS).expect("open acct chars");
        let mut ids = read_char_ids_from_table(&acct, account_id);
        ids.push(character_id);
        let encoded = bitcode::encode(&ids);
        acct.insert(account_id, encoded.as_slice())
            .expect("insert acct chars");
    }
    txn.commit().expect("Failed to commit new character");
}

fn starter_item(
    db: &Database,
    item_id: u32,
    name: &str,
    count: u32,
    price: u32,
) -> InventoryItemData {
    InventoryItemData {
        item_guid: next_id(db, "item_instance"),
        item_id,
        name: name.to_string(),
        quality: 1,
        required_level: 1,
        stack_count: count,
        vendor_sell_price: price,
        unique_auction: false,
    }
}

pub(crate) fn default_inventory(db: &Database) -> InventoryData {
    InventoryData {
        gold: 100_000,
        items: vec![
            starter_item(db, 2589, "Linen Cloth", 20, 13),
            starter_item(db, 2447, "Peacebloom", 10, 10),
            starter_item(db, 2770, "Copper Ore", 10, 25),
        ],
    }
}

/// Read character IDs from an open account_characters table.
pub(crate) fn read_char_ids_from_table(
    table: &redb::Table<u64, &[u8]>,
    account_id: u64,
) -> Vec<u64> {
    match table.get(account_id).expect("read acct chars") {
        Some(guard) => bitcode::decode::<Vec<u64>>(guard.value()).expect("decode char ids"),
        None => Vec::new(),
    }
}

/// Delete a character, validating account ownership first.
pub fn delete_character(db: &Database, account_id: u64, character_id: u64) -> Result<(), String> {
    let char_ids = load_account_char_ids(db, account_id);
    if !char_ids.contains(&character_id) {
        return Err("Character does not belong to this account".into());
    }
    let name = load_character(db, character_id)
        .map(|d| d.name)
        .ok_or("Character not found")?;
    remove_character(db, account_id, character_id, &name, &char_ids);
    Ok(())
}

/// Remove a character from all tables in a single transaction.
pub(crate) fn remove_character(
    db: &Database,
    account_id: u64,
    character_id: u64,
    name: &str,
    char_ids: &[u64],
) {
    let remaining: Vec<u64> = char_ids
        .iter()
        .copied()
        .filter(|&id| id != character_id)
        .collect();
    let encoded = bitcode::encode(&remaining);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut chars = txn.open_table(CHARACTERS).expect("open characters");
        chars.remove(character_id).expect("remove character");
        let mut names = txn.open_table(CHARACTER_NAMES).expect("open names");
        names.remove(name).expect("remove character name");
        let mut acct = txn.open_table(ACCOUNT_CHARACTERS).expect("open acct chars");
        acct.insert(account_id, encoded.as_slice())
            .expect("update acct chars");
    }
    txn.commit().expect("Failed to commit character deletion");
}
