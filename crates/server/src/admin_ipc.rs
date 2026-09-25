use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use shared::components::{EquipmentAppearance, Position};
use shared::protocol::{CharacterListUpdate, LoginResponse};

use crate::admin_handlers::*;
use crate::admin_types::*;
use crate::persistence;
use crate::persistence_accounts;

#[path = "admin_ipc_server.rs"]
mod admin_ipc_server;

use self::admin_ipc_server::{AdminCommand, init};

pub(super) const SOCKET_PATH: &str = "/tmp/game-server-admin.sock";

pub struct AdminIpcPlugin;

impl Plugin for AdminIpcPlugin {
    fn build(&self, app: &mut App) {
        let (receiver, guard) = init();

        app.insert_non_send(receiver)
            .insert_non_send(guard)
            .add_systems(Update, poll_admin_ipc);
    }
}

/// Resources needed by the admin IPC early-dispatch handlers.
#[derive(SystemParam)]
struct AdminResources<'w> {
    time: Res<'w, Time>,
    combat_log: Res<'w, crate::combat::CombatLog>,
    ticket_mgr: ResMut<'w, shared::ticket::TicketManager>,
    motd: ResMut<'w, shared::motd::Motd>,
    autobroadcast: ResMut<'w, shared::motd::Autobroadcast>,
    interest_dirty: Option<Res<'w, crate::interest::InterestDirtyQueue>>,
    item_templates: Option<Res<'w, crate::item_templates::ItemTemplates>>,
    vendor_catalog: Option<Res<'w, crate::merchant::VendorCatalog>>,
    inventory_updates: ResMut<'w, crate::gold::GoldUpdateQueue>,
}

