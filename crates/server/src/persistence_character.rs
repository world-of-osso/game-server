use redb::{Database, ReadableDatabase, ReadableTable};
use shared::components::{CharacterAppearance, GuildMembership};
use std::collections::BTreeSet;

const MAX_CHARACTERS_PER_ACCOUNT: usize = 10;

use crate::persistence::inventory_store::save_inventory_record;
use crate::persistence::next_id;
use crate::persistence::{
    ACCOUNT_CHARACTERS, CHARACTER_GUILDS, CHARACTER_LAST_LOGOUT, CHARACTER_NAMES, CHARACTERS,
    GUILD_MEMBER_NOTES, GUILD_PROFILES, PENDING_GUILD_INVITES,
};
use crate::persistence_types::*;
use crate::player_create_info::NewCharacterKit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildMemberRecord {
    pub character_id: u64,
    pub name: String,
    pub level: u16,
    pub class_id: u8,
    pub guild_name: String,
}

/// Never derive this from rest rows: admin rest mutations also stamp their own clock.
pub fn load_character_last_logout(db: &Database, character_id: u64) -> Option<u64> {
    let txn = db.begin_read().expect("begin observed logout read");
    let table = txn
        .open_table(CHARACTER_LAST_LOGOUT)
        .expect("open observed logout table");
    table
        .get(character_id)
        .expect("read observed logout")
        .map(|row| row.value())
}

/// Called only by the production in-world disconnect observer, not periodic saves.
pub fn save_character_last_logout(db: &Database, character_id: u64, observed_unix_secs: u64) {
    let txn = db.begin_write().expect("begin observed logout write");
    {
        let mut table = txn
            .open_table(CHARACTER_LAST_LOGOUT)
            .expect("open observed logout table");
        table
            .insert(character_id, observed_unix_secs)
            .expect("write observed logout");
    }
    txn.commit().expect("commit observed logout");
}

/// Save a single character to the database.
pub fn save_character(db: &Database, character_id: u64, data: &CharacterData) {
    let bytes = data.encode_stored();
    let txn = db.begin_write().expect("Failed to begin write txn");
    validate_forever_equipment_sources(&txn, character_id, data);
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
    let guard = table.get(character_id).expect("Failed to read character")?;
    let mut data =
        CharacterData::decode_compatible(guard.value()).expect("Failed to decode character");
    let equipped: Vec<crate::persistence::equipment::EquippedItem> =
        crate::persistence::inventory_store::read_row(
            &txn.open_table(crate::persistence::equipment::EQUIPMENT)
                .expect("open equipment for character source hydration"),
            character_id,
        );
    let sources = txn
        .open_table(crate::persistence::inventory_store::ITEM_DEFINITION_SOURCES)
        .expect("open character item definition sources");
    hydrate_equipment_sources(&mut data, &equipped, &sources);
    Some(data)
}

pub(crate) fn hydrate_equipment_sources(
    character: &mut CharacterData,
    equipped: &[crate::persistence::equipment::EquippedItem],
    sources: &impl ReadableTable<u64, u8>,
) {
    for entry in &mut character.equipment_appearance.entries {
        entry.definition_source = entry.item_id.map(|item_id| {
            let owned = equipped.iter().find(|owned| owned.slot == entry.slot);
            match owned {
                Some(owned) => {
                    assert_eq!(
                        owned.item.item_id, item_id,
                        "cached equipment differs from owned slot"
                    );
                    crate::persistence::inventory_store::read_item_definition_source(
                        sources,
                        owned.item.item_guid,
                    )
                }
                // Existing pre-ownership item caches have no GUID and are known Retail.
                None => shared::item_data::ItemDefinitionSource::Retail,
            }
        });
    }
}

