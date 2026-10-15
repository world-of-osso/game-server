//! Account Mount Journal; authentic learning, normal spell casts and mount aura cancellation.
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{GameDatabase, mount_store};
use bevy::prelude::*;
use lightyear::prelude::*;
use rand::seq::SliceRandom;
use shared::protocol::mount_journal_types as types;
use shared::protocol::{
    CollectionChannel, DismissMount, ItemLocation, MountJournalEntry, MountJournalUpdate,
    SetMountFavourite, SummonMount, SummonRandomFavouriteMount,
};
use std::collections::{BTreeMap, BTreeSet};
const CAPABILITY_GROUND: u32 = 0x1;
const CAPABILITY_FLYING: u32 = 0x2;

#[derive(Resource, Default)]
pub(crate) struct MountCatalog {
    pub entries: BTreeMap<u32, MountJournalEntry>,
    by_spell: BTreeMap<u32, u32>,
    learn_items: BTreeMap<u32, u32>,
}

#[derive(Component, Default)]
pub(crate) struct AccountMounts(pub mount_store::AccountMounts);

mod actor;
use actor::{
    PendingMountRequests, enqueue_mount_favourite, enqueue_mount_spell_learning,
    enqueue_random_mount, publish_mount_favourites, publish_mount_spell_learning,
    publish_random_mounts,
};

pub struct MountJournalPlugin;
impl Plugin for MountJournalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingMountRequests>()
            .add_systems(Startup, load_catalog)
            .add_systems(Update, update_mount_journal);
    }
}

// Auth may enter world before or after this exclusive step. Every reader sees
// account state initialized first; replies are queued before Lightyear's send cycle.
fn update_mount_journal(world: &mut World) {
    publish_mount_spell_learning(world);
    publish_mount_favourites(world);
    publish_random_mounts(world);
    sync_accounts(world);
    publish_mount_changes(world);
    receive_favourites(world);
    receive_summons(world);
    receive_dismissals(world);
}

fn load_catalog(mut commands: Commands) {
    let conn = rusqlite::Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open Mount Journal world.db");
    commands.insert_resource(load(&conn).expect("load Mount Journal DB2 joins"));
}

pub(crate) fn load(conn: &rusqlite::Connection) -> rusqlite::Result<MountCatalog> {
    let mut catalog = MountCatalog::default();
    let mut query = conn.prepare("SELECT m.ID,m.SourceSpellID,m.Name_lang,m.Description_lang,m.SourceText_lang,m.SourceTypeEnum,
        COALESCE((SELECT s.SpellIconFileDataID FROM spell_misc s WHERE s.SpellID=m.SourceSpellID AND s.DifficultyID=0 LIMIT 1),0),
        COALESCE((SELECT d.CreatureDisplayInfoID FROM mount_x_display d WHERE d.MountID=m.ID ORDER BY d.PlayerConditionID,d.ID LIMIT 1),0),m.Flags
        FROM mount m ORDER BY m.ID")?;
    for row in query.query_map([], |row| {
        Ok(MountJournalEntry {
            mount_id: row.get(0)?,
            spell_id: row.get(1)?,
            teaching_items: Vec::new(),
            name: row.get(2)?,
            description: row.get(3)?,
            source_text: row.get(4)?,
            source_type: row.get(5)?,
            icon_file_data_id: row.get(6)?,
            display_id: row.get(7)?,
            capability_flags: 0,
            journal_types: 0,
            flags: row.get(8)?,
            collected: false,
            favourite: false,
            usable: false,
            active: false,
        })
    })? {
        let entry = row?;
        if entry.spell_id != 0 {
            catalog.by_spell.insert(entry.spell_id, entry.mount_id);
        }
        catalog.entries.insert(entry.mount_id, entry);
    }
    load_capabilities(conn, &mut catalog)?;
    load_ride_along_types(conn, &mut catalog)?;
    let mut query = conn.prepare("SELECT x.ItemID,m.ID FROM item_x_item_effect x JOIN item_effect e ON e.ID=x.ItemEffectID JOIN mount m ON m.SourceSpellID=e.SpellID WHERE e.TriggerType=6 AND m.SourceSpellID<>0 ORDER BY e.ID")?;
    for row in query.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))? {
        let (item, mount) = row?;
        catalog.learn_items.insert(item, mount);
        catalog
            .entries
            .get_mut(&mount)
            .expect("authentic teaching mount")
            .teaching_items
            .push(item);
    }
    Ok(catalog)
}

