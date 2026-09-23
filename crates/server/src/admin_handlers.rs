//! Handler functions for admin IPC commands.

use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry,
    Position,
};
use shared::protocol::{AuthChannel, CharacterListUpdate, LoginResponse};

use crate::admin_types::*;
use crate::persistence;
use crate::persistence_accounts;

pub(crate) fn handle_combat_log(
    combat_log: &crate::combat::CombatLog,
    time: &Time,
    last_seconds: f64,
) -> AdminResponse {
    let now = time.elapsed_secs_f64();
    let start = (now - last_seconds).max(0.0);
    let entries: Vec<CombatLogSummary> = combat_log
        .range(start, now)
        .iter()
        .map(|e| CombatLogSummary {
            timestamp: e.timestamp,
            event_type: format!("{:?}", e.event.event_type),
            attacker: e.event.attacker,
            target: e.event.target,
            amount: e.event.amount,
            spell_id: e.event.spell_id,
        })
        .collect();
    let count = entries.len();
    AdminResponse::CombatLogEntries { count, entries }
}

pub(crate) fn handle_list_accounts(db: &persistence::GameDatabase) -> AdminResponse {
    let accounts = persistence_accounts::list_accounts(&db.db)
        .into_iter()
        .map(|account| AccountInfo {
            username: account.username,
            account_id: account.account_id,
            token_count: account.token_count,
        })
        .collect();
    AdminResponse::AccountsListed { accounts }
}

pub(crate) fn handle_list_characters(
    db: &persistence::GameDatabase,
    username_filter: Option<&str>,
) -> AdminResponse {
    let mut characters: Vec<_> = persistence_accounts::list_accounts(&db.db)
        .into_iter()
        .filter(|account| {
            username_filter
                .map(|username| account.username == username)
                .unwrap_or(true)
        })
        .flat_map(|account| {
            let username = account.username.clone();
            persistence::list_characters_for_account(&db.db, account.account_id)
                .into_iter()
                .map(move |(character_id, data)| CharacterInfo {
                    username: username.clone(),
                    account_id: account.account_id,
                    character_id,
                    name: data.name,
                    level: data.level,
                    race: data.race,
                    class: data.class,
                })
        })
        .collect();
    characters.sort_by(|a, b| {
        a.username
            .cmp(&b.username)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.character_id.cmp(&b.character_id))
    });
    AdminResponse::CharactersListed { characters }
}

pub(crate) fn handle_find_character(db: &persistence::GameDatabase, name: &str) -> AdminResponse {
    let character = match handle_list_characters(db, None) {
        AdminResponse::CharactersListed { characters } => characters
            .into_iter()
            .find(|character| character.name == name),
        _ => None,
    };
    AdminResponse::CharacterFound { character }
}

pub(crate) fn handle_move_characters(
    db: &persistence::GameDatabase,
    from_username: &str,
    to_username: &str,
) -> AdminResponse {
    let Some(from_account_id) =
        persistence_accounts::find_account_by_username(&db.db, from_username)
    else {
        return AdminResponse::Error(format!("account '{from_username}' not found"));
    };
    let Some(to_account_id) = persistence_accounts::find_account_by_username(&db.db, to_username)
    else {
        return AdminResponse::Error(format!("account '{to_username}' not found"));
    };
    match persistence::move_characters_between_accounts(&db.db, from_account_id, to_account_id) {
        Ok(moved_count) => AdminResponse::CharactersMoved {
            from_username: from_username.to_owned(),
            to_username: to_username.to_owned(),
            moved_count,
        },
        Err(err) => AdminResponse::Error(err),
    }
}

pub(crate) fn handle_check_account(
    db: &persistence::GameDatabase,
    username: &str,
) -> AdminResponse {
    let account = persistence_accounts::inspect_account(&db.db, username);
    AdminResponse::AccountChecked {
        username: account.username,
        exists: account.exists,
        account_id: account.account_id,
        token_count: account.token_count,
        pending_registration: account.pending_registration,
        banned: account.banned,
        ban_reason: account.ban_reason,
    }
}

pub(crate) fn handle_check_password(
    db: &persistence::GameDatabase,
    username: &str,
    password: &str,
) -> AdminResponse {
    match persistence_accounts::check_password(&db.db, username, password) {
        Ok(valid) => AdminResponse::PasswordChecked {
            username: username.to_owned(),
            valid,
        },
        Err(err) => AdminResponse::Error(err),
    }
}

