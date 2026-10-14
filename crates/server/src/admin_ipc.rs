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

const DEFAULT_SOCKET_PATH: &str = "/tmp/game-server-admin.sock";

/// Admin socket; `GAME_SERVER_ADMIN_SOCKET` gives a second server (own
/// `GAME_SERVER_PORT`) its own socket instead of taking over the default one.
pub(super) fn socket_path() -> std::path::PathBuf {
    std::env::var_os("GAME_SERVER_ADMIN_SOCKET")
        .map_or_else(|| DEFAULT_SOCKET_PATH.into(), Into::into)
}

pub struct AdminIpcPlugin;

impl Plugin for AdminIpcPlugin {
    fn build(&self, app: &mut App) {
        let (receiver, guard) = init().unwrap_or_else(|error| panic!("admin ipc: {error}"));

        app.insert_non_send(receiver)
            .insert_non_send(guard)
            .add_systems(Update, poll_admin_ipc);
    }
}

/// Resources needed by the admin IPC early-dispatch handlers.
#[derive(SystemParam)]
struct AdminResources<'w, 's> {
    time: Res<'w, Time>,
    combat_log: Res<'w, crate::combat::CombatLog>,
    ticket_mgr: ResMut<'w, shared::ticket::TicketManager>,
    motd: ResMut<'w, shared::motd::Motd>,
    autobroadcast: ResMut<'w, shared::motd::Autobroadcast>,
    taxi: Res<'w, crate::taxi::TaxiNetwork>,
    interest_dirty: Option<Res<'w, crate::interest::InterestDirtyQueue>>,
    replication_dirty: Option<Res<'w, lightyear::prelude::ReplicationDirtyQueue>>,
    item_templates: Option<Res<'w, crate::item_templates::ItemTemplates>>,
    vendor_catalog: Option<Res<'w, crate::merchant::VendorCatalog>>,
    create_info: Option<Res<'w, crate::player_create_info::PlayerCreateInfos>>,
    inventory_updates: ResMut<'w, crate::gold::GoldUpdateQueue>,
    changed_mailboxes: ResMut<'w, crate::mail::ChangedMailboxes>,
    quests: Option<Res<'w, crate::quest_data::QuestDb>>,
    quest_dirty: Option<ResMut<'w, crate::quest::QuestDirty>>,
    instances: crate::maps::difficulty::AdminInstances<'w, 's>,
    economy: Option<Res<'w, crate::economy::Economy>>,
    economy_config: Option<Res<'w, crate::economy::Config>>,
    refund_window: Option<ResMut<'w, crate::trading_post::RefundWindow>>,
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
            &'static mut shared::components::MovementControl,
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
        dispatch(
            cmd,
            &db,
            &mut queries,
            res.interest_dirty.as_deref(),
            res.replication_dirty.as_deref(),
        );
    }
}