fn load_capabilities(
    conn: &rusqlite::Connection,
    catalog: &mut MountCatalog,
) -> rusqlite::Result<()> {
    let mut query = conn.prepare("SELECT m.ID,c.Flags,c.FlightCapabilityID,
        EXISTS(SELECT 1 FROM spell_effect e WHERE e.SpellID=c.ModSpellAuraID AND e.DifficultyID=0 AND e.EffectAura=58 AND e.EffectBasePointsF>0)
        FROM mount m JOIN mount_type_x_capability x ON x.MountTypeID=m.MountTypeID JOIN mount_capability c ON c.ID=x.MountCapabilityID")?;
    for row in query.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            row.get::<_, u32>(2)?,
            row.get::<_, bool>(3)?,
        ))
    })? {
        let (id, flags, flight, swim) = row?;
        let entry = catalog.entries.get_mut(&id).expect("catalog Mount row");
        entry.capability_flags |= flags;
        if flags & CAPABILITY_FLYING != 0 {
            entry.journal_types |= types::FLYING;
        }
        if flight != 0 {
            entry.journal_types |= types::SKYRIDING;
        }
        if swim {
            entry.journal_types |= types::AQUATIC;
        }
    }
    for entry in catalog.entries.values_mut() {
        if entry.journal_types == 0 && entry.capability_flags & CAPABILITY_GROUND != 0 {
            entry.journal_types = types::GROUND;
        }
    }
    Ok(())
}

fn load_ride_along_types(
    conn: &rusqlite::Connection,
    catalog: &mut MountCatalog,
) -> rusqlite::Result<()> {
    // Same authentic mount/vehicle/enter-exit seat join as scripts/vehicle_models.sql.
    let mut query=conn.prepare("SELECT DISTINCT m.ID FROM mount m
        JOIN spell_effect e ON e.SpellID=m.SourceSpellID
        JOIN creature_template c ON c.entry=e.EffectMiscValue_0
        JOIN vehicle v ON v.ID=c.VehicleId
        JOIN vehicle_seat s ON s.ID IN(v.SeatID_0,v.SeatID_1,v.SeatID_2,v.SeatID_3,v.SeatID_4,v.SeatID_5,v.SeatID_6,v.SeatID_7)
        WHERE e.EffectAura=78 AND e.DifficultyID=0 AND s.ID<>0 AND (s.Flags & 33554432)<>0 AND (s.Flags & 2048)=0")?;
    for row in query.query_map([], |row| row.get::<_, u32>(0))? {
        catalog
            .entries
            .get_mut(&row?)
            .expect("catalog Mount row")
            .journal_types |= types::RIDE_ALONG;
    }
    Ok(())
}

fn connections(world: &mut World) -> Vec<(Entity, u64, Entity)> {
    let mut query = world.query_filtered::<(Entity, &Authenticated, &PlayerLink), With<InWorld>>();
    query
        .iter(world)
        .map(|(conn, auth, link)| (conn, auth.account_id, link.0))
        .collect()
}

fn learned_mount_ids(world: &World, player: Entity) -> BTreeSet<u32> {
    let known = world
        .get::<crate::class_progression::KnownSpells>(player)
        .into_iter()
        .flat_map(|spells| spells.iter());
    let learned = world
        .get::<crate::profession::PlayerProfessions>(player)
        .into_iter()
        .flat_map(|profs| profs.spells.iter().copied());
    let catalog = world.resource::<MountCatalog>();
    known
        .chain(learned)
        .filter_map(|spell| catalog.by_spell.get(&spell).copied())
        .collect()
}

fn sync_accounts(world: &mut World) {
    for (_, account, player) in connections(world) {
        let initial = world.get::<AccountMounts>(player).is_none();
        if initial {
            let owned = mount_store::load(&world.resource::<GameDatabase>().db, account);
            world.entity_mut(player).insert(AccountMounts(owned));
        }
        let owned = &world.get::<AccountMounts>(player).expect("loaded mounts").0;
        let newly_learned: Vec<_> = learned_mount_ids(world, player)
            .into_iter()
            .filter(|id| {
                !owned.contains_key(id)
                    && !world
                        .resource::<PendingMountRequests>()
                        .mount_submitted(account, *id)
            })
            .collect();
        if !newly_learned.is_empty() {
            enqueue_mount_spell_learning(world, account, newly_learned);
        }
    }
}

/// Source spells of an account's owned mounts.
pub(crate) fn owned_spells<'a>(
    world: &'a World,
    mounts: Option<&'a AccountMounts>,
) -> impl Iterator<Item = u32> + 'a {
    let catalog = world.get_resource::<MountCatalog>();
    mounts
        .into_iter()
        .flat_map(|mounts| mounts.0.keys())
        .filter_map(move |id| catalog?.entries.get(id).map(|entry| entry.spell_id))
}