pub(crate) fn handle_create_account(
    db: &persistence::GameDatabase,
    username: &str,
    password: &str,
) -> AdminResponse {
    match persistence_accounts::register_account(&db.db, username, password) {
        Ok((account_id, token)) => AdminResponse::AccountCreated { account_id, token },
        Err(e) => AdminResponse::Error(e),
    }
}

pub(crate) fn handle_approve_registration(
    db: &persistence::GameDatabase,
    username: &str,
) -> AdminResponse {
    match persistence_accounts::approve_registration(&db.db, username) {
        Ok((account_id, token)) => AdminResponse::RegistrationApproved { account_id, token },
        Err(e) => AdminResponse::Error(e),
    }
}

pub(crate) fn handle_set_credentials(
    db: &persistence::GameDatabase,
    token: &str,
    username: &str,
    password: &str,
) -> AdminResponse {
    match persistence_accounts::set_account_credentials(&db.db, token, username, password) {
        Ok(account_id) => AdminResponse::CredentialsSet {
            account_id,
            username: username.to_owned(),
        },
        Err(e) => AdminResponse::Error(e),
    }
}

pub(crate) fn handle_set_level(
    db: &persistence::GameDatabase,
    commands: &mut Commands,
    name: &str,
    level: u16,
) -> AdminResponse {
    if !(1..=crate::spell_info::MAX_LEVEL as u16).contains(&level) {
        return AdminResponse::Error(format!(
            "level must be 1..={}",
            crate::spell_info::MAX_LEVEL
        ));
    }
    let Some(char_id) = persistence::find_character_id_by_name(&db.db, name) else {
        return AdminResponse::Error(format!("character '{name}' not found"));
    };
    let Some(mut data) = persistence::load_character(&db.db, char_id) else {
        return AdminResponse::Error(format!("character data for '{name}' not found"));
    };
    data.level = level;
    persistence::save_character(&db.db, char_id, &data);
    commands.queue(move |world: &mut World| {
        crate::class_progression::set_live_level(world, char_id, level as u8);
    });
    AdminResponse::LevelSet {
        character_name: name.to_owned(),
        level,
    }
}

pub(crate) fn handle_set_position(
    db: &persistence::GameDatabase,
    players: &mut Query<(Entity, &mut Position, &persistence::ClientId)>,
    interest_dirty: Option<&crate::interest::InterestDirtyQueue>,
    name: &str,
    x: f32,
    y: f32,
    z: f32,
) -> AdminResponse {
    let Some(char_id) = persistence::find_character_id_by_name(&db.db, name) else {
        return AdminResponse::Error(format!("character '{name}' not found"));
    };
    let Some(mut data) = persistence::load_character(&db.db, char_id) else {
        return AdminResponse::Error(format!("character data for '{name}' not found"));
    };
    data.x = x;
    data.y = y;
    data.z = z;
    persistence::save_character(&db.db, char_id, &data);

    let (bx, by, bz) = crate::terrain_height::wow_to_bevy(x, y, z);
    let next_position = Position {
        x: bx,
        y: by,
        z: bz,
    };
    for (entity, mut pos, cid) in players.iter_mut() {
        if cid.0 == char_id && *pos != next_position {
            *pos = next_position;
            crate::interest::mark_interest_position_changed(interest_dirty, entity);
        }
    }

    AdminResponse::PositionSet {
        character_name: name.to_owned(),
        x,
        y,
        z,
    }
}

pub(crate) struct EquipmentAppearanceRequest<'a> {
    pub(crate) name: &'a str,
    pub(crate) slot_name: &'a str,
    pub(crate) display_info_id: u32,
}