/// A source-free cache may only be written after Forever ownership is durable
/// in the same transaction (or an earlier creation transaction).
pub(crate) fn validate_forever_equipment_sources(
    txn: &redb::WriteTransaction,
    character_id: u64,
    data: &CharacterData,
) {
    use crate::persistence::{equipment, inventory_store};
    use shared::item_data::ItemDefinitionSource::Forever70205;
    let equipped: Vec<equipment::EquippedItem> = inventory_store::read_row(
        &txn.open_table(equipment::EQUIPMENT)
            .expect("open equipment for source validation"),
        character_id,
    );
    let sources = txn
        .open_table(inventory_store::ITEM_DEFINITION_SOURCES)
        .expect("open item sources for character validation");
    for entry in data
        .equipment_appearance
        .entries
        .iter()
        .filter(|entry| entry.definition_source == Some(Forever70205))
    {
        let owned = equipped
            .iter()
            .find(|owned| owned.slot == entry.slot && Some(owned.item.item_id) == entry.item_id)
            .expect("Forever cached equipment has no owned GUID");
        assert_eq!(
            inventory_store::read_item_definition_source(&sources, owned.item.item_guid),
            Forever70205,
            "Forever cached equipment differs from GUID source"
        );
    }
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
    crate::persistence::charters::join_guild_in(&write, character_id, guild_id, guild_name);
    write.commit().expect("Failed to commit character guild");
    Ok(())
}

/// Remove membership/local data and its actual lifecycle event atomically.
/// Daily bank usage remains, so leaving/rejoining cannot reset today's allowance.
pub fn remove_character_guild(db: &Database, character_id: u64, actor: u64) {
    let txn = db.begin_write().expect("begin guild member removal");
    let guild_id = txn
        .open_table(CHARACTER_GUILDS)
        .expect("open memberships")
        .remove(character_id)
        .expect("remove membership")
        .map(|record| {
            bitcode::decode::<GuildMembershipData>(record.value())
                .expect("decode removed membership")
                .guild_id
        })
        .expect("removed member belongs to a guild");
    txn.open_table(crate::persistence::banks::GUILD_MEMBER_RANKS)
        .expect("open ranks")
        .remove(character_id)
        .expect("remove rank");
    txn.open_table(GUILD_MEMBER_NOTES)
        .expect("open notes")
        .remove(character_id)
        .expect("remove note");
    txn.open_table(crate::persistence::GUILD_PUBLIC_NOTES)
        .expect("open public notes")
        .remove(character_id)
        .expect("remove public note");
    txn.open_table(PENDING_GUILD_INVITES)
        .expect("open invites")
        .remove(character_id)
        .expect("remove invite");
    let member = crate::guild_event_log::character_name_in(&txn, character_id);
    let event = if actor == character_id {
        shared::protocol::GuildEventKind::Quit { member }
    } else {
        shared::protocol::GuildEventKind::Remove {
            actor: crate::guild_event_log::character_name_in(&txn, actor),
            member,
        }
    };
    crate::guild_event_log::record_in(&txn, guild_id, event);
    txn.commit()
        .expect("commit guild member removal and lifecycle event");
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
    crate::guild_event_log::record_in(
        &write,
        guild_id,
        shared::protocol::GuildEventKind::Invite {
            actor: inviter_name.to_owned(),
            member: crate::guild_event_log::character_name_in(&write, character_id),
        },
    );
    write
        .commit()
        .expect("Failed to commit pending guild invite and lifecycle event");
}

pub fn accept_pending_guild_invite(
    db: &Database,
    character_id: u64,
) -> Result<PendingGuildInviteData, String> {
    let invite = load_pending_guild_invite(db, character_id).ok_or("No pending guild invite")?;
    let rank = crate::persistence::banks::load_guild(db, invite.guild_id).lowest_rank();
    let membership = GuildMembershipData {
        guild_id: invite.guild_id,
        guild_name: invite.guild_name.clone(),
    };
    let encoded = bitcode::encode(&membership);
    let txn = db.begin_write().expect("begin guild invite acceptance");
    {
        let mut members = txn.open_table(CHARACTER_GUILDS).expect("open memberships");
        if members
            .get(character_id)
            .expect("read membership")
            .is_some()
        {
            return Err("You are already in a guild.".into());
        }
        members
            .insert(character_id, encoded.as_slice())
            .expect("join guild");
    }
    txn.open_table(crate::persistence::banks::GUILD_MEMBER_RANKS)
        .expect("open ranks")
        .insert(character_id, rank)
        .expect("assign initial rank");
    txn.open_table(PENDING_GUILD_INVITES)
        .expect("open invites")
        .remove(character_id)
        .expect("consume invite");
    crate::persistence::charters::join_guild_in(
        &txn,
        character_id,
        invite.guild_id,
        &invite.guild_name,
    );
    crate::guild_event_log::record_in(
        &txn,
        invite.guild_id,
        shared::protocol::GuildEventKind::Join {
            member: crate::guild_event_log::character_name_in(&txn, character_id),
        },
    );
    txn.commit()
        .expect("commit guild invite acceptance and lifecycle event");
    Ok(invite)
}

