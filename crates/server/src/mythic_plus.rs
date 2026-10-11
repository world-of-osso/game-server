//! Authenticated owner bag keys, Font slots and copy-scoped challenge runs.
use crate::auth::{Authenticated, InWorld};
use crate::maps::{InMap, InstanceManager};
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase};
use ::server::mythic_plus_catalog::{FontContext, load_azure, validate_font};
use ::server::mythic_plus_runtime::{ChallengeDefinition, Run, UnsourcedChallengeConfig};
use bevy::prelude::*;
use lightyear::prelude::*;
use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use shared::components::{Health, Npc, Position};
use shared::protocol::*;
use std::collections::HashMap;

const KEYS: TableDefinition<u64, &[u8]> = TableDefinition::new("mythic_keystones");
const HISTORY: TableDefinition<u64, &[u8]> = TableDefinition::new("mythic_run_history");
pub(crate) const KEYSTONE_ITEM: u32 = 180653;
#[derive(Clone)]
struct Active {
    owner: u64,
    font: Entity,
    run: Run,
    settled: bool,
}
#[derive(Resource, Default)]
struct Challenges {
    definition: Option<ChallengeDefinition>,
    config: UnsourcedChallengeConfig,
    fonts: HashMap<Entity, Entity>,
    slots: HashMap<Entity, Active>,
    runs: HashMap<InMap, Active>,
}
pub struct MythicPlusPlugin;
impl Plugin for MythicPlusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Challenges>()
            .add_systems(PostStartup, load_startup)
            .add_systems(Update, update);
    }
}
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before Unix epoch")
        .as_millis() as u64
}
fn load_startup(world: &mut World) {
    let db = &world.resource::<GameDatabase>().db;
    let txn = db.begin_write().expect("begin challenge schema");
    txn.open_table(KEYS).expect("create keystone table");
    txn.open_table(HISTORY).expect("create run history");
    txn.commit().expect("commit challenge schema");
    let path = std::path::Path::new(crate::spell_info::WORLD_DB_PATH);
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("open challenge world data");
    let definition = load_azure(&conn).expect("qualify imported Azure challenge");
    let config = match std::env::var_os("GAME_SERVER_UNSOURCED_CHALLENGE_CONFIG") {
        Some(path) => {
            serde_json::from_slice(&std::fs::read(path).expect("read unsourced challenge config"))
                .expect("decode unsourced challenge config")
        }
        None => UnsourcedChallengeConfig::default(),
    };
    let mut challenges = world.resource_mut::<Challenges>();
    challenges.definition = definition;
    challenges.config = config;
}
fn save_key(db: &redb::Database, key: &KeystoneInfo) {
    let txn = db.begin_write().expect("begin keystone write");
    txn.open_table(KEYS)
        .expect("open keystones")
        .insert(key.item_guid, bitcode::encode(key).as_slice())
        .expect("write keystone");
    txn.commit().expect("commit keystone");
}
fn owned_keys(db: &redb::Database, character: u64) -> Vec<KeystoneInfo> {
    let inventory = persistence::load_inventory(db, character);
    let txn = db.begin_read().expect("begin keystone read");
    let table = txn.open_table(KEYS).expect("open keystones");
    inventory
        .items
        .iter()
        .filter_map(|item| {
            let row = table.get(item.item_guid).expect("read keystone")?;
            let key: KeystoneInfo = bitcode::decode(row.value()).expect("decode keystone");
            (key.item_id == item.item_id).then_some(key)
        })
        .collect()
}
pub(crate) fn issue_keystone(
    db: &redb::Database,
    templates: &crate::item_templates::ItemTemplates,
    catalog: &crate::merchant::VendorCatalog,
    character: u64,
    challenge: u32,
    level: u32,
) -> Result<(), String> {
    if challenge != 401 || level == 0 {
        return Err("Requires qualified challenge401 and a positive explicit level".into());
    }
    let before = persistence::load_inventory(db, character);
    crate::inventory::grant_item(db, templates, catalog, character, (KEYSTONE_ITEM, 1))?;
    let inventory = persistence::load_inventory(db, character);
    let item = inventory
        .items
        .iter()
        .find(|item| {
            item.item_id == KEYSTONE_ITEM
                && !before
                    .items
                    .iter()
                    .any(|old| old.item_guid == item.item_guid)
        })
        .ok_or("Keystone must have a separate item GUID (stack size1)")?;
    save_key(
        db,
        &KeystoneInfo {
            item_guid: item.item_guid,
            item_id: KEYSTONE_ITEM,
            challenge_id: challenge,
            level,
            affixes: vec![],
        },
    );
    Ok(())
}
fn send_snapshot(world: &mut World, connection: Entity, error: Option<String>) {
    let Some(link) = world.get::<PlayerLink>(connection) else {
        return;
    };
    let player = link.0;
    let Some(character) = world
        .get::<InWorld>(connection)
        .map(|entry| entry.character_id)
    else {
        return;
    };
    let keys = owned_keys(&world.resource::<GameDatabase>().db, character);
    let state = world.resource::<Challenges>();
    let font = state.fonts.get(&connection).map(|font| font.to_bits());
    let run = state
        .slots
        .get(&connection)
        .or_else(|| {
            world
                .get::<InMap>(player)
                .and_then(|map| state.runs.get(map))
        })
        .map(|active| active.run.snapshot.clone());
    if let Some(mut sender) = world.get_mut::<MessageSender<ChallengeSnapshot>>(connection) {
        sender.send::<InstanceChannel>(ChallengeSnapshot {
            font,
            keys,
            run,
            error,
        });
    }
}
fn slot(world: &mut World, connection: Entity, font: u64, guid: u64) -> Result<(), String> {
    let player = world.get::<PlayerLink>(connection).ok_or("No player")?.0;
    let owner = world
        .get::<InWorld>(connection)
        .ok_or("Not in world")?
        .character_id;
    let key = owned_keys(&world.resource::<GameDatabase>().db, owner)
        .into_iter()
        .find(|key| key.item_guid == guid)
        .ok_or("Keystone is not in your bags")?;
    let font = Entity::try_from_bits(font).ok_or("Invalid Font")?;
    let definition = world
        .resource::<Challenges>()
        .definition
        .clone()
        .filter(|d| d.challenge_id == key.challenge_id)
        .ok_or("Keystone dungeon is not qualified")?;
    validate_world_font(world, player, font, &definition)?;
    let map = *world.get::<InMap>(player).ok_or("Player has no map")?;
    if world.resource::<Challenges>().runs.contains_key(&map) {
        return Err("This copy already has a keystone run".into());
    }
    let config = world.resource::<Challenges>().config.clone();
    let run = Run::new(
        persistence::next_id(&world.resource::<GameDatabase>().db, "mythic_run"),
        map.instance_id,
        key,
        definition,
        config,
    );
    world.resource_mut::<Challenges>().slots.insert(
        connection,
        Active {
            owner,
            font,
            run,
            settled: false,
        },
    );
    Ok(())
}
fn validate_world_font(
    world: &World,
    player: Entity,
    font: Entity,
    definition: &ChallengeDefinition,
) -> Result<(), String> {
    let info = world
        .get::<GameObjectInfo>(font)
        .ok_or("Invalid Font object")?;
    let font_map = world.get::<InMap>(font).ok_or("Font has no copy")?;
    let map = world.get::<InMap>(player).ok_or("Player has no copy")?;
    let bounds = world
        .get::<crate::game_objects::GameObjectBounds>(font)
        .ok_or("Font has no interaction bounds")?;
    let font_position = world.get::<Position>(font).ok_or("Font has no position")?;
    let position = world
        .get::<Position>(player)
        .ok_or("Player has no position")?;
    let manager = world.resource::<InstanceManager>();
    let difficulty = manager
        .copy_difficulty(*map)
        .ok_or("Not in a dungeon copy")?
        .difficulty_id;
    let context = FontContext {
        font_type: u32::from(info.go_type),
        player_map: (map.map_id, map.instance_id),
        font_map: (font_map.map_id, font_map.instance_id),
        challenge_map: definition.map_id,
        difficulty,
        reaches: bounds.reaches(
            crate::game_objects::wow_position(font_position),
            crate::game_objects::wow_position(position),
            crate::game_objects::GAME_OBJECT_INTERACT_DISTANCE,
        ),
        alive: world.get::<Health>(player).is_some_and(|h| h.current > 0.0),
        fresh: manager.completed_encounters(*map) == 0,
    };
    validate_font(context).map_err(str::to_owned)
}
fn start(world: &mut World, connection: Entity) -> Result<(), String> {
    let active = world
        .resource::<Challenges>()
        .slots
        .get(&connection)
        .cloned()
        .ok_or("Insert a keystone first")?;
    let player = world.get::<PlayerLink>(connection).ok_or("No player")?.0;
    validate_world_font(world, player, active.font, &active.run.definition)?;
    let key = owned_keys(&world.resource::<GameDatabase>().db, active.owner)
        .into_iter()
        .find(|key| *key == active.run.snapshot.key)
        .ok_or("Slotted keystone changed or left your bags")?;
    let map = *world.get::<InMap>(player).ok_or("No map")?;
    if world.resource::<Challenges>().runs.contains_key(&map) {
        return Err("Another keystone run already owns this copy".into());
    }
    let mut active = active;
    active.run.start(now_ms()).map_err(str::to_owned)?;
    // Deplete at activation, so abandoning or restarting cannot duplicate an upgrade.
    let mut depleted = key;
    depleted.level = depleted.level.saturating_sub(1);
    save_key(&world.resource::<GameDatabase>().db, &depleted);
    let mut state = world.resource_mut::<Challenges>();
    state.slots.remove(&connection);
    state.runs.insert(map, active);
    Ok(())
}
fn receive(world: &mut World) {
    let mut requests = Vec::new();
    for (connection,mut receiver) in world.query_filtered::<(Entity,&mut MessageReceiver<ChallengeRequest>),(With<Authenticated>,With<InWorld>)>().iter_mut(world) {
        requests.extend(receiver.receive().map(|request|(connection,request)));
    }
    for (connection, request) in requests {
        let result = match request {
            ChallengeRequest::Query => Ok(()),
            ChallengeRequest::Slot { font, item_guid } => slot(world, connection, font, item_guid),
            ChallengeRequest::Remove => {
                world.resource_mut::<Challenges>().slots.remove(&connection);
                Ok(())
            }
            ChallengeRequest::Start => start(world, connection),
            ChallengeRequest::Abandon => abandon(world, connection),
        };
        send_snapshot(world, connection, result.err());
    }
}
fn abandon(world: &mut World, connection: Entity) -> Result<(), String> {
    let player = world.get::<PlayerLink>(connection).ok_or("No player")?.0;
    let map = *world.get::<InMap>(player).ok_or("No map")?;
    let owner = world
        .get::<InWorld>(connection)
        .ok_or("No character")?
        .character_id;
    let mut state = world.resource_mut::<Challenges>();
    let active = state.runs.get_mut(&map).ok_or("No challenge run")?;
    if active.owner != owner {
        return Err("Only the keystone owner may abandon this run".into());
    }
    active.run.abandon();
    Ok(())
}
fn update(world: &mut World) {
    receive(world);
    let now = now_ms();
    let mut changes = Vec::new();
    let mut summaries = Vec::new();
    {
        let mut state = world.resource_mut::<Challenges>();
        for (map, active) in &mut state.runs {
            let old = active.run.snapshot.clone();
            active.run.tick(now);
            if old.phase != active.run.snapshot.phase
                || old.elapsed_ms / 1000 != active.run.snapshot.elapsed_ms / 1000
                || old.countdown_ms / 1000 != active.run.snapshot.countdown_ms / 1000
            {
                changes.push(*map);
            }
            if !active.settled && active.run.snapshot.resulting_level.is_some() {
                active.settled = true;
                summaries.push(active.run.snapshot.clone());
            }
        }
    }
    for summary in summaries {
        persist_summary(world, &summary);
    }
    for map in changes {
        send_map(world, map);
    }
}
fn persist_summary(world: &World, summary: &ChallengeRun) {
    let db = &world.resource::<GameDatabase>().db;
    let txn = db.begin_write().expect("begin challenge settlement");
    let mut history = txn.open_table(HISTORY).expect("open run history");
    if history
        .get(summary.run_id)
        .expect("read settled run")
        .is_some()
    {
        return;
    }
    let mut key = summary.key.clone();
    key.level = summary.resulting_level.expect("terminal level");
    key.affixes = world
        .resource::<Challenges>()
        .config
        .unsourced_seasonal_affixes_by_level
        .get(&key.level)
        .cloned()
        .unwrap_or_default();
    txn.open_table(KEYS)
        .expect("open keystones")
        .insert(key.item_guid, bitcode::encode(&key).as_slice())
        .expect("settle key");
    history
        .insert(summary.run_id, bitcode::encode(summary).as_slice())
        .expect("persist summary");
    drop(history);
    txn.commit().expect("commit challenge settlement");
}
fn send_map(world: &mut World, map: InMap) {
    let connections: Vec<_> = world
        .query_filtered::<(Entity, &PlayerLink), (With<Authenticated>, With<InWorld>)>()
        .iter(world)
        .filter(|(_, link)| world.get::<InMap>(link.0) == Some(&map))
        .map(|(entity, _)| entity)
        .collect();
    for connection in connections {
        send_snapshot(world, connection, None);
    }
}
pub(crate) fn record_kill(world: &mut World, npc: Entity) {
    let Some(map) = world.get::<InMap>(npc).copied() else {
        return;
    };
    let Some(entry) = world.get::<Npc>(npc).map(|n| n.template_id) else {
        return;
    };
    let Some(mut state) = world.get_resource_mut::<Challenges>() else {
        return;
    };
    if let Some(active) = state.runs.get_mut(&map) {
        active.run.kill(npc.to_bits(), entry);
    }
    drop(state);
    send_map(world, map);
}
pub(crate) fn record_boss(world: &mut World, map: InMap, encounter: u32) {
    if let Some(mut state) = world.get_resource_mut::<Challenges>() {
        if let Some(active) = state.runs.get_mut(&map) {
            active.run.boss(encounter);
        }
    }
}
pub(crate) fn record_death(world: &mut World, player: Entity) {
    let Some(map) = world.get::<InMap>(player).copied() else {
        return;
    };
    if let Some(mut state) = world.get_resource_mut::<Challenges>() {
        if let Some(active) = state.runs.get_mut(&map) {
            active.run.death();
        }
    }
}

