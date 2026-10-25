//! Account Toy Box requests. Retail references in docs/specs/toys.md.
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{GameDatabase, toy_store};
use crate::toy_catalog::{self, ToyCatalog};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    CollectionChannel, ItemLocation, SetToyFavourite, ToyCollectionUpdate, ToyOperation, ToyResult,
    UseToy,
};

pub struct ToysPlugin;
impl Plugin for ToysPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_catalog)
            .add_systems(Update, (receive_favourites, receive_use))
            .add_systems(PostUpdate, send_initial);
    }
}

fn load_catalog(mut commands: Commands) {
    let conn = rusqlite::Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open toy catalog world.db");
    let catalog =
        toy_catalog::load(&conn).expect("load Toy catalog; run scripts/import-toys.py first");
    info!("Loaded {} Toy.db2 entries", catalog.0.len());
    commands.insert_resource(catalog);
    commands.insert_resource(
        crate::spell_cast::toy_scripts::load_toy_texts(&conn)
            .expect("load authored dice BroadcastText rows"),
    );
}

pub(crate) fn snapshot(world: &World, account_id: u64) -> ToyCollectionUpdate {
    let toys = toy_store::load(&world.resource::<GameDatabase>().db, account_id);
    ToyCollectionUpdate {
        toys: world
            .resource::<ToyCatalog>()
            .0
            .values()
            .map(|toy| {
                let mut entry = toy.snapshot.clone();
                entry.learned = toys.contains_key(&entry.item_id);
                entry.favourite = toys.get(&entry.item_id).copied().unwrap_or(false);
                entry.unavailable_reason = toy_catalog::unavailable(toy, world);
                entry.requires_target_witness = entry
                    .spell_id
                    .and_then(|id| world.resource::<crate::spell_info::SpellStore>().get(id))
                    .is_some_and(crate::spell_cast::requires_target_witness);
                entry
            })
            .collect(),
    }
}

fn send_initial(world: &mut World) {
    let mut query = world.query_filtered::<(Entity, &Authenticated, &PlayerLink), Added<InWorld>>();
    let connections: Vec<_> = query
        .iter(world)
        .map(|(entity, auth, link)| (entity, auth.account_id, link.0))
        .collect();
    for (connection, account, player) in connections {
        crate::achievement::record_toy_ownership(world, player, account);
        send_snapshot(world, connection, account);
    }
}

fn send_snapshot(world: &mut World, connection: Entity, account: u64) {
    let update = snapshot(world, account);
    if let Some(mut sender) = world.get_mut::<MessageSender<ToyCollectionUpdate>>(connection) {
        sender.send::<CollectionChannel>(update);
    }
}

pub(crate) fn broadcast(world: &mut World, account: u64) {
    let mut query = world.query_filtered::<(Entity, &Authenticated), With<InWorld>>();
    let connections: Vec<_> = query
        .iter(world)
        .filter(|(_, auth)| auth.account_id == account)
        .map(|(entity, _)| entity)
        .collect();
    for connection in connections {
        send_snapshot(world, connection, account);
    }
}

fn result(
    world: &mut World,
    connection: Entity,
    item_id: u32,
    operation: ToyOperation,
    spell_id: Option<u32>,
    outcome: Result<(), String>,
) {
    if let Some(mut sender) = world.get_mut::<MessageSender<ToyResult>>(connection) {
        sender.send::<CollectionChannel>(ToyResult {
            item_id,
            operation,
            spell_id,
            error: outcome.err(),
        });
    }
}