pub(crate) fn handle_set_appearance(
    db: &persistence::GameDatabase,
    authenticated_query: &Query<
        (Entity, &crate::auth::Authenticated),
        With<MessageSender<LoginResponse>>,
    >,
    roster_update_senders: &mut Query<&mut MessageSender<CharacterListUpdate>>,
    name: &str,
    appearance: CharacterAppearance,
) -> AdminResponse {
    let Some(char_id) = persistence::find_character_id_by_name(&db.db, name) else {
        return AdminResponse::Error(format!("character '{name}' not found"));
    };
    let Some(mut data) = persistence::load_character(&db.db, char_id) else {
        return AdminResponse::Error(format!("character data for '{name}' not found"));
    };
    data.appearance = appearance.clone();
    persistence::save_character(&db.db, char_id, &data);
    send_character_list_update(
        db,
        authenticated_query,
        roster_update_senders,
        char_id,
        &data,
    );

    AdminResponse::AppearanceSet {
        character_name: name.to_owned(),
        appearance,
    }
}

pub(crate) fn handle_grant_equipment(
    db: &persistence::GameDatabase,
    commands: &mut Commands,
    equipment_query: &mut Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
    authenticated_query: &Query<
        (Entity, &crate::auth::Authenticated),
        With<MessageSender<LoginResponse>>,
    >,
    roster_update_senders: &mut Query<&mut MessageSender<CharacterListUpdate>>,
    name: &str,
    items: &[crate::equipment_admin_types::EquipmentGrantItem],
) -> AdminResponse {
    let (character_id, data, owned) =
        match persistence::equipment::grant_equipment(&db.db, name, items) {
            Ok(result) => result,
            Err(error) => return AdminResponse::Error(error),
        };
    for (entity, appearance, cid) in equipment_query.iter_mut() {
        if cid.0 == character_id && *appearance != data.equipment_appearance {
            commands
                .entity(entity)
                .insert(data.equipment_appearance.clone());
        }
    }
    send_character_list_update(
        db,
        authenticated_query,
        roster_update_senders,
        character_id,
        &data,
    );
    AdminResponse::EquipmentGranted {
        character_id,
        character_name: name.into(),
        items: owned,
    }
}

pub(crate) fn handle_list_equipment(db: &persistence::GameDatabase, name: &str) -> AdminResponse {
    match persistence::equipment::list_equipment(&db.db, name) {
        Ok((character_id, items)) => AdminResponse::EquipmentListed {
            character_id,
            character_name: name.into(),
            items,
        },
        Err(error) => AdminResponse::Error(error),
    }
}

pub(crate) fn handle_set_equipment_appearance(
    db: &persistence::GameDatabase,
    commands: &mut Commands,
    equipment_query: &mut Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
    authenticated_query: &Query<
        (Entity, &crate::auth::Authenticated),
        With<MessageSender<LoginResponse>>,
    >,
    roster_update_senders: &mut Query<&mut MessageSender<CharacterListUpdate>>,
    req: EquipmentAppearanceRequest<'_>,
) -> AdminResponse {
    let EquipmentAppearanceRequest {
        name,
        slot_name,
        display_info_id,
    } = req;
    let Some(slot) = parse_equipment_visual_slot(slot_name) else {
        return AdminResponse::Error(format!("invalid equipment appearance slot '{slot_name}'"));
    };
    let Some(char_id) = persistence::find_character_id_by_name(&db.db, name) else {
        return AdminResponse::Error(format!("character '{name}' not found"));
    };
    let Some(mut data) = persistence::load_character(&db.db, char_id) else {
        return AdminResponse::Error(format!("character data for '{name}' not found"));
    };

    show_equipment_appearance(&mut data.equipment_appearance, slot, display_info_id);
    persistence::save_character(&db.db, char_id, &data);
    send_character_list_update(
        db,
        authenticated_query,
        roster_update_senders,
        char_id,
        &data,
    );

    for (entity, appearance, cid) in equipment_query.iter_mut() {
        if cid.0 == char_id {
            let mut next_appearance = appearance.clone();
            show_equipment_appearance(&mut next_appearance, slot, display_info_id);
            if *appearance != next_appearance {
                commands.entity(entity).insert(next_appearance);
            }
        }
    }

    AdminResponse::EquipmentAppearanceSet {
        character_name: name.to_owned(),
        slot: format!("{slot:?}"),
        display_info_id,
    }
}