pub(crate) fn knows_spell(world: &World, player: Entity, spell: u32) -> bool {
    world
        .get_resource::<MountCatalog>()
        .and_then(|catalog| catalog.by_spell.get(&spell))
        .is_some_and(|id| {
            world
                .get::<AccountMounts>(player)
                .is_some_and(|mounts| mounts.0.contains_key(id))
        })
}

fn usable(world: &World, player: Entity, entry: &MountJournalEntry) -> bool {
    let Some(store) = world.get_resource::<crate::auras::mount::MountStore>() else {
        return false;
    };
    let Some(mount) = store.mount_for_spell(entry.spell_id) else {
        return false;
    };
    crate::auras::mount::with_rider(world, player, |rider| {
        store.capability(mount.mount_type, rider).is_some()
            && !store.usable_displays(entry.mount_id, rider).is_empty()
    })
}

#[derive(Component)]
struct PublishedMountSnapshot {
    player: Entity,
    rows: BTreeMap<u32, MountJournalEntry>,
}

fn build_mount_rows(world: &World, player: Entity) -> Vec<MountJournalEntry> {
    let owned = &world.get::<AccountMounts>(player).expect("loaded mounts").0;
    let auras = world.get::<crate::auras::ActiveAuras>(player);
    world
        .resource::<MountCatalog>()
        .entries
        .values()
        .map(|entry| {
            let mut row = entry.clone();
            row.collected = owned.contains_key(&row.mount_id);
            row.favourite = owned.get(&row.mount_id).copied().unwrap_or(false);
            row.usable = row.collected && usable(world, player, entry);
            row.active =
                auras.is_some_and(|auras| auras.iter().any(|aura| aura.spell_id == entry.spell_id));
            row
        })
        .collect()
}

fn build_mount_update(
    previous: Option<&PublishedMountSnapshot>,
    player: Entity,
    rows: &[MountJournalEntry],
    error: Option<String>,
) -> MountJournalUpdate {
    let previous_rows = previous
        .filter(|snapshot| snapshot.player == player)
        .map(|snapshot| &snapshot.rows);
    let complete = previous_rows.is_none();
    let mounts = rows
        .iter()
        .filter(|row| previous_rows.is_none_or(|previous| previous.get(&row.mount_id) != Some(row)))
        .cloned()
        .collect();
    MountJournalUpdate {
        complete,
        mounts,
        error,
    }
}

