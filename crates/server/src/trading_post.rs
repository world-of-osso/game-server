//! Server-authoritative Mainline Trading Post, using existing monthly gameplay credit.
#[path = "trading_post_catalog.rs"]
mod catalog;
#[path = "trading_post_monthly.rs"]
mod monthly;
#[path = "trading_post_persistence.rs"]
mod persistence;
#[cfg(test)]
#[path = "trading_post_tests.rs"]
mod tests;
use crate::{
    auth::{Authenticated, InWorld},
    networking::PlayerLink,
    persistence::GameDatabase,
    trading_post_store as store,
};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::*;
use std::collections::HashSet;

/// Refund window for new purchases. Retail's duration is unsourced, so it stays 0
/// (refunds disabled) unless a server administrator sets it on the admin socket.
#[derive(Resource, Default, Clone, Copy)]
pub(crate) struct RefundWindow(pub u32);

/// Admin-socket test setup: credit `amount` Tender to `username`'s account.
pub(crate) fn admin_grant_tender(
    db: &redb::Database,
    username: &str,
    amount: u32,
) -> Result<u32, String> {
    let account = crate::persistence_accounts::find_account_by_username(db, username)
        .ok_or_else(|| format!("Account {username} not found"))?;
    Ok(store::grant_tender(db, account, amount)?.tender)
}

pub(crate) struct TradingPostPlugin;
impl Plugin for TradingPostPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<catalog::Catalog>()
            .init_resource::<RefundWindow>()
            .add_systems(Startup, load_catalog)
            .add_systems(
                Update,
                receive_requests.after(crate::npc_interaction::NpcInteractionsProcessed),
            );
    }
}
fn load_catalog(mut commands: Commands) {
    let conn = rusqlite::Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open Trading Post world.db");
    let monthly_path = std::path::Path::new(crate::spell_info::WORLD_DB_PATH)
        .with_file_name("trading-post-monthly-catalog.json");
    let loaded = catalog::load(&conn, &monthly_path);
    if let Err(error) = &loaded {
        error!("Trading Post catalog unavailable: {error}");
    }
    commands.insert_resource(catalog::Catalog(loaded));
}