/// Handle commands that only need resources (no ECS queries).
fn dispatch_resource_commands(
    request: &AdminRequest,
    db: &persistence::GameDatabase,
    res: &mut AdminResources,
) -> Option<AdminResponse> {
    if let AdminRequest::EconomyStatus = request {
        return Some(crate::economy::admin_status(
            res.economy.as_deref(),
            res.economy_config.as_deref(),
        ));
    }
    if let Some(response) = dispatch_trading_post(request, db, res) {
        return Some(response);
    }
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
    if let AdminRequest::GrantKeystone {
        character_name,
        challenge_id,
        level,
    } = request
    {
        let Some(character) = persistence::find_character_id_by_name(&db.db, character_name) else {
            return Some(AdminResponse::Error("Character not found".into()));
        };
        let (Some(templates), Some(catalog)) = (&res.item_templates, &res.vendor_catalog) else {
            return Some(AdminResponse::Error("Item data not loaded".into()));
        };
        return Some(
            match crate::mythic_plus::issue_keystone(
                &db.db,
                templates,
                catalog,
                character,
                *challenge_id,
                *level,
            ) {
                Ok(()) => {
                    res.inventory_updates.record_items(character);
                    AdminResponse::ItemGranted {
                        character_name: character_name.clone(),
                        item_id: crate::mythic_plus::KEYSTONE_ITEM,
                        name: "Mythic Keystone".into(),
                        count: 1,
                    }
                }
                Err(error) => AdminResponse::Error(error),
            },
        );
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
    if let AdminRequest::CreateCharacter {
        username,
        name,
        race,
        class,
    } = request
    {
        let (Some(create_info), Some(templates), Some(catalog)) =
            (&res.create_info, &res.item_templates, &res.vendor_catalog)
        else {
            return Some(AdminResponse::Error("player create info not loaded".into()));
        };
        let kit = crate::player_create_info::NewCharacterKit {
            create_info,
            templates,
            catalog,
        };
        return Some(handle_create_character(
            db, &kit, username, name, *race, *class,
        ));
    }
    if let AdminRequest::QuestComplete {
        character_name,
        quest_id,
    } = request
    {
        let (Some(quests), Some(dirty)) = (&res.quests, &mut res.quest_dirty) else {
            return Some(AdminResponse::Error("quest data not loaded".into()));
        };
        return Some(handle_quest_complete(
            db,
            quests,
            dirty,
            character_name,
            *quest_id,
        ));
    }
    if let AdminRequest::MailDeliverNow { character_name } = request {
        return Some(handle_mail_deliver_now(
            db,
            &mut res.changed_mailboxes,
            character_name,
        ));
    }
    if let AdminRequest::TaxiLearn {
        character_name,
        node,
    } = request
    {
        return Some(handle_taxi_learn(db, &res.taxi, character_name, *node));
    }
    res.instances.dispatch(request, db)
}

/// Private-server test setup for Trading Post live proof; no player-reachable path.
fn dispatch_trading_post(
    request: &AdminRequest,
    db: &persistence::GameDatabase,
    res: &mut AdminResources,
) -> Option<AdminResponse> {
    match request {
        AdminRequest::TradingPostGrantTender { username, amount } => Some(
            match crate::trading_post::admin_grant_tender(&db.db, username, *amount) {
                Ok(tender) => AdminResponse::TradingPostTenderGranted {
                    username: username.clone(),
                    tender,
                },
                Err(error) => AdminResponse::Error(error),
            },
        ),
        AdminRequest::RenownGrant {
            character_name,
            major_faction,
            amount,
        } => Some(
            match crate::renown::admin_grant(&db.db, character_name, *major_faction, *amount) {
                Ok(progress) => AdminResponse::RenownGranted {
                    character_name: character_name.clone(),
                    major_faction: *major_faction,
                    level: progress.level,
                    standing: progress.standing,
                },
                Err(error) => AdminResponse::Error(error),
            },
        ),
        AdminRequest::TradingPostRefundWindow { seconds } => Some(match &mut res.refund_window {
            Some(window) => {
                window.0 = *seconds;
                AdminResponse::TradingPostRefundWindowSet { seconds: *seconds }
            }
            None => AdminResponse::Error("Trading Post not loaded".into()),
        }),
        _ => None,
    }
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

/// `quest-complete`: the quest's counted objectives done; an online owner gets the
/// `QuestLogUpdate` and quest giver markers through `QuestDirty`.
fn handle_quest_complete(
    db: &persistence::GameDatabase,
    quests: &crate::quest_data::QuestDb,
    dirty: &mut crate::quest::QuestDirty,
    name: &str,
    quest_id: u32,
) -> AdminResponse {
    let Some(character_id) = persistence::find_character_id_by_name(&db.db, name) else {
        return AdminResponse::Error(format!("character '{name}' not found"));
    };
    let mut log = persistence::quest_store::load_quest_log(&db.db, character_id);
    if let Err(error) = crate::quest_rules::complete_counted_objectives(quests, &mut log, quest_id)
    {
        return AdminResponse::Error(format!("{name}: {error}"));
    }
    persistence::quest_store::save_quest_log(&db.db, character_id, &log);
    dirty.0.insert(character_id);
    AdminResponse::QuestCompleted {
        character_name: name.to_owned(),
        quest_id,
    }
}

/// `mail-deliver-now`: mail in transit to the character arrives now; an open
/// mailbox gets the inbox again.
fn handle_mail_deliver_now(
    db: &persistence::GameDatabase,
    changed: &mut crate::mail::ChangedMailboxes,
    name: &str,
) -> AdminResponse {
    let Some(character_id) = persistence::find_character_id_by_name(&db.db, name) else {
        return AdminResponse::Error(format!("character '{name}' not found"));
    };
    let count = crate::mail::deliver_pending_mail(&db.db, character_id, crate::mail::unix_now());
    changed.0.insert(character_id);
    AdminResponse::MailDelivered {
        character_name: name.to_owned(),
        count,
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
    replication_dirty: Option<&lightyear::prelude::ReplicationDirtyQueue>,
) {
    let commands = &mut queries.commands;
    let players = &mut queries.players;
    let equipment_query = &mut queries.equipment;
    let authenticated_query = &queries.authenticated;
    let roster_update_senders = &mut queries.roster_senders;

    let response = match cmd.request {
        AdminRequest::Ping => AdminResponse::Pong,
        AdminRequest::CombatLog { .. }
        | AdminRequest::GrantItem { .. }
        | AdminRequest::GrantKeystone { .. }
        | AdminRequest::CreateCharacter { .. }
        | AdminRequest::QuestComplete { .. }
        | AdminRequest::MailDeliverNow { .. } => {
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
        AdminRequest::SetGuildBankRepairRight {
            guild_id,
            rank,
            withdraw_repair,
        } => handle_set_guild_bank_rights(db, guild_id, rank, |rank_data| {
            rank_data.withdraw_repair = withdraw_repair;
            Ok(())
        }),
        AdminRequest::SetLevel {
            character_name,
            level,
        } => handle_set_level(db, commands, &character_name, level),
        AdminRequest::SetRested {
            character_name,
            amount,
        } => handle_set_rested(db, commands, &character_name, amount),
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
        } => handle_set_position(
            db,
            players,
            interest_dirty,
            replication_dirty,
            &character_name,
            x,
            y,
            z,
        ),
        AdminRequest::Teleport {
            character_name,
            map_id,
            x,
            y,
            z,
        } => handle_teleport(
            db,
            commands,
            players,
            &character_name,
            map_id,
            Vec3::new(x, y, z),
        ),
        AdminRequest::Kill { character_name } => {
            handle_kill(db, commands, players, &character_name)
        }
        AdminRequest::LearnSpell {
            character_name,
            spell,
        } => handle_learn_spell(db, commands, players, &character_name, spell),
        AdminRequest::FlightStyle {
            character_name,
            skyriding,
        } => handle_flight_style(db, commands, players, &character_name, skyriding),
        AdminRequest::Damage {
            character_name,
            amount,
            target,
        } => handle_damage(db, commands, players, &character_name, amount, target),
        AdminRequest::Revive { character_name } => {
            handle_revive(db, commands, players, &character_name)
        }
        AdminRequest::EconomyStatus => AdminResponse::Error("economy not initialized".into()),
        // Answered by AdminInstances unless the map data is not loaded.
        AdminRequest::ClockAdvance { .. }
        | AdminRequest::InstanceLocks { .. }
        | AdminRequest::InstanceUnbind { .. }
        | AdminRequest::NpcInfo { .. }
        | AdminRequest::GameObjectInfo { .. } => {
            AdminResponse::Error("instance data not loaded".into())
        }
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
        AdminRequest::TradingPostGrantTender { .. }
        | AdminRequest::RenownGrant { .. }
        | AdminRequest::TradingPostRefundWindow { .. }
        | AdminRequest::TicketList
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
        | AdminRequest::BroadcastDisable
        | AdminRequest::TaxiLearn { .. } => unreachable!(),
    };
    let _ = cmd.respond.send(response);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    #[test]
    fn admin_dispatcher_initializes_object_lookup_and_player_movement_together() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<crate::combat::CombatLog>()
            .init_resource::<shared::ticket::TicketManager>()
            .init_resource::<shared::motd::Motd>()
            .init_resource::<shared::motd::Autobroadcast>()
            .init_resource::<crate::taxi::TaxiNetwork>()
            .init_resource::<crate::gold::GoldUpdateQueue>()
            .init_resource::<crate::mail::ChangedMailboxes>();
        let (_sender, receiver) = mpsc::channel::<AdminCommand>();
        app.insert_non_send(receiver)
            .add_systems(Update, poll_admin_ipc);
        app.update();
    }

    use crate::persistence_accounts;

    /// Linen Cloth (2589): `content_item` name and a 1000 stack (world.db, Retail 12.1).
    fn linen_data() -> (
        crate::item_templates::ItemTemplates,
        crate::merchant::VendorCatalog,
    ) {
        let templates = crate::item_templates::ItemTemplates {
            retail: [(
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
            forever: Default::default(),
            loot_need: Default::default(),
            retail_learning_spells: Default::default(),
        };
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
    fn create_character_adds_a_listed_character_to_the_named_account() {
        let db = persistence::GameDatabase::new(persistence::temp_database());
        persistence_accounts::register_account(&db.db, "fb_probe", "pw").expect("account");

        let response = handle_create_character(
            &db,
            &crate::player_create_info::OwnedKit::bare().kit(),
            "fb_probe",
            "Probeone",
            1,
            2,
        );

        let AdminResponse::CharacterCreated { character_id, .. } = response else {
            panic!("expected CharacterCreated, got {response:?}");
        };
        let AdminResponse::CharactersListed { characters } =
            handle_list_characters(&db, Some("fb_probe"))
        else {
            panic!("listing failed");
        };
        assert_eq!(
            characters
                .iter()
                .map(|c| (c.character_id, c.name.as_str(), c.race, c.class))
                .collect::<Vec<_>>(),
            vec![(character_id, "Probeone", 1, 2)]
        );
    }

    #[test]
    fn create_character_reports_unknown_account_and_invalid_names() {
        let db = persistence::GameDatabase::new(persistence::temp_database());
        assert!(matches!(
            handle_create_character(&db, &crate::player_create_info::OwnedKit::bare().kit(), "nobody", "Probeone", 1, 2),
            AdminResponse::Error(ref e) if e.contains("nobody")
        ));
        persistence_accounts::register_account(&db.db, "fb_probe", "pw").expect("account");
        assert!(matches!(
            handle_create_character(
                &db,
                &crate::player_create_info::OwnedKit::bare().kit(),
                "fb_probe",
                "x1",
                1,
                2
            ),
            AdminResponse::Error(_)
        ));
    }

    #[test]
    fn grant_item_puts_the_stack_in_the_bags_and_queues_a_delta() {
        let db = persistence::GameDatabase::new(persistence::temp_database());
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
    fn quest_complete_saves_the_finished_objectives_and_marks_the_log_dirty() {
        let Some(quests) = crate::quest_data::test_quest_db() else {
            return;
        };
        let db = persistence::GameDatabase::new(persistence::temp_database());
        let (character_id, _) =
            persistence::create_character(&db.db, 1, "Quester", 1, 1).expect("character");
        let mut dirty = crate::quest::QuestDirty::default();
        assert!(matches!(
            handle_quest_complete(&db, quests, &mut dirty, "Quester", 26391),
            AdminResponse::Error(ref e) if e.contains("not in the log")
        ));
        let mut log = persistence::quest_store::load_quest_log(&db.db, character_id);
        log.active.push(persistence::quest_store::ActiveQuestData {
            quest_id: 26391,
            counts: vec![3],
            ..Default::default()
        });
        persistence::quest_store::save_quest_log(&db.db, character_id, &log);

        let response = handle_quest_complete(&db, quests, &mut dirty, "Quester", 26391);

        assert_eq!(
            response,
            AdminResponse::QuestCompleted {
                character_name: "Quester".into(),
                quest_id: 26391
            }
        );
        let saved = persistence::quest_store::load_quest_log(&db.db, character_id);
        assert_eq!(saved.active(26391).unwrap().counts, [8]);
        assert!(dirty.0.contains(&character_id));
    }

    #[test]
    fn mail_deliver_now_delivers_mail_in_transit_and_refreshes_the_mailbox() {
        use crate::persistence::mail_store::{self, MailData, MailSender};
        let db = persistence::GameDatabase::new(persistence::temp_database());
        let (character_id, _) =
            persistence::create_character(&db.db, 1, "Tradeb", 1, 1).expect("character");
        let now = crate::mail::unix_now();
        mail_store::deliver_mail(
            &db.db,
            character_id,
            MailData {
                mail_id: 1,
                sender: MailSender::Player {
                    character_id: 99,
                    name: "Tradea".into(),
                },
                subject: "Linen".into(),
                body: String::new(),
                items: Vec::new(),
                money: 5,
                cod: 0,
                delivers_at: now + 3_600,
                expires_at: now + 3_600 + shared::mail::MAIL_EXPIRY_SECS,
                read: false,
                returned: false,
                cod_payment: false,
            },
        );
        let mut changed = crate::mail::ChangedMailboxes::default();

        let response = handle_mail_deliver_now(&db, &mut changed, "Tradeb");

        assert!(matches!(
            response,
            AdminResponse::MailDelivered { count: 1, .. }
        ));
        let mail = &mail_store::load_mailbox(&db.db, character_id)[0];
        assert!(mail.is_delivered(crate::mail::unix_now()));
        assert!(mail.expires_at >= now + shared::mail::MAIL_EXPIRY_SECS);
        assert!(changed.0.contains(&character_id));
        assert!(matches!(
            handle_mail_deliver_now(&db, &mut changed, "Nobody"),
            AdminResponse::Error(_)
        ));
    }

    #[test]
    fn grant_item_refuses_unknown_items_and_characters() {
        let db = persistence::GameDatabase::new(persistence::temp_database());
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
    fn taxi_learn_adds_a_flight_point_to_the_characters_known_nodes() {
        let path = crate::test_data::world_db_path();
        let conn = rusqlite::Connection::open(&path).unwrap();
        let network = crate::taxi::load_taxi_network(&conn).unwrap();
        let game_db = persistence::GameDatabase::new(persistence::temp_database());
        let (character_id, _) =
            persistence::create_character(&game_db.db, 1, "Scout", 1, 1).expect("create character");

        assert_eq!(
            handle_taxi_learn(&game_db, &network, "Scout", 4),
            AdminResponse::TaxiNodeLearned {
                character_name: "Scout".into(),
                node: 4,
                node_name: "Sentinel Hill, Westfall".into(),
            }
        );
        assert_eq!(
            persistence::taxi_store::load_known_taxi_nodes(&game_db.db, character_id),
            std::collections::BTreeSet::from([4])
        );
        assert_eq!(
            handle_taxi_learn(&game_db, &network, "Scout", 999_999),
            AdminResponse::Error("taxi node 999999 not found".into())
        );
    }

    #[test]
    fn damage_hits_the_online_characters_target_and_a_lethal_amount_kills_it() {
        let db = persistence::temp_database();
        let (character_id, _) =
            persistence::create_character(&db, 1, "Striker", 1, 1).expect("character");
        let game_db = persistence::GameDatabase::new(db);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<crate::death::PlayerDeathTransition>();
        app.update();
        let npc = app
            .world_mut()
            .spawn((
                shared::components::Npc {
                    template_id: 46254,
                    name: "Hogger".into(),
                },
                shared::components::Health {
                    current: 500.0,
                    max: 500.0,
                },
            ))
            .id();
        app.world_mut().spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            shared::components::Health {
                current: 100.0,
                max: 100.0,
            },
            persistence::ClientId(character_id),
            shared::components::MovementControl::default(),
            crate::networking::CombatTarget(Some(npc)),
        ));
        let damage = |app: &mut App, amount: f32| {
            let mut system_state = SystemState::<(
                Commands,
                Query<(
                    Entity,
                    &mut Position,
                    &persistence::ClientId,
                    &mut shared::components::MovementControl,
                )>,
            )>::new(app.world_mut());
            let (mut commands, players) = system_state.get_mut(app.world_mut()).unwrap();
            let response =
                handle_damage(&game_db, &mut commands, &players, "Striker", amount, None);
            system_state.apply(app.world_mut());
            response
        };
        assert_eq!(
            damage(&mut app, 120.0),
            AdminResponse::Damaged {
                character_name: "Striker".into(),
                amount: 120.0
            }
        );
        let health = |app: &App| {
            app.world()
                .get::<shared::components::Health>(npc)
                .unwrap()
                .current
        };
        assert_eq!(health(&app), 380.0);
        damage(&mut app, 1_000.0);
        assert_eq!(health(&app), 0.0);
        assert!(app.world().get::<crate::combat::DeathState>(npc).is_some());
    }

    #[test]
    fn revive_brings_a_dead_online_character_back_at_full_health() {
        let db = persistence::temp_database();
        let (character_id, _) =
            persistence::create_character(&db, 1, "Fallen", 1, 1).expect("character");
        let game_db = persistence::GameDatabase::new(db);
        let mut app = App::new();
        let player = app
            .world_mut()
            .spawn((
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                shared::components::Health {
                    current: 0.0,
                    max: 46_500.0,
                },
                crate::death::PlayerDeathState {
                    state: shared::death::DeathState::Dead,
                    ..Default::default()
                },
                persistence::ClientId(character_id),
                shared::components::MovementControl::default(),
            ))
            .id();
        let mut system_state = SystemState::<(
            Commands,
            Query<(
                Entity,
                &mut Position,
                &persistence::ClientId,
                &mut shared::components::MovementControl,
            )>,
        )>::new(app.world_mut());
        let (mut commands, players) = system_state.get_mut(app.world_mut()).unwrap();
        let response = handle_revive(&game_db, &mut commands, &players, "Fallen");
        system_state.apply(app.world_mut());
        assert_eq!(
            response,
            AdminResponse::Revived {
                character_name: "Fallen".into()
            }
        );
        let world = app.world();
        assert_eq!(
            world
                .get::<shared::components::Health>(player)
                .unwrap()
                .current,
            46_500.0
        );
        assert_eq!(
            world
                .get::<crate::death::PlayerDeathState>(player)
                .unwrap()
                .state,
            shared::death::DeathState::Alive
        );
    }

    /// `.die` -> `Unit::Kill`: the character dies as from a lethal hit, so its client gets
    /// the combat log's Death line (UNIT_DIED) its meter counts.
    #[test]
    fn killing_an_online_character_logs_its_death_and_starts_its_death_transition() {
        let Some(mut h) = crate::spell_cast::test_harness::harness(Vec::new(), &[], 1) else {
            return;
        };
        let db = persistence::temp_database();
        let (character_id, _) =
            persistence::create_character(&db, 1, "Doomed", 1, 1).expect("character");
        let game_db = persistence::GameDatabase::new(db);
        let player = h.caster;
        h.app.world_mut().entity_mut(player).insert((
            persistence::ClientId(character_id),
            shared::components::MovementControl::default(),
        ));
        let mut system_state = SystemState::<(
            Commands,
            Query<(
                Entity,
                &mut Position,
                &persistence::ClientId,
                &mut shared::components::MovementControl,
            )>,
        )>::new(h.app.world_mut());
        let (mut commands, players) = system_state.get_mut(h.app.world_mut()).unwrap();
        let response = handle_kill(&game_db, &mut commands, &players, "Doomed");
        system_state.apply(h.app.world_mut());
        h.advance(1);

        assert_eq!(
            response,
            AdminResponse::Killed {
                character_name: "Doomed".into()
            }
        );
        assert_eq!(h.health(player), 0.0);
        let deaths: Vec<_> = h
            .inbox
            .log
            .iter()
            .filter(|event| event.kind == shared::protocol::CombatLogKind::Death)
            .map(|event| (event.source, event.target))
            .collect();
        let unit = Some(player.to_bits());
        assert_eq!(deaths, vec![(unit, unit)]);
        let transitions = h
            .app
            .world()
            .resource::<Messages<crate::death::PlayerDeathTransition>>();
        assert_eq!(transitions.len(), 1);
    }

    #[test]
    fn teleporting_an_offline_character_saves_its_map_and_position() {
        let db = persistence::temp_database();
        let (character_id, _) =
            persistence::create_character(&db, 1, "Voyager", 1, 1).expect("character");
        let game_db = persistence::GameDatabase::new(db);
        let mut app = App::new();
        let mut system_state = SystemState::<(
            Commands,
            Query<(
                Entity,
                &mut Position,
                &persistence::ClientId,
                &mut shared::components::MovementControl,
            )>,
        )>::new(app.world_mut());
        let (mut commands, players) = system_state.get_mut(app.world_mut()).unwrap();

        // Ratchet, Kalimdor.
        let response = handle_teleport(
            &game_db,
            &mut commands,
            &players,
            "Voyager",
            1,
            Vec3::new(-956.0, -3754.0, 5.0),
        );

        assert_eq!(
            response,
            AdminResponse::Teleported {
                character_name: "Voyager".into(),
                map_id: 1,
                online: false,
            }
        );
        assert_eq!(
            crate::maps::persistence::load_character_map(&game_db.db, character_id),
            1
        );
        let saved = persistence::load_character(&game_db.db, character_id).unwrap();
        assert_eq!((saved.x, saved.y, saved.z), (-956.0, -3754.0, 5.0));
    }

    #[test]
    fn handle_set_position_updates_persisted_character_coords() {
        let db = persistence::temp_database();
        let (character_id, _) =
            persistence::create_character(&db, 1, "Mover", 1, 1).expect("character");
        let game_db = persistence::GameDatabase::new(db);
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
                shared::components::MovementControl::default(),
            ))
            .id();
        let mut system_state = SystemState::<
            Query<(
                Entity,
                &mut Position,
                &persistence::ClientId,
                &mut shared::components::MovementControl,
            )>,
        >::new(app.world_mut());
        let mut query = system_state
            .get_mut(app.world_mut())
            .expect("admin position query should be valid");

        let response = handle_set_position(
            &game_db, &mut query, None, None, "Mover", -8920.5, -188.25, 97.75,
        );

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
        let control = app
            .world()
            .get::<shared::components::MovementControl>(live_entity)
            .unwrap();
        assert_eq!(control.epoch, 1, "the client adopts the new position");
    }

    #[test]
    fn handle_check_account_reports_existing_user() {
        let db = persistence::temp_database();
        let (account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("account");
        let game_db = persistence::GameDatabase::new(db);

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
        let game_db = persistence::GameDatabase::new(db);

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
        let game_db = persistence::GameDatabase::new(db);

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
        let game_db = persistence::GameDatabase::new(db);

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
        let game_db = persistence::GameDatabase::new(db);

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
        let game_db = persistence::GameDatabase::new(db);

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
        let game_db = persistence::GameDatabase::new(db);
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
        let game_db = persistence::GameDatabase::new(db);
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
        let game_db = persistence::GameDatabase::new(db);
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
        let game_db = persistence::GameDatabase::new(db);
        let initial = EquipmentAppearance {
            entries: vec![shared::components::EquippedAppearanceEntry {
                slot: shared::components::EquipmentVisualSlot::Chest,
                item_id: Some(100),
                definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
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
        let game_db = persistence::GameDatabase::new(db);
        let initial = EquipmentAppearance {
            entries: vec![shared::components::EquippedAppearanceEntry {
                slot: shared::components::EquipmentVisualSlot::Chest,
                item_id: None,
                definition_source: None,
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
        let game_db = persistence::GameDatabase::new(db);
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