fn send_snapshot(world: &mut World, connection: Entity, player: Entity, error: Option<String>) {
    let rows = build_mount_rows(world, player);
    let update = build_mount_update(
        world.get::<PublishedMountSnapshot>(connection),
        player,
        &rows,
        error,
    );
    info!(?connection, complete = update.complete, count = update.mounts.len(), mare = ?update.mounts.iter().find(|mount| mount.mount_id == 18).map(|mount| (mount.collected, mount.favourite, mount.active)), error = ?update.error, "MOUNT_JOURNAL_SENT");
    world.entity_mut(connection).insert(PublishedMountSnapshot {
        player,
        rows: rows.into_iter().map(|row| (row.mount_id, row)).collect(),
    });
    if let Some(mut sender) = world.get_mut::<MessageSender<MountJournalUpdate>>(connection) {
        sender.send::<CollectionChannel>(update);
    }
}

pub(crate) fn broadcast_account(world: &mut World, account: u64) {
    let owned = mount_store::load(&world.resource::<GameDatabase>().db, account);
    for (connection, id, player) in connections(world) {
        if id == account {
            world
                .entity_mut(player)
                .insert(AccountMounts(owned.clone()));
            send_snapshot(world, connection, player, None);
        }
    }
}

fn requests<M: lightyear::prelude::Message>(world: &mut World) -> Vec<(Entity, u64, Entity, M)> {
    let mut query = world.query_filtered::<(Entity,&Authenticated,&PlayerLink,&mut MessageReceiver<M>),With<InWorld>>();
    query
        .iter_mut(world)
        .flat_map(|(conn, auth, link, mut receiver)| {
            receiver
                .receive()
                .map(|request| (conn, auth.account_id, link.0, request))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn receive_favourites(world: &mut World) {
    for (connection, account, player, request) in requests::<SetMountFavourite>(world) {
        enqueue_mount_favourite(world, connection, account, player, request);
    }
}

#[cfg(test)]
fn random_candidates(world: &World, player: Entity) -> Vec<u32> {
    world
        .get::<AccountMounts>(player)
        .map(|mounts| random_candidates_from_ownership(world, player, &mounts.0))
        .unwrap_or_default()
}

fn random_candidates_from_ownership(
    world: &World,
    player: Entity,
    owned: &mount_store::AccountMounts,
) -> Vec<u32> {
    world
        .resource::<MountCatalog>()
        .entries
        .values()
        .filter(|entry| owned.get(&entry.mount_id) == Some(&true) && usable(world, player, entry))
        .map(|entry| entry.mount_id)
        .collect()
}

fn summon(world: &mut World, player: Entity, id: u32) -> Result<(), String> {
    let entry = world
        .resource::<MountCatalog>()
        .entries
        .get(&id)
        .ok_or("Unknown mount.")?;
    if !knows_spell(world, player, entry.spell_id) {
        return Err("You have not collected this mount.".into());
    }
    if !usable(world, player, entry) {
        return Err("This mount is not usable here or requires riding skill.".into());
    }
    let spell = entry.spell_id;
    crate::spell_cast::cast_for_player(world, player, spell)
        .map_err(|failure| format!("{failure:?}"))
}

fn receive_summons(world: &mut World) {
    for (connection, _, player, request) in requests::<SummonMount>(world) {
        if let Err(error) = summon(world, player, request.mount_id) {
            send_snapshot(world, connection, player, Some(error));
        }
    }
    for (connection, account, player, _) in requests::<SummonRandomFavouriteMount>(world) {
        enqueue_random_mount(world, connection, account, player);
    }
}

fn summon_random_from_ownership(
    world: &mut World,
    player: Entity,
    owned: &mount_store::AccountMounts,
) -> Result<(), String> {
    let candidates = random_candidates_from_ownership(world, player, owned);
    candidates
        .choose(&mut rand::thread_rng())
        .copied()
        .ok_or_else(|| "No usable favourite mounts.".to_string())
        .and_then(|id| summon(world, player, id))
}

fn receive_dismissals(world: &mut World) {
    for (_, _, player, _) in requests::<DismissMount>(world) {
        crate::auras::mount::cancel_mount_aura(world, player);
    }
}

#[derive(Component, PartialEq, Eq)]
struct PublishedMountContext {
    display: Option<u32>,
    map: u32,
    riding: u16,
    swimming: bool,
}

fn publish_mount_changes(world: &mut World) {
    for (connection, _, player) in connections(world) {
        let context =
            crate::auras::mount::with_rider(world, player, |rider| PublishedMountContext {
                display: world
                    .get::<shared::components::Mounted>(player)
                    .map(|mount| mount.mount_display_id),
                map: rider.map_id,
                riding: rider.riding_skill,
                swimming: rider.submerged,
            });
        if world.get::<PublishedMountContext>(player) != Some(&context) {
            world.entity_mut(player).insert(context);
            send_snapshot(world, connection, player, None);
        }
    }
}

/// Only authentic ITEM_SPELLTRIGGER_LEARN_SPELL rows teach a mount; normal use mounts do not.
pub(crate) fn learn_item(
    world: &mut World,
    character: u64,
    player: Entity,
    location: ItemLocation,
) -> Result<bool, shared::protocol::InventoryError> {
    let fail = |detail: String| shared::protocol::InventoryError {
        reason: shared::protocol::InventoryErrorReason::CantDoRightNow,
        detail: Some(detail),
    };
    let (db, templates) = crate::inventory::use_store(world)?;
    let record = crate::persistence::inventory_store::load_inventory_record(db, character)
        .ok_or_else(|| fail("Character inventory missing.".into()))?;
    let model = crate::inventory::inventory_model::InventoryModel::from_record(
        record,
        templates,
        &mut || crate::persistence::next_id(db, "item_instance"),
    );
    let item = model.existing(location)?;
    if model.definition_source(item.item_guid) != shared::item_data::ItemDefinitionSource::Retail {
        return Ok(false);
    }
    let Some(id) = world
        .get_resource::<MountCatalog>()
        .and_then(|catalog| catalog.learn_items.get(&item.item_id))
        .copied()
    else {
        return Ok(false);
    };
    let item_id = item.item_id;
    let template = &templates.retail[&item_id];
    let rider = world
        .get::<shared::components::Player>(player)
        .ok_or_else(|| fail("Player missing.".into()))?;
    if !template.allows(rider.class, rider.race) {
        return Err(fail("Mount item race/class restriction.".into()));
    }
    let level = world
        .get::<shared::components::Level>(player)
        .map_or(0, |level| level.0);
    if u16::from(level) < template.required_level {
        return Err(fail("Level too low to learn this mount.".into()));
    }
    let account = connections(world)
        .into_iter()
        .find(|(_, _, owner)| *owner == player)
        .map(|(_, account, _)| account)
        .ok_or_else(|| fail("Mount learner not authenticated.".into()))?;
    let (db, templates) = crate::inventory::use_store(world)?;
    let record = crate::persistence::inventory_store::load_inventory_record(db, character)
        .ok_or_else(|| fail("Character inventory missing.".into()))?;
    let mut model = crate::inventory::inventory_model::InventoryModel::from_record(
        record,
        templates,
        &mut || crate::persistence::next_id(db, "item_instance"),
    );
    model.destroy(location, 1)?;
    mount_store::learn_item(db, account, character, id, &model.into_record()).map_err(fail)?;
    world
        .resource_mut::<crate::gold::GoldUpdateQueue>()
        .record_items(character);
    broadcast_account(world, account);
    Ok(true)
}

#[cfg(test)]
#[path = "mount_journal_tests.rs"]
mod tests;