fn receive_requests(world: &mut World) {
    persistence::flush_replies(world);
    let requests: Vec<_> = world
        .query_filtered::<(
            Entity,
            &Authenticated,
            &PlayerLink,
            &mut MessageReceiver<TradingPostRequest>,
        ), With<InWorld>>()
        .iter_mut(world)
        .flat_map(|(connection, account, link, mut receiver)| {
            receiver
                .receive()
                .map(|request| (connection, account.account_id, link.0, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, account, player, request) in requests {
        info!(?connection, account, ?request, "TP_DELIVERY_SERVER_RECV");
        crate::adventure_guide::settle_month_for_connection(world, connection);
        persistence::queue_request(world, connection, account, player, &request);
    }
}

/// Which catalog rows a reply carries. The 1321-row catalog is ~0.5 MB, so only a
/// vendor Query sends it; a purchase or refund sends the rows it can change.
enum Rows {
    Catalog,
    AffectedBy(Vec<u32>),
    None,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum CollectionKey {
    Appearance(u32),
    Mount(u32),
    Pet(u32),
    Toy(u32),
}

fn collection_keys(grant: &store::Grant) -> Vec<CollectionKey> {
    match grant {
        store::Grant::Appearances(ids) => {
            ids.iter().copied().map(CollectionKey::Appearance).collect()
        }
        store::Grant::Mount(id) => vec![CollectionKey::Mount(*id)],
        store::Grant::Pet(id) => vec![CollectionKey::Pet(*id)],
        store::Grant::Toy(id) => vec![CollectionKey::Toy(*id)],
        // A receipt's concrete pet instance is never a catalog collection key.
        store::Grant::PetInstance(_) => Vec::new(),
    }
}

/// Batch lookup avoids scanning every cart product for every catalog row.
fn affected_rows(catalog: &catalog::CatalogData, vendors: &[u32]) -> HashSet<u32> {
    let changed: HashSet<_> = vendors.iter().copied().collect();
    let keys: HashSet<_> = vendors
        .iter()
        .filter_map(|id| {
            catalog
                .items
                .get(id)
                .and_then(|(_, grant)| grant.as_ref().ok())
        })
        .flat_map(collection_keys)
        .collect();
    catalog
        .items
        .iter()
        .filter(|(id, (_, grant))| {
            changed.contains(*id)
                || grant
                    .as_ref()
                    .is_ok_and(|grant| collection_keys(grant).iter().any(|key| keys.contains(key)))
        })
        .map(|(id, _)| *id)
        .collect()
}

fn active_vendor(world: &World, connection: Entity, player: Entity) -> Result<Entity, String> {
    let active = world
        .get::<crate::npc_interaction::ActiveInteraction>(connection)
        .ok_or("No active Trading Post interaction.")?;
    if active.role != Some(NpcRole::TradingPost) {
        return Err("The active interaction is not a Trading Post vendor.".into());
    }
    crate::npc_interaction::find_interactable_trading_vendor(world, player, active.npc)
        .map_err(|error| format!("Trading Post interaction: {error:?}"))?;
    Ok(active.npc)
}

fn prepare_snapshot(
    world: &World,
    connection: Entity,
    rows: &Rows,
) -> (TradingPostSnapshot, Option<catalog::CatalogData>, i64) {
    let now = world.resource::<crate::maps::ServerClock>().unix();
    let calendar = crate::adventure_guide::calendar_month(now);
    let summary = world
        .get::<crate::adventure_guide::MonthlySummary>(connection)
        .copied()
        .unwrap_or(crate::adventure_guide::MonthlySummary {
            points: 0,
            start_unix: calendar.start_unix,
            end_unix: calendar.end_unix,
        });
    let state = store::State::default();
    let npc = world
        .get::<PlayerLink>(connection)
        .and_then(|link| active_vendor(world, connection, link.0).ok())
        .map(Entity::to_bits);
    let mut snapshot =
        TradingPostSnapshot {
            npc_net_id: npc,
            tender: state.tender,
            points: summary.points,
            now_unix: now,
            month_end_unix: summary.end_unix,
            frozen_item_id: state.frozen,
            categories: Vec::new(),
            items: Vec::new(),
            thresholds: Vec::new(),
            catalog_complete: matches!(rows, Rows::Catalog),
            source_gaps: vec![
            "TP_MONTHLY_CATALOG_WOWHEAD_NEWS_BEST_GUESS: news-derived inventory; not Retail parity. Seasonal activity/reward binding remains unresolved."
                .into(),
            refund_gap(world.get_resource::<RefundWindow>().map_or(0, |window| window.0)),
        ],
            error: None,
        };
    if let crate::adventure_guide::AdventureCatalog::Ready(data) =
        world.resource::<crate::adventure_guide::AdventureCatalog>()
    {
        snapshot.thresholds = data
            .thresholds
            .iter()
            .map(|row| TradingPostThreshold {
                id: row.id,
                points: row.points,
                tender: row.tender,
                claimed: state.claimed.contains(&row.id),
            })
            .collect();
    }
    let reply_catalog = match &world.resource::<catalog::Catalog>().0 {
        Ok(catalog) => {
            let affected = match rows {
                Rows::Catalog => {
                    snapshot.categories = catalog.categories.clone();
                    None
                }
                Rows::AffectedBy(vendors) => Some(affected_rows(catalog, vendors)),
                Rows::None => Some(HashSet::new()),
            };
            let items = catalog
                .items
                .iter()
                .filter(|(id, _)| affected.as_ref().is_none_or(|ids| ids.contains(*id)))
                .map(|(id, (row, grant))| {
                    let mut row = row.clone();
                    row.usable = item_usability(world, connection, row.item_id);
                    (*id, (row, grant.clone()))
                })
                .collect();
            Some(catalog::CatalogData {
                categories: Vec::new(),
                items,
                monthly: catalog.monthly.clone(),
            })
        }
        Err(error) => {
            snapshot.source_gaps.push(error.clone());
            None
        }
    };
    (snapshot, reply_catalog, summary.start_unix)
}

/// Existing Player::CanUseItem class/race/level facts, also used by merchant lists.
fn item_usability(world: &World, connection: Entity, item_id: u32) -> Option<bool> {
    let player_id = world.get::<PlayerLink>(connection)?.0;
    let player = world.get::<shared::components::Player>(player_id)?;
    let level = u16::from(world.get::<shared::components::Level>(player_id)?.0);
    let templates = world.get_resource::<crate::item_templates::ItemTemplates>()?;
    let item = templates.get(shared::item_data::ItemDefinitionSource::Retail, item_id)?;
    Some(item.allows(player.class, player.race) && item.required_level <= level)
}

fn refund_gap(window: u32) -> String {
    if window == 0 {
        "Refund duration unsourced: refunds disabled.".into()
    } else {
        format!("Refund duration unsourced: {window}s window set by server administrator.")
    }
}

fn collection_rows(
    db: &redb::Database,
    account: u64,
    catalog: &catalog::CatalogData,
    state: &store::State,
    now: i64,
) -> Vec<TradingPostItem> {
    let appearances = crate::persistence::appearance_store::load_account_appearances(db, account);
    let mounts = crate::persistence::mount_store::load(db, account);
    let toys = crate::persistence::toy_store::load(db, account);
    let pets = crate::battle_pets::load_journal(db, account);
    catalog
        .items
        .values()
        .map(|(row, grant)| {
            let mut row = row.clone();
            row.collected = match grant {
                Ok(store::Grant::Appearances(ids)) => ids.iter().all(|id| appearances.has(*id)),
                Ok(store::Grant::Mount(id)) => mounts.contains_key(id),
                Ok(store::Grant::Pet(id)) => pets.count_species(*id) > 0,
                Ok(store::Grant::Toy(id)) => toys.contains_key(id),
                _ => false,
            };
            if let Some(receipt) = state.purchases.get(&row.id) {
                row.purchased = true;
                row.refund_seconds = receipt
                    .refund_until
                    .saturating_sub(now)
                    .clamp(0, i64::from(u32::MAX)) as u32;
            }
            row
        })
        .collect()
}

fn publish_collections(world: &mut World, account: u64, changed: &[(u32, store::Grant)]) {
    if changed
        .iter()
        .any(|(_, grant)| matches!(grant, store::Grant::Mount(_)))
    {
        crate::mount_journal::broadcast_account(world, account);
    }
    if changed
        .iter()
        .any(|(_, grant)| matches!(grant, store::Grant::Toy(_)))
    {
        crate::toys::broadcast(world, account);
    }
    let appearances = changed
        .iter()
        .any(|(_, grant)| matches!(grant, store::Grant::Appearances(_)));
    let pets = changed
        .iter()
        .any(|(_, grant)| matches!(grant, store::Grant::Pet(_) | store::Grant::PetInstance(_)));
    if appearances || pets {
        publish_appearances_and_pets(world, account, appearances, pets);
    }
}

fn publish_appearances_and_pets(
    world: &mut World,
    account: u64,
    appearances_changed: bool,
    pets_changed: bool,
) {
    let owners: Vec<_> = world
        .query_filtered::<(Entity, &Authenticated, &PlayerLink), With<InWorld>>()
        .iter(world)
        .filter(|(_, auth, _)| auth.account_id == account)
        .map(|(connection, _, link)| (connection, link.0))
        .collect();
    let appearances = crate::persistence::appearance_store::load_account_appearances(
        &world.resource::<GameDatabase>().db,
        account,
    );
    let journal = crate::battle_pets::load_journal(&world.resource::<GameDatabase>().db, account);
    for (connection, player) in owners {
        if appearances_changed
            && let Some(mut sender) =
                world.get_mut::<MessageSender<AppearanceCollectionUpdate>>(connection)
        {
            sender.send::<TooltipChannel>(AppearanceCollectionUpdate {
                appearances: appearances.all().collect(),
            });
        }
        if pets_changed
            && let Some(mut state) = world.get_mut::<crate::battle_pets::AccountPetJournal>(player)
        {
            state.journal = journal.clone();
            crate::battle_pets::refresh_catalog_snapshot(world, player);
            crate::battle_pets::send_pet_update(world, connection, player, None);
        }
    }
}
