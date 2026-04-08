use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{
    CombatRatings, GuildMembership, Health, Level, Mana, MovementSpeed, Player, Position, Rotation,
    SelectedTalents, UnitStats, VerticalVelocity,
};
use shared::protocol::{
    AuthChannel, CreateCharacter, CreateCharacterResponse, DeleteCharacter,
    DeleteCharacterResponse, EnterWorldResponse, LoadTerrain, SelectCharacter,
};
use std::collections::HashSet;

use crate::auth::{Authenticated, InWorld, SentTiles};
use crate::networking::{CombatTarget, PlayerLink, queue_player_despawn, send_load_terrain};
use crate::persistence::{self, CharacterData, ClientId, GameDatabase};

/// Default run speed in yards per second (WoW run speed).
const DEFAULT_RUN_SPEED: f32 = 7.0;

/// Process character creation requests from authenticated connections.
pub fn process_create_character(
    mut receivers: Query<(
        Entity,
        &Authenticated,
        &mut MessageReceiver<CreateCharacter>,
    )>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<CreateCharacterResponse>>,
) {
    let Some(db) = db.as_ref() else { return };
    let requests: Vec<_> = collect_create_requests(&mut receivers);
    for (entity, account_id, req) in requests {
        handle_create(entity, account_id, &req, db, &mut senders);
    }
}

pub(crate) fn collect_create_requests(
    receivers: &mut Query<(
        Entity,
        &Authenticated,
        &mut MessageReceiver<CreateCharacter>,
    )>,
) -> Vec<(Entity, u64, CreateCharacter)> {
    let mut out = Vec::new();
    for (entity, auth, mut receiver) in receivers.iter_mut() {
        for req in receiver.receive() {
            out.push((entity, auth.account_id, req));
        }
    }
    out
}

fn handle_create(
    entity: Entity,
    account_id: u64,
    req: &CreateCharacter,
    db: &GameDatabase,
    senders: &mut Query<&mut MessageSender<CreateCharacterResponse>>,
) {
    let Ok(mut sender) = senders.get_mut(entity) else {
        return;
    };
    match persistence::create_character_with_appearance(
        &db.db,
        account_id,
        &req.name,
        req.race,
        req.class,
        req.appearance,
    ) {
        Ok((cid, data)) => {
            let entry = shared::protocol::CharacterListEntry {
                character_id: cid,
                name: data.name,
                level: data.level,
                race: data.race,
                class: data.class,
                appearance: data.appearance,
                equipment_appearance: data.equipment_appearance,
            };
            sender.send::<AuthChannel>(CreateCharacterResponse {
                success: true,
                character: Some(entry),
                error: None,
            });
        }
        Err(e) => {
            sender.send::<AuthChannel>(CreateCharacterResponse {
                success: false,
                character: None,
                error: Some(e),
            });
        }
    }
}

/// Process character deletion requests from authenticated connections.
pub fn process_delete_character(
    mut receivers: Query<(
        Entity,
        &Authenticated,
        &mut MessageReceiver<DeleteCharacter>,
    )>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<DeleteCharacterResponse>>,
) {
    let Some(db) = db.as_ref() else { return };
    let requests: Vec<_> = collect_delete_requests(&mut receivers);
    for (entity, account_id, req) in requests {
        handle_delete(entity, account_id, &req, db, &mut senders);
    }
}

pub(crate) fn collect_delete_requests(
    receivers: &mut Query<(
        Entity,
        &Authenticated,
        &mut MessageReceiver<DeleteCharacter>,
    )>,
) -> Vec<(Entity, u64, DeleteCharacter)> {
    let mut out = Vec::new();
    for (entity, auth, mut receiver) in receivers.iter_mut() {
        for req in receiver.receive() {
            out.push((entity, auth.account_id, req));
        }
    }
    out
}

fn handle_delete(
    entity: Entity,
    account_id: u64,
    req: &DeleteCharacter,
    db: &GameDatabase,
    senders: &mut Query<&mut MessageSender<DeleteCharacterResponse>>,
) {
    let Ok(mut sender) = senders.get_mut(entity) else {
        return;
    };
    match persistence::delete_character(&db.db, account_id, req.character_id) {
        Ok(()) => {
            sender.send::<AuthChannel>(DeleteCharacterResponse {
                success: true,
                character_id: req.character_id,
                error: None,
            });
        }
        Err(e) => {
            sender.send::<AuthChannel>(DeleteCharacterResponse {
                success: false,
                character_id: req.character_id,
                error: Some(e),
            });
        }
    }
}

/// Process character selection and world entry.
pub fn process_select_character(
    mut receivers: Query<
        (
            Entity,
            &Authenticated,
            &mut MessageReceiver<SelectCharacter>,
        ),
        Without<InWorld>,
    >,
    db: Option<Res<GameDatabase>>,
    mut enter_senders: Query<&mut MessageSender<EnterWorldResponse>>,
    mut terrain_senders: Query<&mut MessageSender<LoadTerrain>>,
    existing_players: Query<(Entity, &ClientId)>,
    mut commands: Commands,
) {
    let Some(db) = db.as_ref() else { return };
    let requests: Vec<_> = collect_select_requests(&mut receivers);
    for (entity, account_id, req) in requests {
        despawn_stale_player(&existing_players, req.character_id, &mut commands);
        handle_select(
            entity,
            account_id,
            &req,
            db,
            &mut enter_senders,
            &mut terrain_senders,
            &mut commands,
        );
    }
}