fn receive_favourites(world: &mut World) {
    let mut query = world.query_filtered::<(
        Entity,
        &Authenticated,
        &mut MessageReceiver<SetToyFavourite>,
    ), With<InWorld>>();
    let requests: Vec<_> = query
        .iter_mut(world)
        .flat_map(|(connection, auth, mut receiver)| {
            let account = auth.account_id;
            receiver
                .receive()
                .map(move |request| (connection, account, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, account, request) in requests {
        let outcome = if world
            .resource::<ToyCatalog>()
            .0
            .contains_key(&request.item_id)
        {
            toy_store::favourite(
                &world.resource::<GameDatabase>().db,
                account,
                request.item_id,
                request.favourite,
            )
        } else {
            Err("unknown toy".into())
        };
        if outcome.is_ok() {
            broadcast(world, account);
        }
        result(
            world,
            connection,
            request.item_id,
            ToyOperation::Favourite,
            None,
            outcome,
        );
    }
}

fn receive_use(world: &mut World) {
    let mut query = world.query_filtered::<(
        Entity,
        &Authenticated,
        &PlayerLink,
        &mut MessageReceiver<UseToy>,
    ), With<InWorld>>();
    let requests: Vec<_> = query
        .iter_mut(world)
        .flat_map(|(connection, auth, link, mut receiver)| {
            let (account, player) = (auth.account_id, link.0);
            receiver
                .receive()
                .map(move |request| (connection, account, player, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, account, player, request) in requests {
        let item_id = request.item_id;
        let spell_id = world
            .resource::<ToyCatalog>()
            .0
            .get(&item_id)
            .and_then(|toy| toy.snapshot.spell_id);
        let outcome = use_toy(world, account, player, &request);
        result(
            world,
            connection,
            item_id,
            ToyOperation::Use,
            spell_id,
            outcome,
        );
    }
}

fn check_item_requirements(world: &World, player: Entity, item_id: u32) -> Result<(), String> {
    let player_data = world
        .get::<shared::components::Player>(player)
        .ok_or("player missing")?;
    let template = world
        .resource::<crate::item_templates::ItemTemplates>()
        .get(shared::item_data::ItemDefinitionSource::Retail, item_id)
        .ok_or("toy item template missing")?;
    if !template.allows(player_data.class, player_data.race) {
        return Err("toy race/class restriction".into());
    }
    let level = world
        .get::<shared::components::Level>(player)
        .ok_or("player level missing")?
        .0;
    if u16::from(level) < template.required_level {
        return Err("toy requires higher level".into());
    }
    Ok(())
}

pub(crate) fn use_toy(
    world: &mut World,
    account: u64,
    player: Entity,
    request: &UseToy,
) -> Result<(), String> {
    let item_id = request.item_id;
    let toy = world
        .resource::<ToyCatalog>()
        .0
        .get(&item_id)
        .cloned()
        .ok_or("unknown toy")?;
    if !toy_store::load(&world.resource::<GameDatabase>().db, account).contains_key(&item_id) {
        return Err("toy not learned".into());
    }
    if let Some(reason) = toy_catalog::unavailable(&toy, world) {
        return Err(reason);
    }
    check_item_requirements(world, player, item_id)?;
    if toy.snapshot.spell_id == Some(16739) {
        let native = world
            .get::<shared::components::Player>(player)
            .ok_or("player missing")?;
        if crate::auras::orb_deception_display(native.race, native.appearance.sex).is_none() {
            return Err(format!(
                "Orb of Deception has no source display mapping for race {}",
                native.race
            ));
        }
    }
    let target = world
        .get::<crate::networking::CombatTarget>(player)
        .and_then(|target| target.0);
    let item_level = world
        .resource::<crate::item_stats::ItemScalingTables>()
        .item_level(
            &world
                .resource::<crate::item_templates::ItemTemplates>()
                .retail[&item_id]
                .scaling,
        );
    crate::spell_cast::cast_toy_spell(world, player, &toy, target, item_level, request).map_err(
        |failure| match failure {
            crate::spell_cast::CastFailure::IncorrectArea(_) => failure
                .to_contract()
                .expect("area failures use the spell result contract")
                .1
                .unwrap(),
            crate::spell_cast::CastFailure::Inventory(error) => error.reason.message(),
            _ => format!("{failure:?}"),
        },
    )
}

/// Called by normal UseItem before casting the item's spell. Returns false for non-toys.
pub(crate) fn learn_item(
    world: &mut World,
    character_id: u64,
    player: Entity,
    location: ItemLocation,
) -> Result<bool, shared::protocol::InventoryError> {
    use shared::protocol::{InventoryError, InventoryErrorReason};
    let fail = |message: String| InventoryError {
        reason: InventoryErrorReason::CantDoRightNow,
        detail: Some(message),
    };
    let db = &world.resource::<GameDatabase>().db;
    let templates = world.resource::<crate::item_templates::ItemTemplates>();
    let record = crate::persistence::inventory_store::load_inventory_record(db, character_id)
        .ok_or_else(|| fail("character inventory missing".into()))?;
    let mut model = crate::inventory::inventory_model::InventoryModel::from_record(
        record,
        templates,
        &mut || crate::persistence::next_id(db, "item_instance"),
    );
    let item = model.existing(location)?;
    if model.definition_source(item.item_guid) != shared::item_data::ItemDefinitionSource::Retail
        || !world.resource::<ToyCatalog>().0.contains_key(&item.item_id)
    {
        return Ok(false);
    }
    let item_id = item.item_id;
    check_item_requirements(world, player, item_id).map_err(fail)?;
    let mut query = world.query::<(Entity, &Authenticated, &PlayerLink, &InWorld)>();
    let (connection, account) = query
        .iter(world)
        .find(|(_, _, link, in_world)| link.0 == player && in_world.character_id == character_id)
        .map(|(entity, auth, _, _)| (entity, auth.account_id))
        .ok_or_else(|| fail("toy learner not authenticated".into()))?;
    let db = &world.resource::<GameDatabase>().db;
    model.destroy(location, 1)?;
    let outcome = toy_store::learn(db, account, character_id, item_id, &model.into_record());
    result(
        world,
        connection,
        item_id,
        ToyOperation::Learn,
        None,
        outcome.clone(),
    );
    outcome.map_err(fail)?;
    world
        .resource_mut::<crate::gold::GoldUpdateQueue>()
        .record_items(character_id);
    crate::achievement::record_toy_ownership(world, player, account);
    broadcast(world, account);
    Ok(true)
}

#[cfg(test)]
#[path = "toys_tests.rs"]
mod tests;
