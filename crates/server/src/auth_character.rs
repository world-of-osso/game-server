use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{
    CombatRatings, DerivedStats, Gold, GuildMembership, Health, Level, Mana, MovementSpeed, Player,
    PlayerMotion, Position, Rotation, UnitFactionTemplate, UnitLevel, UnitStats, VerticalVelocity,
};
use shared::protocol::{
    AuthChannel, CreateCharacter, CreateCharacterResponse, DeleteCharacter,
    DeleteCharacterResponse, EnterWorldResponse, LoadTerrain, SelectCharacter,
};
use shared::reputation::default_faction_registry;
use std::collections::HashSet;

use crate::auth::{Authenticated, InWorld, SentTiles};
use crate::interest::NetworkVisibility;
use crate::item_templates::ItemTemplates;
use crate::merchant::VendorCatalog;
use crate::networking::{CombatTarget, PlayerLink, queue_player_despawn, send_load_terrain};
use crate::persistence::{self, CharacterData, ClientId, GameDatabase};
use crate::player_create_info::{NewCharacterKit, PlayerCreateInfos};
use crate::profession::PlayerProfessions;

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
    create_info: Res<PlayerCreateInfos>,
    templates: Res<ItemTemplates>,
    catalog: Res<VendorCatalog>,
    mut senders: Query<&mut MessageSender<CreateCharacterResponse>>,
) {
    let Some(db) = db.as_ref() else { return };
    let kit = NewCharacterKit {
        create_info: &create_info,
        templates: &templates,
        catalog: &catalog,
    };
    let requests = collect_create_requests(&mut receivers);
    for (entity, account_id, req) in requests {
        handle_create(entity, account_id, &req, db, &kit, &mut senders);
    }
}

pub(crate) fn collect_create_requests(
    receivers: &mut Query<(
        Entity,
        &Authenticated,
        &mut MessageReceiver<CreateCharacter>,
    )>,
) -> Vec<(Entity, u64, CreateCharacter)> {
    collect_account_requests(receivers)
}

fn handle_create(
    entity: Entity,
    account_id: u64,
    req: &CreateCharacter,
    db: &GameDatabase,
    kit: &NewCharacterKit,
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
        req.appearance.clone(),
        kit,
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
                tabard: None,
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
    let requests = collect_delete_requests(&mut receivers);
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
    collect_account_requests(receivers)
}