pub(crate) fn collect_select_requests(
    receivers: &mut Query<
        (
            Entity,
            &Authenticated,
            &mut MessageReceiver<SelectCharacter>,
        ),
        Without<InWorld>,
    >,
) -> Vec<(Entity, u64, SelectCharacter)> {
    let mut out = Vec::new();
    for (entity, auth, mut receiver) in receivers.iter_mut() {
        for req in receiver.receive() {
            if !push_unique_select_request(&mut out, entity, auth.account_id, req) {
                warn!(
                    "Ignoring duplicate SelectCharacter for connection {:?} in same tick",
                    entity
                );
            }
        }
    }
    out
}

pub(crate) fn push_unique_select_request(
    out: &mut Vec<(Entity, u64, SelectCharacter)>,
    entity: Entity,
    account_id: u64,
    req: SelectCharacter,
) -> bool {
    if out.iter().any(|(existing, _, _)| *existing == entity) {
        return false;
    }
    out.push((entity, account_id, req));
    true
}

pub(crate) fn despawn_stale_player(
    existing: &Query<(Entity, &ClientId)>,
    character_id: u64,
    commands: &mut Commands,
) {
    for (entity, cid) in existing.iter() {
        if cid.0 == character_id {
            warn!("Despawning stale player entity {entity:?} for character {character_id}");
            queue_player_despawn(commands, entity);
        }
    }
}

fn handle_select(
    conn: Entity,
    account_id: u64,
    req: &SelectCharacter,
    db: &GameDatabase,
    enter_senders: &mut Query<&mut MessageSender<EnterWorldResponse>>,
    terrain_senders: &mut Query<&mut MessageSender<LoadTerrain>>,
    commands: &mut Commands,
) {
    if !validate_ownership(db, account_id, req.character_id) {
        send_enter_error(conn, enter_senders, "Character not found");
        return;
    }
    let Some(data) = persistence::load_character(&db.db, req.character_id) else {
        send_enter_error(conn, enter_senders, "Character data missing");
        return;
    };
    let guild = persistence::load_character_guild(&db.db, req.character_id);
    let player_entity = spawn_player_entity(commands, &data, req.character_id, guild.as_ref());
    insert_world_components(commands, conn, player_entity, req.character_id);
    send_enter_success(conn, enter_senders, player_entity);
    send_load_terrain(conn, terrain_senders);
    info!(
        "Character {} entered world (account {})",
        req.character_id, account_id
    );
}

fn validate_ownership(db: &GameDatabase, account_id: u64, character_id: u64) -> bool {
    let chars = persistence::list_characters_for_account(&db.db, account_id);
    chars.iter().any(|(id, _)| *id == character_id)
}

fn insert_world_components(
    commands: &mut Commands,
    conn: Entity,
    player_entity: Entity,
    character_id: u64,
) {
    let mut initial_tiles = HashSet::new();
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let ty = (32i32 + dy).clamp(0, 63) as u32;
            let tx = (48i32 + dx).clamp(0, 63) as u32;
            initial_tiles.insert((ty, tx));
        }
    }
    commands.entity(conn).insert((
        PlayerLink(player_entity),
        SentTiles {
            tiles: initial_tiles,
        },
        InWorld { character_id },
    ));
}

fn send_enter_success(
    entity: Entity,
    senders: &mut Query<&mut MessageSender<EnterWorldResponse>>,
    player_entity: Entity,
) {
    let Ok(mut sender) = senders.get_mut(entity) else {
        return;
    };
    sender.send::<AuthChannel>(EnterWorldResponse {
        success: true,
        player_entity: Some(player_entity.to_bits()),
        error: None,
    });
}

fn send_enter_error(
    entity: Entity,
    senders: &mut Query<&mut MessageSender<EnterWorldResponse>>,
    error: &str,
) {
    let Ok(mut sender) = senders.get_mut(entity) else {
        return;
    };
    sender.send::<AuthChannel>(EnterWorldResponse {
        success: false,
        player_entity: None,
        error: Some(error.to_string()),
    });
}

fn build_player_components(data: &CharacterData, character_id: u64) -> impl Bundle {
    // CharacterData stores WoW coords (from DB); convert to Bevy space for ECS.
    let (bx, by, bz) = crate::terrain_height::wow_to_bevy(data.x, data.y, data.z);
    (
        Position {
            x: bx,
            y: by,
            z: bz,
        },
        Health {
            current: data.health,
            max: data.max_health,
        },
        Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        MovementSpeed(DEFAULT_RUN_SPEED),
        VerticalVelocity::default(),
        Player {
            name: data.name.clone(),
            race: data.race,
            class: data.class,
            appearance: data.appearance,
        },
        UnitStats::default(),
        CombatRatings::default(),
        Level(data.level as u8),
        data.equipment_appearance.clone(),
        SelectedTalents::default(),
        ClientId(character_id),
        CombatTarget::default(),
        Replicate::new(ReplicationMode::SingleServer(NetworkTarget::All)),
        NetworkVisibility,
    )
}

/// Spawn a player entity with replicated components.
pub fn spawn_player_entity(
    commands: &mut Commands,
    data: &CharacterData,
    character_id: u64,
    guild: Option<&GuildMembership>,
) -> Entity {
    let entity = commands
        .spawn(build_player_components(data, character_id))
        .id();
    if shared::formulas::base_mana(data.class, data.level as u8).is_some() {
        commands.entity(entity).insert(Mana {
            current: 0.0,
            max: 0.0,
        });
    }
    if let Some(g) = guild {
        commands.entity(entity).insert(g.clone());
    }
    info!(
        "Spawned player '{}' at ({}, {}, {})",
        data.name, data.x, data.y, data.z
    );
    entity
}