pub(crate) fn open_font(world: &mut World, connection: Entity, font: Entity) {
    let result = (|| {
        let player = world.get::<PlayerLink>(connection).ok_or("No player")?.0;
        let definition = world
            .resource::<Challenges>()
            .definition
            .as_ref()
            .ok_or("No qualified keystone dungeon")?;
        validate_world_font(world, player, font, definition)?;
        world
            .resource_mut::<Challenges>()
            .fonts
            .insert(connection, font);
        Ok::<(), String>(())
    })();
    send_snapshot(world, connection, result.err());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mythic_bag_slot_activation_and_persistent_settlement() {
        let path = std::env::var("MYTHIC_PLUS_WORLD_COPY").expect("private imported copy");
        let conn =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let definition = load_azure(&conn).unwrap().unwrap();
        let store = crate::maps::store::load_map_store(&conn).unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut h = crate::bank_test_harness::harness_at(file.path(), "Keymaster").unwrap();
        let player = h.player;
        let connection = h.connection;
        let owner = h.character_id;
        let mut manager = InstanceManager::default();
        let map = manager
            .create_map(
                &store,
                2515,
                crate::maps::instances::InstanceEntrant {
                    character_id: owner,
                    group: None,
                    leader: owner,
                    difficulty_id: 8,
                },
                crate::maps::instances::Now {
                    unix: 1791705600,
                    elapsed: 0.0,
                },
            )
            .unwrap()
            .map;
        h.app.world_mut().insert_resource(store);
        h.app.world_mut().insert_resource(manager);
        h.app.world_mut().entity_mut(player).insert(map);
        h.app.world_mut().insert_resource(Challenges {
            definition: Some(definition.clone()),
            ..Default::default()
        });
        let txn = h.db().begin_write().unwrap();
        txn.open_table(KEYS).unwrap();
        txn.open_table(HISTORY).unwrap();
        txn.commit().unwrap();
        let mut inventory = h.inventory();
        inventory.items.push(persistence::InventoryItemData {
            item_guid: 4242,
            item_id: 180653,
            name: "Mythic Keystone".into(),
            quality: 4,
            required_level: 0,
            stack_count: 1,
            vendor_sell_price: 0,
            unique_auction: false,
        });
        persistence::save_inventory(h.db(), owner, &inventory);
        let key = KeystoneInfo {
            item_guid: 4242,
            item_id: 180653,
            challenge_id: 401,
            level: 8,
            affixes: vec![],
        };
        save_key(h.db(), &key);
        let font = h
            .app
            .world_mut()
            .spawn((
                map,
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                GameObjectInfo {
                    entry: 246779,
                    go_type: 49,
                    display_id: 0,
                    name: "Font of Power".into(),
                    scale: 1.0,
                },
                crate::game_objects::GameObjectBounds {
                    min: Vec3::ZERO,
                    max: Vec3::ONE,
                    scale: 1.0,
                    orientation: 0.0,
                },
            ))
            .id();
        assert!(slot(h.app.world_mut(), connection, font.to_bits(), 999).is_err());
        slot(h.app.world_mut(), connection, font.to_bits(), 4242).unwrap();
        h.app.world_mut().get_mut::<Position>(player).unwrap().x = 100.0;
        assert!(start(h.app.world_mut(), connection).is_err());
        assert_eq!(owned_keys(h.db(), owner)[0].level, 8);
        h.app.world_mut().get_mut::<Position>(player).unwrap().x = 0.0;
        start(h.app.world_mut(), connection).unwrap();
        assert_eq!(owned_keys(h.db(), owner)[0].level, 7);
        assert!(start(h.app.world_mut(), connection).is_err());
        let summary = {
            let mut state = h.app.world_mut().resource_mut::<Challenges>();
            let active = state.runs.get_mut(&map).unwrap();
            for spawn in 0..82 {
                active.run.kill(spawn, 187160);
            }
            for boss in [2582, 2585, 2583, 2584] {
                active.run.boss(boss);
            }
            active.run.tick(now_ms());
            active.run.snapshot.clone()
        };
        assert_eq!(summary.phase, ChallengePhase::Completed);
        persist_summary(h.app.world(), &summary);
        persist_summary(h.app.world(), &summary);
        assert_eq!(owned_keys(h.db(), owner)[0].level, 11);
        {
            let txn = h.db().begin_read().unwrap();
            let table = txn.open_table(HISTORY).unwrap();
            let row = table.get(summary.run_id).unwrap().unwrap();
            assert_eq!(
                bitcode::decode::<ChallengeRun>(row.value()).unwrap(),
                summary
            );
        }
        drop(h);
        let reopened = redb::Database::open(file.path()).unwrap();
        assert_eq!(owned_keys(&reopened, owner)[0].level, 11);
    }
}