pub fn load_pending_guild_invite(
    db: &Database,
    character_id: u64,
) -> Option<PendingGuildInviteData> {
    let txn = db.begin_read().expect("begin pending invite read");
    let table = txn
        .open_table(PENDING_GUILD_INVITES)
        .expect("open pending invites");
    table
        .get(character_id)
        .expect("read pending invite")
        .map(|raw| bitcode::decode(raw.value()).expect("decode pending invite"))
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

pub fn load_guild_public_note(db: &Database, character_id: u64) -> String {
    let read = db.begin_read().expect("read public guild note");
    let table = match read.open_table(crate::persistence::GUILD_PUBLIC_NOTES) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return String::new(),
        Err(error) => panic!("open public guild notes: {error}"),
    };
    table
        .get(character_id)
        .expect("read public guild note")
        .map(|value| value.value().to_owned())
        .unwrap_or_default()
}

pub fn save_guild_public_note(db: &Database, character_id: u64, note: &str) {
    let write = db.begin_write().expect("write public guild note");
    write
        .open_table(crate::persistence::GUILD_PUBLIC_NOTES)
        .expect("open public guild notes")
        .insert(character_id, note)
        .expect("save public guild note");
    write.commit().expect("commit public guild note");
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

/// Character id of `character_name` compared ASCII case-insensitively.
pub fn find_character_id_by_name_ignore_case(db: &Database, character_name: &str) -> Option<u64> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let names = txn
        .open_table(CHARACTER_NAMES)
        .expect("Failed to open character_names");
    names
        .iter()
        .expect("Failed to iterate character names")
        .map(|row| row.expect("Failed to read character name"))
        .find(|(name, _)| name.value().eq_ignore_ascii_case(character_name))
        .map(|(_, id)| id.value())
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
    let table = match txn.open_table(ACCOUNT_CHARACTERS) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return None,
        Err(error) => panic!("Failed to open account_characters: {error}"),
    };
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

/// Test fixture: a new character with an empty kit (no items, no action buttons).
#[cfg(test)]
pub fn create_character(
    db: &Database,
    account_id: u64,
    name: &str,
    race: u8,
    class: u8,
) -> Result<(u64, CharacterData), String> {
    let bare = crate::player_create_info::OwnedKit::bare();
    create_character_with_appearance(
        db,
        account_id,
        name,
        race,
        class,
        CharacterAppearance::default(),
        &bare.kit(),
    )
}