fn collect_account_requests<M: lightyear::prelude::Message>(
    receivers: &mut Query<(Entity, &Authenticated, &mut MessageReceiver<M>)>,
) -> Vec<(Entity, u64, M)> {
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
    existing_players: Query<LivePlayer>,
    mut commands: Commands,
    mut player_deaths: MessageWriter<crate::death::PlayerDeathTransition>,
    mut maps: crate::maps::LoginMaps,
) {
    let Some(db) = db.as_ref() else { return };
    let requests: Vec<_> = collect_select_requests(&mut receivers);
    for (entity, account_id, req) in requests {
        save_and_despawn_stale_player(&db.db, &existing_players, req.character_id, &mut commands);
        handle_select(
            CharacterSelection {
                connection: entity,
                account_id,
                request: &req,
            },
            db,
            &mut enter_senders,
            &mut terrain_senders,
            &mut commands,
            &mut player_deaths,
            &mut maps,
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

/// A player entity still in the world, with what `save_live_character` writes.
pub(crate) type LivePlayer = (
    Entity,
    &'static ClientId,
    Option<(
        &'static Player,
        &'static Position,
        &'static Health,
        Option<&'static crate::death::PlayerDeathState>,
        Option<&'static crate::maps::InMap>,
        Option<&'static crate::maps::MapTransfer>,
    )>,
);

/// The character is entering the world again while an older session's player is
/// still there (re-auth): save that player's live state first, so the new player
/// loads where the old one was, then despawn it.
pub(crate) fn save_and_despawn_stale_player(
    db: &redb::Database,
    existing: &Query<LivePlayer>,
    character_id: u64,
    commands: &mut Commands,
) {
    for (entity, cid, live) in existing.iter() {
        if cid.0 != character_id {
            continue;
        }
        if let Some((player, position, health, death, map, transfer)) = live {
            let map_id = crate::maps::saved_map_id(map, transfer);
            persistence::save_live_character(
                db,
                character_id,
                player,
                position,
                health,
                death,
                map_id,
            );
        }
        warn!("Despawning stale player entity {entity:?} for character {character_id}");
        queue_player_despawn(commands, entity);
    }
}

struct CharacterSelection<'a> {
    connection: Entity,
    account_id: u64,
    request: &'a SelectCharacter,
}

fn handle_select(
    selection: CharacterSelection<'_>,
    db: &GameDatabase,
    enter_senders: &mut Query<&mut MessageSender<EnterWorldResponse>>,
    terrain_senders: &mut Query<&mut MessageSender<LoadTerrain>>,
    commands: &mut Commands,
    player_deaths: &mut MessageWriter<crate::death::PlayerDeathTransition>,
    maps: &mut crate::maps::LoginMaps,
) {
    let character_id = selection.request.character_id;
    if !validate_ownership(db, selection.account_id, character_id) {
        send_enter_error(selection.connection, enter_senders, "Character not found");
        return;
    }
    let Some(mut data) = persistence::load_character(&db.db, character_id) else {
        send_enter_error(
            selection.connection,
            enter_senders,
            "Character data missing",
        );
        return;
    };
    if let Err(error) = crate::race_class::validate_race_class(data.race, data.class) {
        send_enter_error(selection.connection, enter_senders, &error);
        return;
    }
    let saved_map = crate::maps::persistence::load_character_map(&db.db, character_id);
    let dungeon_difficulty = crate::maps::difficulty::load_dungeon_difficulty(&db.db, character_id);
    let raid_difficulty = crate::maps::raid::load(&db.db, character_id);
    let Some(place) = maps.resolve(
        commands,
        character_id,
        dungeon_difficulty.0,
        raid_difficulty,
        saved_map,
        persisted_player_position(&data),
    ) else {
        send_enter_error(
            selection.connection,
            enter_senders,
            "Character map unavailable",
        );
        return;
    };
    (data.x, data.y, data.z) =
        crate::terrain_height::bevy_to_wow(place.position.x, place.position.y, place.position.z);
    let saved_death = persistence::load_player_death_state(&db.db, character_id);
    if saved_death
        .as_ref()
        .is_some_and(|death| death.state != shared::death::DeathState::Alive)
    {
        data.health = 0.0;
    }
    let state = load_selected_character_state(
        db,
        selection.account_id,
        character_id,
        data.race,
        data.class,
    );
    let guild = state.guild.clone();
    let player_entity = spawn_player_entity(
        commands,
        &data,
        place.map,
        character_id,
        guild.as_ref(),
        build_player_init_state(state),
    );
    commands
        .entity(player_entity)
        .insert((dungeon_difficulty, raid_difficulty));
    if let Some(bind) = crate::spell_cast::homebind::load_bind(&db.db, character_id) {
        // The creation homebind preserves source-authored facing without changing
        // CharacterData's persisted ABI or repositioning an existing character.
        if bind.0.map_id == place.map.map_id
            && bind.0.position == bevy::math::Vec3::new(data.x, data.y, data.z)
        {
            commands.entity(player_entity).insert(Rotation {
                x: 0.0,
                y: bind.0.orientation + std::f32::consts::FRAC_PI_2,
                z: 0.0,
            });
        }
        commands.entity(player_entity).insert(bind);
    }
    if let Some(death) = saved_death {
        commands.entity(player_entity).insert(death);
    } else if data.health <= 0.0 {
        // Compatibility for characters saved before full death-state persistence.
        match persistence::load_corpse(&db.db, character_id) {
            Some(corpse) => {
                commands
                    .entity(player_entity)
                    .insert(crate::death::PersistedGhost(shared::death::Corpse {
                        player: character_id,
                        map_id: corpse.map_id,
                        x: corpse.x,
                        y: corpse.y,
                        z: corpse.z,
                        created_at: corpse.created_at,
                    }));
            }
            None => {
                player_deaths.write(crate::death::PlayerDeathTransition {
                    player: player_entity,
                });
            }
        }
    }
    insert_world_components(commands, selection.connection, player_entity, character_id);
    commands.queue(move |world: &mut World| {
        crate::pvp::reconnect::logged_in(world, player_entity, character_id);
    });
    send_enter_success(selection.connection, enter_senders, player_entity);
    send_load_terrain(
        selection.connection,
        &place.directory,
        &place.position,
        terrain_senders,
    );
    info!(
        "Character {} entered world (account {})",
        character_id, selection.account_id
    );
}

fn build_player_init_state(state: SelectedCharacterState) -> PlayerInitState {
    PlayerInitState {
        gold: state.gold,
        professions: state.professions,
        achievements: state.achievements,
        pvp: state.pvp,
        world_map: state.world_map,
        friends: state.friends,
        ignore_list: state.ignore_list,
        collections: state.collections,
        currencies: crate::currency::PlayerCurrencies(state.currencies),
        reputation: state.reputation,
    }
}

fn validate_ownership(db: &GameDatabase, account_id: u64, character_id: u64) -> bool {
    let chars = persistence::list_characters_for_account(&db.db, account_id);
    chars.iter().any(|(id, _)| *id == character_id)
}

struct SelectedCharacterState {
    gold: u64,
    guild: Option<GuildMembership>,
    professions: PlayerProfessions,
    achievements: crate::achievement::PlayerAchievementState,
    pvp: crate::pvp::PlayerPvpState,
    world_map: crate::world_map::PlayerWorldMapState,
    friends: crate::friends::PlayerFriendsState,
    ignore_list: crate::ignore_list::PlayerIgnoreListState,
    collections: crate::collection::PlayerCollectionState,
    currencies: shared::currency::CurrencyWallet,
    reputation: shared::reputation::CharacterReputation,
}

fn load_selected_character_state(
    db: &GameDatabase,
    account_id: u64,
    character_id: u64,
    race: u8,
    class: u8,
) -> SelectedCharacterState {
    let currencies = persistence::load_currencies(&db.db, character_id);
    let mut pvp = persistence::load_pvp(&db.db, character_id);
    crate::pvp::sync_progress_currencies_from_wallet(&mut pvp, &currencies);

    SelectedCharacterState {
        gold: persistence::load_inventory(&db.db, character_id).gold,
        guild: persistence::load_character_guild(&db.db, character_id),
        professions: persistence::load_professions(&db.db, character_id),
        achievements: crate::achievement::load_character_progress(db, account_id, character_id),
        pvp: crate::pvp::PlayerPvpState::from_progress(pvp),
        world_map: persistence::load_world_map(&db.db, character_id),
        friends: persistence::load_friends(&db.db, character_id),
        ignore_list: persistence::load_ignore_list(&db.db, character_id),
        collections: persistence::load_collections(&db.db, character_id),
        currencies,
        reputation: persistence::load_reputation(&db.db, character_id, || {
            shared::reputation::CharacterReputation::new_for_player(
                &default_faction_registry(),
                race,
                class,
            )
        }),
    }
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
            last_tile: None,
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

fn persisted_player_position(data: &CharacterData) -> Position {
    // CharacterData stores WoW coords (from DB); convert to Bevy space for ECS.
    let (bx, by, bz) = crate::terrain_height::wow_to_bevy(data.x, data.y, data.z);
    Position {
        x: bx,
        y: by,
        z: bz,
    }
}

fn build_player_identity(data: &CharacterData) -> Player {
    Player {
        name: data.name.clone(),
        race: data.race,
        class: data.class,
        appearance: data.appearance.clone(),
    }
}

fn build_player_runtime_components(character_id: u64, data: &CharacterData) -> impl Bundle {
    (
        Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        MovementSpeed(DEFAULT_RUN_SPEED),
        PlayerMotion::default(),
        shared::components::PlayerStandState::default(),
        shared::death::DeathState::Alive,
        VerticalVelocity::default(),
        UnitStats::default(),
        CombatRatings::default(),
        DerivedStats::default(),
        Level(data.level as u8),
        UnitLevel(data.level as u8),
        ClientId(character_id),
        CombatTarget::default(),
        Replicate::to_clients(NetworkTarget::All),
        NetworkVisibility::default(),
    )
}

fn build_player_components(data: &CharacterData, character_id: u64) -> impl Bundle {
    (
        persisted_player_position(data),
        Health {
            current: data.health,
            max: data.max_health,
        },
        build_player_identity(data),
        UnitFactionTemplate(crate::faction::faction_template_for_race(data.race)),
        crate::faction::current_pvp_faction_for_race(data.race),
        data.equipment_appearance.clone(),
        build_player_runtime_components(character_id, data),
    )
}

pub struct PlayerInitState {
    pub gold: u64,
    pub professions: PlayerProfessions,
    pub achievements: crate::achievement::PlayerAchievementState,
    pub pvp: crate::pvp::PlayerPvpState,
    pub world_map: crate::world_map::PlayerWorldMapState,
    pub friends: crate::friends::PlayerFriendsState,
    pub ignore_list: crate::ignore_list::PlayerIgnoreListState,
    pub collections: crate::collection::PlayerCollectionState,
    pub currencies: crate::currency::PlayerCurrencies,
    pub reputation: shared::reputation::CharacterReputation,
}

fn insert_player_spawn_components(
    commands: &mut Commands,
    entity: Entity,
    data: &CharacterData,
    init: PlayerInitState,
) -> crate::collection::PlayerCollectionState {
    let (_, by, _) = crate::terrain_height::wow_to_bevy(data.x, data.y, data.z);
    commands.entity(entity).insert((
        crate::networking::FallTracker::grounded(by),
        crate::mirror_timers::MirrorTimers::default(),
        shared::components::CombatStatus(false),
    ));
    crate::durability::insert_equipped_durability(commands, entity, data);
    insert_player_init_state(commands, entity, init)
}

fn insert_player_init_state(
    commands: &mut Commands,
    entity: Entity,
    init: PlayerInitState,
) -> crate::collection::PlayerCollectionState {
    let PlayerInitState {
        gold,
        professions,
        achievements,
        pvp,
        world_map,
        friends,
        ignore_list,
        mut collections,
        currencies,
        reputation,
    } = init;
    // Legacy character mount IDs were display IDs and included invented starters.
    // Only authentic learned spells migrate into the account Mount Journal.
    collections.mounts.clear();
    commands.entity(entity).insert(Gold(gold));
    commands.entity(entity).insert(professions);
    commands.entity(entity).insert(achievements);
    commands.entity(entity).insert(pvp);
    commands.entity(entity).insert(world_map);
    commands.entity(entity).insert(friends);
    commands.entity(entity).insert(ignore_list);
    commands
        .entity(entity)
        .insert(crate::lfg::PlayerLfgState::default());
    commands.entity(entity).insert(collections.clone());
    commands.entity(entity).insert(currencies);
    commands.entity(entity).insert(reputation);
    collections
}

fn insert_spawn_mana_state(commands: &mut Commands, entity: Entity, data: &CharacterData) {
    if crate::stats::base_mana(data.class, data.level as u8).is_none() {
        return;
    }
    commands.entity(entity).insert(Mana {
        current: 0.0,
        max: 0.0,
    });
}

fn insert_spawn_guild_membership(
    commands: &mut Commands,
    entity: Entity,
    guild: Option<&GuildMembership>,
) {
    let Some(guild) = guild else {
        return;
    };
    commands.entity(entity).insert(guild.clone());
}

/// Spawn a player entity with replicated components.
pub fn spawn_player_entity(
    commands: &mut Commands,
    data: &CharacterData,
    map: crate::maps::InMap,
    character_id: u64,
    guild: Option<&GuildMembership>,
    init: PlayerInitState,
) -> Entity {
    let entity = commands
        .spawn((build_player_components(data, character_id), map))
        .id();
    insert_player_spawn_components(commands, entity, data, init);
    insert_spawn_mana_state(commands, entity, data);
    insert_spawn_guild_membership(commands, entity, guild);
    info!(
        "Spawned player '{}' at ({}, {}, {})",
        data.name, data.x, data.y, data.z
    );
    entity
}

#[cfg(test)]
#[path = "auth_character_tests.rs"]
mod tests;
