//! Account Mount Journal; authentic learning, normal spell casts and mount aura cancellation.
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{GameDatabase, mount_store};
use bevy::prelude::*;
use lightyear::prelude::*;
use rand::seq::SliceRandom;
use shared::protocol::{
    CollectionChannel, DismissMount, ItemLocation, MountJournalEntry, MountJournalUpdate,
    SetMountFavourite, SummonMount, SummonRandomFavouriteMount,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Resource, Default)]
pub(crate) struct MountCatalog {
    pub entries: BTreeMap<u32, MountJournalEntry>,
    by_spell: BTreeMap<u32, u32>,
    learn_items: BTreeMap<u32, u32>,
}

#[derive(Component, Default)]
pub(crate) struct AccountMounts(pub mount_store::AccountMounts);

pub struct MountJournalPlugin;
impl Plugin for MountJournalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_catalog)
            .add_systems(
                Update,
                (
                    sync_accounts,
                    receive_favourites,
                    receive_summons,
                    receive_dismissals,
                )
                    .chain(),
            )
            .add_systems(PostUpdate, publish_mount_changes);
    }
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
        COALESCE((SELECT d.CreatureDisplayInfoID FROM mount_x_display d WHERE d.MountID=m.ID ORDER BY d.PlayerConditionID,d.ID LIMIT 1),0)
        FROM mount m ORDER BY m.ID")?;
    for row in query.query_map([], |row| {
        Ok(MountJournalEntry {
            mount_id: row.get(0)?,
            spell_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            source_text: row.get(4)?,
            source_type: row.get(5)?,
            icon_file_data_id: row.get(6)?,
            display_id: row.get(7)?,
            capability_flags: 0,
            collected: false,
            favourite: false,
            usable: false,
            active: false,
        })
    })? {
        let entry = row?;
        catalog.by_spell.insert(entry.spell_id, entry.mount_id);
        catalog.entries.insert(entry.mount_id, entry);
    }
    load_capabilities(conn, &mut catalog)?;
    let mut query = conn.prepare("SELECT x.ItemID,m.ID FROM item_x_item_effect x JOIN item_effect e ON e.ID=x.ItemEffectID JOIN mount m ON m.SourceSpellID=e.SpellID WHERE e.TriggerType=6 ORDER BY e.ID")?;
    for row in query.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))? {
        let (item, mount) = row?;
        catalog.learn_items.insert(item, mount);
    }
    Ok(catalog)
}

fn load_capabilities(
    conn: &rusqlite::Connection,
    catalog: &mut MountCatalog,
) -> rusqlite::Result<()> {
    let mut query = conn.prepare("SELECT m.ID,c.Flags FROM mount m JOIN mount_type_x_capability x ON x.MountTypeID=m.MountTypeID JOIN mount_capability c ON c.ID=x.MountCapabilityID")?;
    for row in query.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))? {
        let (id, flags) = row?;
        catalog
            .entries
            .get_mut(&id)
            .expect("catalog Mount row")
            .capability_flags |= flags;
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
    for (connection, account, player) in connections(world) {
        let initial = world.get::<AccountMounts>(player).is_none();
        if initial {
            let owned = mount_store::load(&world.resource::<GameDatabase>().db, account);
            world.entity_mut(player).insert(AccountMounts(owned));
        }
        let owned = &world.get::<AccountMounts>(player).expect("loaded mounts").0;
        let newly_learned: Vec<_> = learned_mount_ids(world, player)
            .into_iter()
            .filter(|id| !owned.contains_key(id))
            .collect();
        if !newly_learned.is_empty() {
            mount_store::learn_spells(
                &world.resource::<GameDatabase>().db,
                account,
                &newly_learned,
            );
            broadcast_account(world, account);
        } else if initial {
            send_snapshot(world, connection, player, None);
        }
    }
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

fn send_snapshot(world: &mut World, connection: Entity, player: Entity, error: Option<String>) {
    let owned = &world.get::<AccountMounts>(player).expect("loaded mounts").0;
    let auras = world.get::<crate::auras::ActiveAuras>(player);
    let mounts = world
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
        .collect();
    if let Some(mut sender) = world.get_mut::<MessageSender<MountJournalUpdate>>(connection) {
        sender.send::<CollectionChannel>(MountJournalUpdate { mounts, error });
    }
}

fn broadcast_account(world: &mut World, account: u64) {
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

fn requests<M: Message>(world: &mut World) -> Vec<(Entity, u64, Entity, M)> {
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
        let result = mount_store::favourite(
            &world.resource::<GameDatabase>().db,
            account,
            request.mount_id,
            request.favourite,
        );
        if result.is_ok() {
            broadcast_account(world, account);
        } else {
            send_snapshot(world, connection, player, result.err());
        }
    }
}

fn random_candidates(world: &World, player: Entity) -> Vec<u32> {
    world
        .resource::<MountCatalog>()
        .entries
        .values()
        .filter(|entry| {
            world
                .get::<AccountMounts>(player)
                .is_some_and(|mounts| mounts.0.get(&entry.mount_id) == Some(&true))
                && usable(world, player, entry)
        })
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
    for (connection, _, player, _) in requests::<SummonRandomFavouriteMount>(world) {
        let candidates = random_candidates(world, player);
        let result = candidates
            .choose(&mut rand::thread_rng())
            .copied()
            .ok_or_else(|| "No usable favourite mounts.".to_string())
            .and_then(|id| summon(world, player, id));
        if let Err(error) = result {
            send_snapshot(world, connection, player, Some(error));
        }
    }
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