/// Queries needed by the admin IPC dispatch loop.
#[derive(SystemParam)]
struct AdminQueries<'w, 's> {
    commands: Commands<'w, 's>,
    players: Query<
        'w,
        's,
        (
            Entity,
            &'static mut Position,
            &'static persistence::ClientId,
        ),
    >,
    equipment: Query<
        'w,
        's,
        (
            Entity,
            &'static mut EquipmentAppearance,
            &'static persistence::ClientId,
        ),
    >,
    authenticated: Query<
        'w,
        's,
        (Entity, &'static crate::auth::Authenticated),
        With<MessageSender<LoginResponse>>,
    >,
    roster_senders: Query<'w, 's, &'static mut MessageSender<CharacterListUpdate>>,
}

/// Poll IPC commands each frame and dispatch them.
fn poll_admin_ipc(
    receiver: NonSend<mpsc::Receiver<AdminCommand>>,
    db: Option<Res<persistence::GameDatabase>>,
    mut res: AdminResources,
    mut queries: AdminQueries,
) {
    let Some(db) = db else { return };
    while let Ok(cmd) = receiver.try_recv() {
        if let Some(response) = dispatch_resource_commands(&cmd.request, &db, &mut res) {
            let _ = cmd.respond.send(response);
            continue;
        }
        dispatch(cmd, &db, &mut queries, res.interest_dirty.as_deref());
    }
}

/// Handle commands that only need resources (no ECS queries).
fn dispatch_resource_commands(
    request: &AdminRequest,
    db: &persistence::GameDatabase,
    res: &mut AdminResources,
) -> Option<AdminResponse> {
    if let AdminRequest::CombatLog { last_seconds } = request {
        return Some(handle_combat_log(&res.combat_log, &res.time, *last_seconds));
    }
    if let Some(r) = dispatch_ticket(request, &res.time, &mut res.ticket_mgr) {
        return Some(r);
    }
    if let Some(r) = dispatch_motd(request, &res.time, &mut res.motd, db) {
        return Some(r);
    }
    if let Some(r) = dispatch_autobroadcast(request, &mut res.autobroadcast) {
        return Some(r);
    }
    if let AdminRequest::GrantItem {
        character_name,
        item_id,
        count,
    } = request
    {
        let (Some(templates), Some(catalog)) = (&res.item_templates, &res.vendor_catalog) else {
            return Some(AdminResponse::Error("item data not loaded".into()));
        };
        return Some(handle_grant_item(
            db,
            (templates, catalog),
            &mut res.inventory_updates,
            character_name,
            (*item_id, *count),
        ));
    }
    None
}

/// `grant-item`: store the items in the character's bags; an online owner gets the
/// `InventoryDelta` through `GoldUpdateQueue`.
fn handle_grant_item(
    db: &persistence::GameDatabase,
    (templates, catalog): (
        &crate::item_templates::ItemTemplates,
        &crate::merchant::VendorCatalog,
    ),
    inventory_updates: &mut crate::gold::GoldUpdateQueue,
    name: &str,
    (item_id, count): (u32, u32),
) -> AdminResponse {
    let Some(character_id) = persistence::find_character_id_by_name(&db.db, name) else {
        return AdminResponse::Error(format!("character '{name}' not found"));
    };
    match crate::inventory::grant_item(&db.db, templates, catalog, character_id, (item_id, count)) {
        Ok(item_name) => {
            inventory_updates.record_items(character_id);
            AdminResponse::ItemGranted {
                character_name: name.to_owned(),
                item_id,
                name: item_name,
                count,
            }
        }
        Err(error) => AdminResponse::Error(format!("grant {count} x {item_id} to {name}: {error}")),
    }
}

/// Dispatch ticket commands, returning Some if handled.
fn dispatch_ticket(
    request: &AdminRequest,
    time: &Time,
    ticket_mgr: &mut shared::ticket::TicketManager,
) -> Option<AdminResponse> {
    match request {
        AdminRequest::TicketList => Some(handle_ticket_list(ticket_mgr)),
        AdminRequest::TicketClaim { ticket_id, gm_name } => {
            Some(handle_ticket_claim(ticket_mgr, *ticket_id, gm_name))
        }
        AdminRequest::TicketClaimNext { gm_name } => {
            Some(handle_ticket_claim_next(ticket_mgr, gm_name))
        }
        AdminRequest::TicketRespond {
            ticket_id,
            gm_name,
            message,
        } => {
            let now = time.elapsed_secs_f64() as u64;
            Some(handle_ticket_respond(
                ticket_mgr, *ticket_id, gm_name, message, now,
            ))
        }
        AdminRequest::TicketResolve { ticket_id } => {
            Some(handle_ticket_resolve(ticket_mgr, *ticket_id))
        }
        AdminRequest::TicketEscalate { ticket_id } => {
            Some(handle_ticket_escalate(ticket_mgr, *ticket_id))
        }
        AdminRequest::TicketHistory { player_id } => {
            Some(handle_ticket_history(ticket_mgr, *player_id))
        }
        _ => None,
    }
}

/// Dispatch MOTD commands, returning Some if handled.
fn dispatch_motd(
    request: &AdminRequest,
    time: &Time,
    motd: &mut shared::motd::Motd,
    db: &persistence::GameDatabase,
) -> Option<AdminResponse> {
    match request {
        AdminRequest::GetMotd => Some(AdminResponse::MotdRetrieved {
            message: motd.message.clone(),
            updated_at: motd.updated_at,
        }),
        AdminRequest::SetMotd { message } => {
            let now = time.elapsed_secs_f64() as u64;
            match motd.set(message, now) {
                Ok(()) => {
                    persistence::save_motd(&db.db, motd);
                    Some(AdminResponse::MotdSet)
                }
                Err(e) => Some(AdminResponse::Error(format!("{e:?}"))),
            }
        }
        AdminRequest::ClearMotd => {
            let now = time.elapsed_secs_f64() as u64;
            motd.clear(now);
            persistence::save_motd(&db.db, motd);
            Some(AdminResponse::MotdCleared)
        }
        _ => None,
    }
}

/// Dispatch autobroadcast commands, returning Some if handled.
fn dispatch_autobroadcast(
    request: &AdminRequest,
    ab: &mut shared::motd::Autobroadcast,
) -> Option<AdminResponse> {
    match request {
        AdminRequest::BroadcastList => Some(AdminResponse::BroadcastListed {
            messages: ab.messages.clone(),
            interval: ab.interval,
            enabled: ab.enabled,
        }),
        AdminRequest::BroadcastAdd { message } => {
            ab.add_message(message.clone());
            Some(AdminResponse::BroadcastAdded {
                index: ab.message_count() - 1,
            })
        }
        AdminRequest::BroadcastRemove { index } => match ab.remove_message(*index) {
            Some(msg) => Some(AdminResponse::BroadcastRemoved { message: msg }),
            None => Some(AdminResponse::Error("Index out of range".into())),
        },
        AdminRequest::BroadcastSetInterval { seconds } => {
            ab.interval = *seconds;
            Some(AdminResponse::BroadcastIntervalSet { seconds: *seconds })
        }
        AdminRequest::BroadcastEnable => {
            ab.enabled = true;
            Some(AdminResponse::BroadcastEnabled)
        }
        AdminRequest::BroadcastDisable => {
            ab.enabled = false;
            Some(AdminResponse::BroadcastDisabled)
        }
        _ => None,
    }
}

fn queue_account_disconnect(
    commands: &mut Commands,
    authenticated_query: &Query<
        (Entity, &crate::auth::Authenticated),
        With<MessageSender<LoginResponse>>,
    >,
    account_id: u64,
    message: &str,
) -> bool {
    let Some(entity) = authenticated_query
        .iter()
        .find_map(|(entity, auth)| (auth.account_id == account_id).then_some(entity))
    else {
        return false;
    };
    crate::auth::queue_forced_disconnect(commands, entity, message, false);
    true
}

fn kick_message(reason: Option<String>) -> String {
    match reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
    {
        Some(reason) => format!("You were kicked: {reason}"),
        None => "You were kicked by an administrator.".to_string(),
    }
}

fn ban_reason(reason: Option<String>) -> String {
    match reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
    {
        Some(reason) => reason.to_string(),
        None => "Banned by an administrator.".to_string(),
    }
}

fn dispatch(
    cmd: AdminCommand,
    db: &persistence::GameDatabase,
    queries: &mut AdminQueries<'_, '_>,
    interest_dirty: Option<&crate::interest::InterestDirtyQueue>,
) {
    let commands = &mut queries.commands;
    let players = &mut queries.players;
    let equipment_query = &mut queries.equipment;
    let authenticated_query = &queries.authenticated;
    let roster_update_senders = &mut queries.roster_senders;

    let response = match cmd.request {
        AdminRequest::Ping => AdminResponse::Pong,
        AdminRequest::CombatLog { .. } | AdminRequest::GrantItem { .. } => {
            // Handled in poll_admin_ipc before dispatch is called
            unreachable!()
        }
        AdminRequest::ListAccounts => handle_list_accounts(db),
        AdminRequest::ListCharacters { username } => {
            handle_list_characters(db, username.as_deref())
        }
        AdminRequest::FindCharacter { name } => handle_find_character(db, &name),
        AdminRequest::MoveCharacters {
            from_username,
            to_username,
        } => handle_move_characters(db, &from_username, &to_username),
        AdminRequest::CheckAccount { username } => handle_check_account(db, &username),
        AdminRequest::CheckPassword { username, password } => {
            handle_check_password(db, &username, &password)
        }
        AdminRequest::CreateAccount { username, password } => {
            handle_create_account(db, &username, &password)
        }
        AdminRequest::KickAccount { username, reason } => {
            match persistence_accounts::find_account_by_username(&db.db, &username) {
                Some(account_id) => {
                    let message = kick_message(reason);
                    let disconnected = queue_account_disconnect(
                        commands,
                        authenticated_query,
                        account_id,
                        &message,
                    );
                    AdminResponse::AccountKicked {
                        username,
                        disconnected,
                    }
                }
                None => AdminResponse::Error(format!("account '{username}' not found")),
            }
        }
        AdminRequest::BanAccount { username, reason } => {
            let reason = ban_reason(reason);
            match persistence_accounts::ban_account(&db.db, &username, &reason) {
                Ok(account_id) => {
                    let disconnected = queue_account_disconnect(
                        commands,
                        authenticated_query,
                        account_id,
                        &format!("Account banned: {reason}"),
                    );
                    AdminResponse::AccountBanned {
                        username,
                        disconnected,
                        reason,
                    }
                }
                Err(error) => AdminResponse::Error(error),
            }
        }
        AdminRequest::UnbanAccount { username } => {
            match persistence_accounts::unban_account(&db.db, &username) {
                Ok(_) => AdminResponse::AccountUnbanned { username },
                Err(error) => AdminResponse::Error(error),
            }
        }
        AdminRequest::ApproveRegistration { username } => {
            handle_approve_registration(db, &username)
        }
        AdminRequest::CreateGuild {
            character_name,
            guild_id,
            guild_name,
        } => handle_create_guild(db, &character_name, guild_id, &guild_name),
        AdminRequest::SetGuildRank {
            character_name,
            rank,
        } => handle_set_guild_rank(db, &character_name, rank),
        AdminRequest::SetGuildBankTabRights {
            guild_id,
            rank,
            tab,
            view,
            deposit,
            withdrawals_per_day,
        } => handle_set_guild_bank_rights(db, guild_id, rank, |rank_data| {
            let rights = rank_data
                .tabs
                .get_mut(usize::from(tab))
                .ok_or_else(|| format!("guild bank tab {tab} does not exist"))?;
            *rights = crate::persistence::banks::GuildTabRights {
                view,
                deposit,
                withdrawals_per_day,
            };
            Ok(())
        }),
        AdminRequest::SetGuildBankGoldRights {
            guild_id,
            rank,
            withdraw_gold,
            gold_per_day,
        } => handle_set_guild_bank_rights(db, guild_id, rank, |rank_data| {
            rank_data.withdraw_gold = withdraw_gold;
            rank_data.gold_per_day = gold_per_day;
            Ok(())
        }),
        AdminRequest::SetLevel {
            character_name,
            level,
        } => handle_set_level(db, commands, &character_name, level),
        AdminRequest::SetCredentials {
            token,
            username,
            password,
        } => handle_set_credentials(db, &token, &username, &password),
        AdminRequest::SetPosition {
            character_name,
            x,
            y,
            z,
        } => handle_set_position(db, players, interest_dirty, &character_name, x, y, z),
        AdminRequest::SetGold {
            character_name,
            copper,
        } => handle_set_gold(db, commands, &character_name, copper),
        AdminRequest::SetDurability {
            character_name,
            percent,
        } => handle_set_durability(db, commands, &character_name, percent),
        AdminRequest::SetAppearance {
            character_name,
            appearance,
        } => handle_set_appearance(
            db,
            authenticated_query,
            roster_update_senders,
            &character_name,
            appearance,
        ),
        AdminRequest::GrantEquipment {
            character_name,
            items,
        } => handle_grant_equipment(
            db,
            commands,
            equipment_query,
            authenticated_query,
            roster_update_senders,
            &character_name,
            &items,
        ),
        AdminRequest::ListEquipment { character_name } => {
            handle_list_equipment(db, &character_name)
        }
        AdminRequest::SetEquipmentAppearance {
            character_name,
            slot,
            display_info_id,
        } => handle_set_equipment_appearance(
            db,
            commands,
            equipment_query,
            authenticated_query,
            roster_update_senders,
            EquipmentAppearanceRequest {
                name: &character_name,
                slot_name: &slot,
                display_info_id,
            },
        ),
        AdminRequest::ClearEquipmentAppearance {
            character_name,
            slot,
        } => handle_clear_equipment_appearance(
            db,
            commands,
            equipment_query,
            authenticated_query,
            roster_update_senders,
            &character_name,
            &slot,
        ),
        // Handled in early-dispatch functions before this match
        AdminRequest::TicketList
        | AdminRequest::TicketClaim { .. }
        | AdminRequest::TicketClaimNext { .. }
        | AdminRequest::TicketRespond { .. }
        | AdminRequest::TicketResolve { .. }
        | AdminRequest::TicketEscalate { .. }
        | AdminRequest::TicketHistory { .. }
        | AdminRequest::GetMotd
        | AdminRequest::SetMotd { .. }
        | AdminRequest::ClearMotd
        | AdminRequest::BroadcastList
        | AdminRequest::BroadcastAdd { .. }
        | AdminRequest::BroadcastRemove { .. }
        | AdminRequest::BroadcastSetInterval { .. }
        | AdminRequest::BroadcastEnable
        | AdminRequest::BroadcastDisable => unreachable!(),
    };
    let _ = cmd.respond.send(response);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    use crate::persistence_accounts;

    /// Linen Cloth (2589): `content_item` name and a 1000 stack (world.db, Retail 12.1).
    fn linen_data() -> (
        crate::item_templates::ItemTemplates,
        crate::merchant::VendorCatalog,
    ) {
        let templates = crate::item_templates::ItemTemplates(
            [(
                2589,
                crate::item_templates::ItemTemplate {
                    max_stack: 1000,
                    allowable_class: -1,
                    allowable_race: u64::MAX,
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
        );
        let mut catalog = crate::merchant::VendorCatalog::default();
        catalog.goods.insert(
            2589,
            crate::merchant::VendorGoods {
                name: "Linen Cloth".into(),
                quality: 1,
                buy_price: 0,
                sell_price: 13,
                buy_count: 1,
                max_count: 0,
            },
        );
        (templates, catalog)
    }

    #[test]
    fn grant_item_puts_the_stack_in_the_bags_and_queues_a_delta() {
        let db = persistence::GameDatabase {
            db: persistence::temp_database(),
        };
        let (character_id, _) =
            persistence::create_character(&db.db, 1, "Stitchwell", 1, 1).expect("character");
        let (templates, catalog) = linen_data();
        let mut updates = crate::gold::GoldUpdateQueue::default();

        let response = handle_grant_item(
            &db,
            (&templates, &catalog),
            &mut updates,
            "Stitchwell",
            (2589, 4),
        );

        assert!(matches!(
            response,
            AdminResponse::ItemGranted { ref name, item_id: 2589, count: 4, .. } if name == "Linen Cloth"
        ));
        let bags = persistence::load_inventory(&db.db, character_id);
        assert_eq!(
            bags.items
                .iter()
                .map(|item| (item.item_id, item.stack_count, item.name.as_str()))
                .collect::<Vec<_>>(),
            vec![(2589, 4, "Linen Cloth")]
        );
        assert!(updates.changed_inventories().contains(&character_id));
    }

    #[test]
    fn grant_item_refuses_unknown_items_and_characters() {
        let db = persistence::GameDatabase {
            db: persistence::temp_database(),
        };
        persistence::create_character(&db.db, 1, "Stitchwell", 1, 1).expect("character");
        let (templates, catalog) = linen_data();
        let mut updates = crate::gold::GoldUpdateQueue::default();
        let data = (&templates, &catalog);
        assert!(matches!(
            handle_grant_item(&db, data, &mut updates, "Stitchwell", (999_999, 1)),
            AdminResponse::Error(_)
        ));
        assert!(matches!(
            handle_grant_item(&db, data, &mut updates, "Nobody", (2589, 1)),
            AdminResponse::Error(_)
        ));
        assert!(updates.changed_inventories().is_empty());
    }

    #[test]
    fn handle_set_position_updates_persisted_character_coords() {
        let db = persistence::temp_database();
        let (character_id, _) =
            persistence::create_character(&db, 1, "Mover", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };
        let mut app = App::new();
        let live_entity = app
            .world_mut()
            .spawn((
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                persistence::ClientId(character_id),
            ))
            .id();
        let mut system_state =
            SystemState::<Query<(Entity, &mut Position, &persistence::ClientId)>>::new(
                app.world_mut(),
            );
        let mut query = system_state
            .get_mut(app.world_mut())
            .expect("admin position query should be valid");

        let response =
            handle_set_position(&game_db, &mut query, None, "Mover", -8920.5, -188.25, 97.75);

        assert_eq!(
            response,
            AdminResponse::PositionSet {
                character_name: "Mover".to_string(),
                x: -8920.5,
                y: -188.25,
                z: 97.75,
            }
        );
        system_state.apply(app.world_mut());
        let saved = persistence::load_character(&game_db.db, character_id).expect("saved");
        assert_eq!((saved.x, saved.y, saved.z), (-8920.5, -188.25, 97.75));
        let pos = app.world().entity(live_entity).get::<Position>().unwrap();
        let (bx, by, bz) = crate::terrain_height::wow_to_bevy(-8920.5, -188.25, 97.75);
        assert_eq!((pos.x, pos.y, pos.z), (bx, by, bz));
    }

    #[test]
    fn handle_check_account_reports_existing_user() {
        let db = persistence::temp_database();
        let (account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("account");
        let game_db = persistence::GameDatabase { db };

        let response = handle_check_account(&game_db, "admin");

        assert!(matches!(
            response,
            AdminResponse::AccountChecked {
                username,
                exists: true,
                account_id: Some(id),
                token_count: 1,
                pending_registration: false,
                banned: false,
                ban_reason: None,
            } if username == "admin" && id == account_id
        ));
    }

    #[test]
    fn handle_list_accounts_returns_all_accounts() {
        let db = persistence::temp_database();
        persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        persistence_accounts::register_account(&db, "alice", "secret").expect("alice");
        let game_db = persistence::GameDatabase { db };

        let response = handle_list_accounts(&game_db);

        assert!(matches!(
            response,
            AdminResponse::AccountsListed { accounts }
                if accounts.len() == 2
                    && accounts.iter().any(|account| account.username == "admin")
                    && accounts.iter().any(|account| account.username == "alice")
        ));
    }

    #[test]
    fn handle_check_password_reports_validity() {
        let db = persistence::temp_database();
        persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        let game_db = persistence::GameDatabase { db };

        assert_eq!(
            handle_check_password(&game_db, "admin", "admin"),
            AdminResponse::PasswordChecked {
                username: "admin".to_string(),
                valid: true,
            }
        );
        assert_eq!(
            handle_check_password(&game_db, "admin", "wrong"),
            AdminResponse::PasswordChecked {
                username: "admin".to_string(),
                valid: false,
            }
        );
    }

    #[test]
    fn handle_list_characters_returns_account_characters() {
        let db = persistence::temp_database();
        let (account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        persistence::create_character(&db, account_id, "Theron", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };

        let response = handle_list_characters(&game_db, Some("admin"));

        assert!(matches!(
            response,
            AdminResponse::CharactersListed { characters }
                if characters.len() == 1
                    && characters[0].username == "admin"
                    && characters[0].account_id == account_id
                    && characters[0].name == "Theron"
        ));
    }

    #[test]
    fn handle_find_character_reports_owner() {
        let db = persistence::temp_database();
        let (account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        let (character_id, _data) =
            persistence::create_character(&db, account_id, "Theron", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };

        assert_eq!(
            handle_find_character(&game_db, "Theron"),
            AdminResponse::CharacterFound {
                character: Some(CharacterInfo {
                    username: "admin".to_string(),
                    account_id,
                    character_id,
                    name: "Theron".to_string(),
                    level: 1,
                    race: 1,
                    class: 1,
                })
            }
        );
    }

    #[test]
    fn handle_move_characters_moves_ownership_between_accounts() {
        let db = persistence::temp_database();
        let (from_account_id, _token) =
            persistence_accounts::register_account(&db, "alice", "secret").expect("alice");
        let (to_account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        persistence::create_character(&db, from_account_id, "Theron", 1, 1).expect("character");
        persistence::create_character(&db, from_account_id, "Elara", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };

        assert_eq!(
            handle_move_characters(&game_db, "alice", "admin"),
            AdminResponse::CharactersMoved {
                from_username: "alice".to_string(),
                to_username: "admin".to_string(),
                moved_count: 2,
            }
        );
        assert!(persistence::list_characters_for_account(&game_db.db, from_account_id).is_empty());
        let moved = persistence::list_characters_for_account(&game_db.db, to_account_id);
        assert_eq!(moved.len(), 2);
        assert!(moved.iter().any(|(_, data)| data.name == "Theron"));
        assert!(moved.iter().any(|(_, data)| data.name == "Elara"));
    }

    #[test]
    fn grant_equipment_delivers_owner_roster_update_without_notifying_other_accounts() {
        use lightyear::connection::host::HostClient;
        use lightyear::prelude::{Connected, MessageReceiver, PeerId, RemoteId};
        let db = persistence::temp_database();
        let (account_id, _) =
            persistence_accounts::register_account(&db, "roster-owner", "test").unwrap();
        let (id, _) = persistence::create_character(&db, account_id, "Theron", 1, 1).unwrap();
        let game_db = persistence::GameDatabase { db };
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::state::app::StatesPlugin,
            lightyear::prelude::server::ServerPlugins {
                tick_duration: std::time::Duration::from_millis(50),
            },
            shared::ProtocolPlugin,
        ));
        app.finish();
        app.cleanup();
        let mut connections = Vec::new();
        for peer in [account_id, account_id + 1] {
            connections.push(
                app.world_mut()
                    .spawn((
                        RemoteId(PeerId::Local(peer)),
                        Connected,
                        HostClient { buffer: vec![] },
                        crate::auth::Authenticated { account_id: peer },
                        MessageSender::<LoginResponse>::default(),
                        MessageReceiver::<LoginResponse>::default(),
                        MessageSender::<CharacterListUpdate>::default(),
                        MessageReceiver::<CharacterListUpdate>::default(),
                    ))
                    .id(),
            );
        }
        let mut state = bevy::ecs::system::SystemState::<(
            Commands,
            Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
            Query<(Entity, &crate::auth::Authenticated), With<MessageSender<LoginResponse>>>,
            Query<&mut MessageSender<CharacterListUpdate>>,
        )>::new(app.world_mut());
        {
            let (mut commands, mut query, authenticated, mut roster) =
                state.get_mut(app.world_mut()).unwrap();
            let definition = crate::equipment_admin_types::EquipmentGrantItem {
                item_id: 25,
                name: "Worn Shortsword".into(),
                quality: 1,
                required_level: 1,
                inventory_type: 21,
                vendor_sell_price: 1,
                allowable_class: -1,
                allowable_race: -1,
            };
            assert!(matches!(
                handle_grant_equipment(
                    &game_db,
                    &mut commands,
                    &mut query,
                    &authenticated,
                    &mut roster,
                    "Theron",
                    &[definition]
                ),
                AdminResponse::EquipmentGranted { .. }
            ));
        }
        state.apply(app.world_mut());
        app.update();
        let updates: Vec<_> = app
            .world_mut()
            .get_mut::<MessageReceiver<CharacterListUpdate>>(connections[0])
            .unwrap()
            .receive()
            .collect();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].character.character_id, id);
        assert_eq!(
            updates[0].character.equipment_appearance.entries[0].item_id,
            Some(25)
        );
        assert!(
            app.world_mut()
                .get_mut::<MessageReceiver<CharacterListUpdate>>(connections[1])
                .unwrap()
                .receive()
                .next()
                .is_none()
        );
    }

    #[test]
    fn grant_equipment_publishes_live_appearance_and_rejection_has_no_side_effects() {
        let db = persistence::temp_database();
        let (account_id, _) =
            persistence_accounts::register_account(&db, "gear-admin", "test").unwrap();
        let (id, data) = persistence::create_character(&db, account_id, "Theron", 1, 1).unwrap();
        let game_db = persistence::GameDatabase { db };
        let mut world = World::new();
        let player = world
            .spawn((data.equipment_appearance.clone(), persistence::ClientId(id)))
            .id();
        let other = world
            .spawn((
                EquipmentAppearance::default(),
                persistence::ClientId(id + 100),
            ))
            .id();
        let mut state = bevy::ecs::system::SystemState::<(
            Commands,
            Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
            Query<(Entity, &crate::auth::Authenticated), With<MessageSender<LoginResponse>>>,
            Query<&mut MessageSender<CharacterListUpdate>>,
        )>::new(&mut world);
        let definition = crate::equipment_admin_types::EquipmentGrantItem {
            item_id: 25,
            name: "Worn Shortsword".into(),
            quality: 1,
            required_level: 1,
            inventory_type: 13,
            vendor_sell_price: 1,
            allowable_class: 1,
            allowable_race: 1,
        };
        {
            let (mut commands, mut query, authenticated, mut roster) =
                state.get_mut(&mut world).unwrap();
            let result = handle_grant_equipment(
                &game_db,
                &mut commands,
                &mut query,
                &authenticated,
                &mut roster,
                "Theron",
                &[definition.clone()],
            );
            assert!(matches!(result, AdminResponse::EquipmentGranted { .. }));
        }
        state.apply(&mut world);
        let expected = persistence::load_character(&game_db.db, id)
            .unwrap()
            .equipment_appearance;
        assert_eq!(world.get::<EquipmentAppearance>(player).unwrap(), &expected);
        assert!(
            world
                .get::<EquipmentAppearance>(other)
                .unwrap()
                .entries
                .is_empty()
        );
        let tick = world
            .entity(player)
            .get_ref::<EquipmentAppearance>()
            .unwrap()
            .last_changed();
        world.increment_change_tick();
        {
            let (mut commands, mut query, authenticated, mut roster) =
                state.get_mut(&mut world).unwrap();
            let result = handle_grant_equipment(
                &game_db,
                &mut commands,
                &mut query,
                &authenticated,
                &mut roster,
                "Theron",
                &[definition],
            );
            assert!(matches!(result, AdminResponse::Error(_)));
        }
        state.apply(&mut world);
        assert_eq!(
            world
                .entity(player)
                .get_ref::<EquipmentAppearance>()
                .unwrap()
                .last_changed(),
            tick
        );
        assert_eq!(world.get::<EquipmentAppearance>(player).unwrap(), &expected);
    }

    #[test]
    fn handle_set_equipment_appearance_updates_character_data() {
        let db = persistence::temp_database();
        let (account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        persistence::create_character(&db, account_id, "Theron", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };
        let mut world = World::new();
        let mut state = bevy::ecs::system::SystemState::<(
            Commands,
            Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
            Query<(Entity, &crate::auth::Authenticated), With<MessageSender<LoginResponse>>>,
            Query<&mut MessageSender<CharacterListUpdate>>,
        )>::new(&mut world);
        let (mut commands, mut query, authenticated_query, mut roster_update_senders) = state
            .get_mut(&mut world)
            .expect("admin equipment query should be valid");

        let response = handle_set_equipment_appearance(
            &game_db,
            &mut commands,
            &mut query,
            &authenticated_query,
            &mut roster_update_senders,
            EquipmentAppearanceRequest {
                name: "Theron",
                slot_name: "chest",
                display_info_id: 777,
            },
        );

        assert_eq!(
            response,
            AdminResponse::EquipmentAppearanceSet {
                character_name: "Theron".to_string(),
                slot: "Chest".to_string(),
                display_info_id: 777,
            }
        );
        let char_id = persistence::find_character_id_by_name(&game_db.db, "Theron").unwrap();
        let data = persistence::load_character(&game_db.db, char_id).unwrap();
        assert_eq!(data.equipment_appearance.entries.len(), 1);
        assert_eq!(
            data.equipment_appearance.entries[0].slot,
            shared::components::EquipmentVisualSlot::Chest
        );
        assert_eq!(
            data.equipment_appearance.entries[0].display_info_id,
            Some(777)
        );
    }

    #[test]
    fn equal_equipment_appearance_request_does_not_dirty_live_component() {
        let db = persistence::temp_database();
        let (account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        let (character_id, _data) =
            persistence::create_character(&db, account_id, "Theron", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };
        let initial = EquipmentAppearance {
            entries: vec![shared::components::EquippedAppearanceEntry {
                slot: shared::components::EquipmentVisualSlot::Chest,
                item_id: Some(100),
                display_info_id: Some(777),
                inventory_type: 5,
                hidden: false,
            }],
        };
        let mut world = World::new();
        let entity = world
            .spawn((initial.clone(), persistence::ClientId(character_id)))
            .id();
        world.increment_change_tick();
        let tick_before = world
            .entity(entity)
            .get_ref::<EquipmentAppearance>()
            .expect("live equipment appearance")
            .last_changed();
        let mut state = bevy::ecs::system::SystemState::<(
            Commands,
            Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
            Query<(Entity, &crate::auth::Authenticated), With<MessageSender<LoginResponse>>>,
            Query<&mut MessageSender<CharacterListUpdate>>,
        )>::new(&mut world);

        {
            let (mut commands, mut query, authenticated_query, mut roster_update_senders) = state
                .get_mut(&mut world)
                .expect("admin equipment query should be valid");
            let response = handle_set_equipment_appearance(
                &game_db,
                &mut commands,
                &mut query,
                &authenticated_query,
                &mut roster_update_senders,
                EquipmentAppearanceRequest {
                    name: "Theron",
                    slot_name: "chest",
                    display_info_id: 777,
                },
            );
            assert_eq!(
                response,
                AdminResponse::EquipmentAppearanceSet {
                    character_name: "Theron".to_string(),
                    slot: "Chest".to_string(),
                    display_info_id: 777,
                }
            );
        }

        assert_eq!(
            world
                .entity(entity)
                .get::<EquipmentAppearance>()
                .expect("live equipment appearance"),
            &initial
        );
        assert_eq!(
            world
                .entity(entity)
                .get_ref::<EquipmentAppearance>()
                .expect("live equipment appearance")
                .last_changed(),
            tick_before,
            "an equal admin request must not dirty EquipmentAppearance"
        );
    }

    #[test]
    fn equal_clear_equipment_appearance_request_does_not_dirty_live_component() {
        let db = persistence::temp_database();
        let (account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        let (character_id, _data) =
            persistence::create_character(&db, account_id, "Theron", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };
        let initial = EquipmentAppearance {
            entries: vec![shared::components::EquippedAppearanceEntry {
                slot: shared::components::EquipmentVisualSlot::Chest,
                item_id: None,
                display_info_id: None,
                inventory_type: 0,
                hidden: true,
            }],
        };
        let mut world = World::new();
        let entity = world
            .spawn((initial.clone(), persistence::ClientId(character_id)))
            .id();
        world.increment_change_tick();
        let tick_before = world
            .entity(entity)
            .get_ref::<EquipmentAppearance>()
            .expect("live equipment appearance")
            .last_changed();
        let mut state = bevy::ecs::system::SystemState::<(
            Commands,
            Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
            Query<(Entity, &crate::auth::Authenticated), With<MessageSender<LoginResponse>>>,
            Query<&mut MessageSender<CharacterListUpdate>>,
        )>::new(&mut world);

        {
            let (mut commands, mut query, authenticated_query, mut roster_update_senders) = state
                .get_mut(&mut world)
                .expect("admin equipment query should be valid");
            let response = handle_clear_equipment_appearance(
                &game_db,
                &mut commands,
                &mut query,
                &authenticated_query,
                &mut roster_update_senders,
                "Theron",
                "chest",
            );
            assert_eq!(
                response,
                AdminResponse::EquipmentAppearanceCleared {
                    character_name: "Theron".to_string(),
                    slot: "Chest".to_string(),
                }
            );
        }

        assert_eq!(
            world
                .entity(entity)
                .get::<EquipmentAppearance>()
                .expect("live equipment appearance"),
            &initial
        );
        assert_eq!(
            world
                .entity(entity)
                .get_ref::<EquipmentAppearance>()
                .expect("live equipment appearance")
                .last_changed(),
            tick_before,
            "an equal clear request must not dirty EquipmentAppearance"
        );
    }

    #[test]
    fn handle_clear_equipment_appearance_marks_slot_hidden() {
        let db = persistence::temp_database();
        let (account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        persistence::create_character(&db, account_id, "Theron", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };
        let mut world = World::new();
        let mut state = bevy::ecs::system::SystemState::<(
            Commands,
            Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
            Query<(Entity, &crate::auth::Authenticated), With<MessageSender<LoginResponse>>>,
            Query<&mut MessageSender<CharacterListUpdate>>,
        )>::new(&mut world);
        let (mut commands, mut query, authenticated_query, mut roster_update_senders) = state
            .get_mut(&mut world)
            .expect("admin equipment query should be valid");

        let response = handle_clear_equipment_appearance(
            &game_db,
            &mut commands,
            &mut query,
            &authenticated_query,
            &mut roster_update_senders,
            "Theron",
            "feet",
        );

        assert_eq!(
            response,
            AdminResponse::EquipmentAppearanceCleared {
                character_name: "Theron".to_string(),
                slot: "Feet".to_string(),
            }
        );
        let char_id = persistence::find_character_id_by_name(&game_db.db, "Theron").unwrap();
        let data = persistence::load_character(&game_db.db, char_id).unwrap();
        assert_eq!(data.equipment_appearance.entries.len(), 1);
        assert_eq!(
            data.equipment_appearance.entries[0].slot,
            shared::components::EquipmentVisualSlot::Feet
        );
        assert_eq!(data.equipment_appearance.entries[0].display_info_id, None);
        assert!(data.equipment_appearance.entries[0].hidden);
    }
}