pub(crate) fn handle_clear_equipment_appearance(
    db: &persistence::GameDatabase,
    commands: &mut Commands,
    equipment_query: &mut Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
    authenticated_query: &Query<
        (Entity, &crate::auth::Authenticated),
        With<MessageSender<LoginResponse>>,
    >,
    roster_update_senders: &mut Query<&mut MessageSender<CharacterListUpdate>>,
    name: &str,
    slot_name: &str,
) -> AdminResponse {
    let Some(slot) = parse_equipment_visual_slot(slot_name) else {
        return AdminResponse::Error(format!("invalid equipment appearance slot '{slot_name}'"));
    };
    let Some(char_id) = persistence::find_character_id_by_name(&db.db, name) else {
        return AdminResponse::Error(format!("character '{name}' not found"));
    };
    let Some(mut data) = persistence::load_character(&db.db, char_id) else {
        return AdminResponse::Error(format!("character data for '{name}' not found"));
    };

    hide_equipment_appearance(&mut data.equipment_appearance, slot);
    persistence::save_character(&db.db, char_id, &data);
    send_character_list_update(
        db,
        authenticated_query,
        roster_update_senders,
        char_id,
        &data,
    );

    for (entity, appearance, cid) in equipment_query.iter_mut() {
        if cid.0 == char_id {
            let mut next_appearance = appearance.clone();
            hide_equipment_appearance(&mut next_appearance, slot);
            if *appearance != next_appearance {
                commands.entity(entity).insert(next_appearance);
            }
        }
    }

    AdminResponse::EquipmentAppearanceCleared {
        character_name: name.to_owned(),
        slot: format!("{slot:?}"),
    }
}

fn send_character_list_update(
    db: &persistence::GameDatabase,
    authenticated_query: &Query<
        (Entity, &crate::auth::Authenticated),
        With<MessageSender<LoginResponse>>,
    >,
    roster_update_senders: &mut Query<&mut MessageSender<CharacterListUpdate>>,
    character_id: u64,
    data: &persistence::CharacterData,
) {
    let Some(account_id) = persistence::find_account_id_for_character(&db.db, character_id) else {
        return;
    };
    let update = CharacterListUpdate {
        character: shared::protocol::CharacterListEntry {
            character_id,
            name: data.name.clone(),
            level: data.level,
            race: data.race,
            class: data.class,
            appearance: data.appearance.clone(),
            equipment_appearance: data.equipment_appearance.clone(),
        },
    };
    for (entity, authenticated) in authenticated_query.iter() {
        if authenticated.account_id != account_id {
            continue;
        }
        if let Ok(mut sender) = roster_update_senders.get_mut(entity) {
            sender.send::<AuthChannel>(update.clone());
        }
    }
}

/// Find the entry for this slot, or create a blank one if missing.
fn find_or_create_slot(
    appearance: &mut EquipmentAppearance,
    slot: EquipmentVisualSlot,
) -> &mut EquippedAppearanceEntry {
    let exists = appearance.entries.iter().any(|e| e.slot == slot);
    if !exists {
        appearance.entries.push(EquippedAppearanceEntry {
            slot,
            item_id: None,
            display_info_id: None,
            inventory_type: 0,
            hidden: false,
        });
    }
    appearance
        .entries
        .iter_mut()
        .find(|e| e.slot == slot)
        .unwrap()
}

/// Set a slot to show a specific item appearance.
fn show_equipment_appearance(
    appearance: &mut EquipmentAppearance,
    slot: EquipmentVisualSlot,
    display_info_id: u32,
) {
    let entry = find_or_create_slot(appearance, slot);
    entry.display_info_id = Some(display_info_id);
    entry.hidden = false;
}

/// Set a slot to hidden (transmog: hide this slot).
fn hide_equipment_appearance(appearance: &mut EquipmentAppearance, slot: EquipmentVisualSlot) {
    let entry = find_or_create_slot(appearance, slot);
    entry.item_id = None;
    entry.display_info_id = None;
    entry.inventory_type = 0;
    entry.hidden = true;
}