pub fn create_character_with_appearance(
    db: &Database,
    account_id: u64,
    name: &str,
    race: u8,
    class: u8,
    appearance: CharacterAppearance,
    kit: &NewCharacterKit,
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
    crate::race_class::validate_race_class(race, class)?;
    if appearance.visage.is_some() && !crate::race_class::has_visage_form(race) {
        return Err(format!("Race {race} has no visage form"));
    }
    let location = kit.starting_location(race, class)?;
    let character_id = next_id(db, "character");
    let data = default_character_data(name, race, class, appearance, location);
    let progression = kit.starting_progression(race, class)?;
    let mut mint = || next_id(db, "item_instance");
    let mut inventory = kit.starting_inventory(data, &mut mint)?;
    inventory.appearance_account = Some(account_id);
    save_inventory_record(db, character_id, &inventory);
    save_new_character(
        db,
        account_id,
        character_id,
        name,
        &inventory.character,
        location,
    );
    crate::maps::persistence::save_character_map(db, character_id, location.map_id);
    crate::persistence_progression::save_progression(db, character_id, &progression);
    Ok((character_id, inventory.character))
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

fn default_character_data(
    name: &str,
    race: u8,
    class: u8,
    appearance: CharacterAppearance,
    location: crate::player_start::StartLocation,
) -> CharacterData {
    let crate::player_start::StartLocation { x, y, z, .. } = location;
    CharacterData {
        name: name.to_string(),
        x,
        y,
        z,
        health: 100.0,
        max_health: 100.0,
        race,
        class,
        level: u16::from(crate::race_class::start_level(race, class)),
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
    location: crate::player_start::StartLocation,
) {
    let bytes = data.encode_stored();
    let txn = db.begin_write().expect("Failed to begin write txn");
    validate_forever_equipment_sources(&txn, character_id, data);
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
    crate::spell_cast::homebind::save_bind_in(
        &txn,
        character_id,
        crate::maps::WorldLocation {
            map_id: location.map_id,
            position: bevy::math::Vec3::new(data.x, data.y, data.z),
            orientation: location.orientation,
        },
    );
    txn.commit().expect("Failed to commit new character");
}

/// Test fixture: a new character whose bags are granted 20 Linen Cloth, 10
/// Peacebloom and 10 Copper Ore, with 10 gold (new characters start empty).
#[cfg(test)]
pub(crate) fn create_character_with_test_bags(
    db: &Database,
    account_id: u64,
    name: &str,
    race: u8,
    class: u8,
) -> Result<(u64, CharacterData), String> {
    let created = create_character(db, account_id, name, race, class)?;
    let item = |item_id, name: &str, stack_count, vendor_sell_price| InventoryItemData {
        item_guid: next_id(db, "item_instance"),
        item_id,
        name: name.to_string(),
        quality: 1,
        required_level: 1,
        stack_count,
        vendor_sell_price,
        unique_auction: false,
    };
    let bags = InventoryData {
        gold: 100_000,
        items: vec![
            item(2589, "Linen Cloth", 20, 13),
            item(2447, "Peacebloom", 10, 10),
            item(2770, "Copper Ore", 10, 25),
        ],
    };
    crate::persistence::save_inventory(db, created.0, &bags);
    Ok(created)
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
        txn.open_table(CHARACTER_LAST_LOGOUT)
            .expect("open observed logout table")
            .remove(character_id)
            .expect("remove observed logout");
        let mut names = txn.open_table(CHARACTER_NAMES).expect("open names");
        names.remove(name).expect("remove character name");
        let mut acct = txn.open_table(ACCOUNT_CHARACTERS).expect("open acct chars");
        acct.insert(account_id, encoded.as_slice())
            .expect("update acct chars");
        let mut progression = txn
            .open_table(crate::persistence_progression::CHARACTER_PROGRESSION)
            .expect("open character progression");
        progression
            .remove(character_id)
            .expect("remove character progression");
        let mut traits = txn
            .open_table(crate::persistence_progression::CHARACTER_TRAITS)
            .expect("open character traits");
        traits
            .remove(character_id)
            .expect("remove character traits");
        txn.open_table(crate::persistence_progression::CHARACTER_TRAIT_LOADOUTS)
            .expect("open character trait loadouts")
            .remove(character_id)
            .expect("remove character trait loadouts");
        let mut xp = txn
            .open_table(crate::persistence_progression::CHARACTER_XP)
            .expect("open character xp");
        xp.remove(character_id).expect("remove character xp");
        let mut flight_style = txn
            .open_table(crate::persistence_progression::CHARACTER_FLIGHT_STYLE)
            .expect("open character flight style");
        flight_style
            .remove(character_id)
            .expect("remove character flight style");
        let mut rest = txn
            .open_table(crate::persistence_progression::CHARACTER_REST)
            .expect("open character rest");
        rest.remove(character_id).expect("remove character rest");
        let mut corpses = txn
            .open_table(crate::persistence::CHARACTER_CORPSES)
            .expect("open character corpses");
        corpses
            .remove(character_id)
            .expect("remove character corpse");
        let mut maps = txn
            .open_table(crate::maps::persistence::CHARACTER_MAPS)
            .expect("open character maps");
        maps.remove(character_id).expect("remove character map");
    }
    txn.commit().expect("Failed to commit character deletion");
}