fn parse_equipment_visual_slot(value: &str) -> Option<EquipmentVisualSlot> {
    match value.to_ascii_lowercase().as_str() {
        "head" => Some(EquipmentVisualSlot::Head),
        "shoulder" | "shoulders" => Some(EquipmentVisualSlot::Shoulder),
        "back" | "cloak" | "cape" => Some(EquipmentVisualSlot::Back),
        "chest" => Some(EquipmentVisualSlot::Chest),
        "shirt" => Some(EquipmentVisualSlot::Shirt),
        "tabard" => Some(EquipmentVisualSlot::Tabard),
        "wrist" | "wrists" | "bracers" => Some(EquipmentVisualSlot::Wrist),
        "hands" | "hand" | "gloves" => Some(EquipmentVisualSlot::Hands),
        "waist" | "belt" => Some(EquipmentVisualSlot::Waist),
        "legs" | "pants" => Some(EquipmentVisualSlot::Legs),
        "feet" | "boots" => Some(EquipmentVisualSlot::Feet),
        "mainhand" | "main-hand" | "mh" => Some(EquipmentVisualSlot::MainHand),
        "offhand" | "off-hand" | "oh" => Some(EquipmentVisualSlot::OffHand),
        _ => None,
    }
}

pub(crate) fn handle_create_guild(
    db: &persistence::GameDatabase,
    name: &str,
    guild_id: u32,
    guild_name: &str,
) -> AdminResponse {
    match persistence::set_character_guild_by_name(&db.db, name, guild_id, guild_name) {
        Ok(character_id) => AdminResponse::GuildCreated {
            character_id,
            guild_id,
            guild_name: guild_name.to_owned(),
        },
        Err(e) => AdminResponse::Error(e),
    }
}

// --- Ticket handlers ---

fn ticket_to_info(t: &shared::ticket::Ticket) -> TicketInfo {
    TicketInfo {
        id: t.id,
        player: t.player,
        category: format!("{:?}", t.category),
        description: t.description.clone(),
        state: format!("{:?}", t.state),
        priority: format!("{:?}", t.priority),
        assigned_to: t.assigned_to.clone(),
        response_count: t.responses.len(),
        created_at: t.created_at,
    }
}

pub(crate) fn handle_ticket_list(mgr: &shared::ticket::TicketManager) -> AdminResponse {
    let tickets = mgr.queue().into_iter().map(ticket_to_info).collect();
    AdminResponse::TicketListed { tickets }
}

pub(crate) fn handle_ticket_claim(
    mgr: &mut shared::ticket::TicketManager,
    ticket_id: u64,
    gm_name: &str,
) -> AdminResponse {
    match mgr.assign(ticket_id, gm_name) {
        Ok(()) => AdminResponse::TicketClaimed {
            ticket_id,
            gm_name: gm_name.to_string(),
        },
        Err(e) => AdminResponse::Error(format!("{e:?}")),
    }
}

pub(crate) fn handle_ticket_claim_next(
    mgr: &mut shared::ticket::TicketManager,
    gm_name: &str,
) -> AdminResponse {
    match mgr.claim_next(gm_name) {
        Ok(ticket_id) => AdminResponse::TicketClaimed {
            ticket_id,
            gm_name: gm_name.to_string(),
        },
        Err(e) => AdminResponse::Error(format!("{e:?}")),
    }
}

pub(crate) fn handle_ticket_respond(
    mgr: &mut shared::ticket::TicketManager,
    ticket_id: u64,
    gm_name: &str,
    message: &str,
    now: u64,
) -> AdminResponse {
    match mgr.respond(ticket_id, gm_name, message, now) {
        Ok(()) => AdminResponse::TicketResponded { ticket_id },
        Err(e) => AdminResponse::Error(format!("{e:?}")),
    }
}

pub(crate) fn handle_ticket_resolve(
    mgr: &mut shared::ticket::TicketManager,
    ticket_id: u64,
) -> AdminResponse {
    match mgr.resolve(ticket_id) {
        Ok(()) => AdminResponse::TicketResolved { ticket_id },
        Err(e) => AdminResponse::Error(format!("{e:?}")),
    }
}

pub(crate) fn handle_ticket_escalate(
    mgr: &mut shared::ticket::TicketManager,
    ticket_id: u64,
) -> AdminResponse {
    match mgr.escalate(ticket_id) {
        Ok(()) => AdminResponse::TicketEscalated { ticket_id },
        Err(e) => AdminResponse::Error(format!("{e:?}")),
    }
}

pub(crate) fn handle_ticket_history(
    mgr: &shared::ticket::TicketManager,
    player_id: u64,
) -> AdminResponse {
    let tickets = mgr
        .player_history(player_id)
        .into_iter()
        .map(ticket_to_info)
        .collect();
    AdminResponse::TicketHistoryListed { player_id, tickets }
}
