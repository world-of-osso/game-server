use std::collections::HashMap;

use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::Connection;
use shared::components::{EquipmentAppearance, EquipmentVisualSlot, Player};
use shared::protocol::{
    DurabilityChannel, DurabilitySlotSnapshot, DurabilitySnapshot, DurabilityStateUpdate,
    QueryDurabilityStatus,
};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{self, CharacterData, ClientId, DurabilityEntryData, GameDatabase};

#[cfg(test)]
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct EquippedDurability {
    pub slots: Vec<DurabilitySlotState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurabilitySlotState {
    pub slot: EquipmentVisualSlot,
    /// The equipped item this durability belongs to.
    pub item_id: u32,
    pub current: u32,
    pub max: u32,
}

/// Durability of one item: AzerothCore `item_template.MaxDurability` (joined by
/// `content_item.SourceEntry`) and the copper per lost point, AzerothCore
/// `Player::DurabilityRepair` (Player.cpp:4752): `DurabilityCosts[ItemLevel]`
/// multiplier for the item's class/subclass times `DurabilityQuality[(Quality+1)*2]`,
/// from the Retail DB2s with Retail item levels. `None` when a DB2 row is missing,
/// which makes the item unrepairable as in AzerothCore.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemDurabilityInfo {
    pub max: u32,
    pub cost_per_point: Option<f64>,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ItemDurability(pub HashMap<u32, ItemDurabilityInfo>);

impl ItemDurability {
    fn info(&self, item_id: u32) -> Option<ItemDurabilityInfo> {
        self.0.get(&item_id).copied().filter(|info| info.max > 0)
    }

    /// `MaxDurability` of a new item; `None` for items without durability.
    pub fn max_durability(&self, item_id: u32) -> Option<u32> {
        self.info(item_id).map(|info| info.max)
    }

    /// Copper to restore `lost` points (at least 1, AzerothCore `if (costs == 0) costs = 1`);
    /// `None` if the item can't be repaired.
    pub fn repair_cost(&self, item_id: u32, lost: u32) -> Option<u32> {
        if lost == 0 {
            return Some(0);
        }
        let rate = self.info(item_id)?.cost_per_point?;
        Some(((f64::from(lost) * rate) as u32).max(1))
    }
}

/// AzerothCore `ItemSubClassToDurabilityMultiplierId` (ItemTemplate.h:557):
/// weapons use their subclass, armor 21 + subclass, anything else 0.
fn cost_column(class_id: i64, subclass_id: i64) -> usize {
    match class_id {
        2 => subclass_id as usize,
        4 => 21 + subclass_id as usize,
        _ => 0,
    }
}

pub fn load_item_durability(conn: &Connection) -> rusqlite::Result<ItemDurability> {
    let mut costs: HashMap<i64, Vec<i64>> = HashMap::new();
    let mut statement = conn.prepare("SELECT * FROM durability_costs")?;
    let columns = statement.column_count();
    let rows = statement.query_map([], |row| {
        let multipliers = (1..columns)
            .map(|column| row.get::<_, i64>(column))
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok((row.get::<_, i64>(0)?, multipliers))
    })?;
    for row in rows {
        let (item_level, multipliers) = row?;
        costs.insert(item_level, multipliers);
    }
    let quality: HashMap<i64, f64> = conn
        .prepare("SELECT ID, Data FROM durability_quality")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut statement = conn.prepare(
        "SELECT c.ID, t.MaxDurability, c.ItemLevel, c.Quality, c.ClassID, c.SubclassID \
         FROM content_item c JOIN item_template t ON t.entry = c.SourceEntry \
         WHERE t.MaxDurability > 0",
    )?;
    let items = statement.query_map([], |row| {
        let item_level: i64 = row.get(2)?;
        let item_quality: i64 = row.get(3)?;
        let column = cost_column(row.get(4)?, row.get(5)?);
        let multiplier = costs
            .get(&item_level)
            .and_then(|row_costs| row_costs.get(column))
            .copied();
        let quality_mod = quality.get(&((item_quality + 1) * 2)).copied();
        let info = ItemDurabilityInfo {
            max: row.get(1)?,
            cost_per_point: multiplier.zip(quality_mod).map(|(m, q)| m as f64 * q),
        };
        Ok((row.get::<_, u32>(0)?, info))
    })?;
    Ok(ItemDurability(items.collect::<rusqlite::Result<_>>()?))
}

pub struct DurabilityPlugin;

#[cfg(test)]
static CHANGED_DURABILITY_PLAYERS_VISITED: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static CHANGED_DURABILITY_TEST_LOCK: Mutex<()> = Mutex::new(());

impl Plugin for DurabilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemDurability>();
        app.add_systems(
            Update,
            (
                sync_durability_from_equipment_appearance,
                send_initial_durability_snapshot,
            )
                .chain(),
        );
        app.add_systems(Update, process_durability_queries);
        app.add_systems(Update, persist_changed_durability);
        app.add_systems(Update, send_changed_durability_snapshot);
    }
}

type NewInWorldLink<'w, 's> =
    Query<'w, 's, (Entity, &'static PlayerLink), (With<Authenticated>, Added<InWorld>)>;
type DurabilityAppearanceQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static EquipmentAppearance,
        &'static mut EquippedDurability,
        Option<&'static ClientId>,
    ),
    (With<Player>, Changed<EquipmentAppearance>),
>;
type ChangedDurabilityQuery<'w, 's> = Query<
    'w,
    's,
    (&'static ClientId, &'static EquippedDurability),
    (With<Player>, Changed<EquippedDurability>),
>;
type ChangedDurabilitySnapshotQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, Ref<'static, EquippedDurability>),
    (With<Player>, Changed<EquippedDurability>),
>;

/// Durability of the equipped items at login: saved current values, full for
/// items without a saved value.
pub fn load_equipped_durability(
    data: &CharacterData,
    rules: &ItemDurability,
) -> EquippedDurability {
    let saved = |slot, _item_id| {
        data.durability
            .iter()
            .find(|entry| entry.slot == slot)
            .map(|entry| entry.current)
    };
    EquippedDurability {
        slots: durability_slots(&data.equipment_appearance, rules, saved),
    }
}

/// One slot per equipped item with a max durability, `current` from `previous`
/// (capped at max) or full.
fn durability_slots(
    appearance: &EquipmentAppearance,
    rules: &ItemDurability,
    previous: impl Fn(EquipmentVisualSlot, u32) -> Option<u32>,
) -> Vec<DurabilitySlotState> {
    let mut slots: Vec<DurabilitySlotState> = appearance
        .entries
        .iter()
        .filter_map(|entry| {
            let item_id = entry.item_id?;
            let max = rules.info(item_id)?.max;
            let current = previous(entry.slot, item_id).map_or(max, |current| current.min(max));
            Some(DurabilitySlotState {
                slot: entry.slot,
                item_id,
                current,
                max,
            })
        })
        .collect();
    slots.sort_by_key(|entry| slot_sort_key(entry.slot));
    slots.dedup_by_key(|entry| entry.slot);
    slots
}

/// Insert `EquippedDurability` on a spawning player once `ItemDurability` is at hand.
pub fn insert_equipped_durability(commands: &mut Commands, player: Entity, data: &CharacterData) {
    let data = data.clone();
    commands
        .entity(player)
        .queue(move |mut entity: EntityWorldMut| {
            let durability = entity
                .world()
                .get_resource::<ItemDurability>()
                .map(|rules| load_equipped_durability(&data, rules))
                .unwrap_or_default();
            entity.insert(durability);
        });
}

pub fn apply_death_loss(durability: &mut EquippedDurability) -> bool {
    apply_fractional_loss(durability, 0.10)
}

pub fn apply_spirit_healer_loss(durability: &mut EquippedDurability) -> bool {
    apply_fractional_loss(durability, 0.25)
}

pub fn build_snapshot(
    durability: &EquippedDurability,
    rules: &ItemDurability,
) -> DurabilitySnapshot {
    let slots = durability
        .slots
        .iter()
        .map(|entry| DurabilitySlotSnapshot {
            slot: entry.slot,
            current: entry.current,
            max: entry.max,
            repair_cost: rules
                .repair_cost(entry.item_id, entry.max - entry.current)
                .unwrap_or(0),
        })
        .collect::<Vec<_>>();
    let total_repair_cost = slots.iter().map(|entry| entry.repair_cost).sum();
    DurabilitySnapshot {
        total_repair_cost,
        slots,
    }
}

fn send_initial_durability_snapshot(
    connections: NewInWorldLink,
    durability: Query<&EquippedDurability>,
    mut senders: Query<&mut MessageSender<DurabilityStateUpdate>>,
    rules: Res<ItemDurability>,
) {
    for (conn_entity, link) in &connections {
        let Ok(durability) = durability.get(link.0) else {
            send_error(
                conn_entity,
                &mut senders,
                "durability unavailable for player",
            );
            continue;
        };
        send_update(
            conn_entity,
            build_snapshot(durability, &rules),
            "durability loaded",
            &mut senders,
        );
    }
}

fn process_durability_queries(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &mut MessageReceiver<QueryDurabilityStatus>,
        ),
        With<Authenticated>,
    >,
    durability: Query<&EquippedDurability>,
    mut senders: Query<&mut MessageSender<DurabilityStateUpdate>>,
    rules: Res<ItemDurability>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Ok(durability) = durability.get(link.0) else {
            send_error(
                conn_entity,
                &mut senders,
                "durability unavailable for player",
            );
            continue;
        };
        send_update(
            conn_entity,
            build_snapshot(durability, &rules),
            "durability refreshed",
            &mut senders,
        );
    }
}

/// New items start full; equipment moves restore the inventory's saved item damage.
fn sync_durability_from_equipment_appearance(
    mut players: DurabilityAppearanceQuery,
    rules: Res<ItemDurability>,
    db: Option<Res<GameDatabase>>,
) {
    for (appearance, mut durability, character_id) in &mut players {
        let saved = db
            .as_ref()
            .zip(character_id)
            .and_then(|(db, id)| persistence::load_character(&db.db, id.0));
        let next = durability_slots(appearance, &rules, |slot, item_id| {
            if let Some(data) = &saved {
                return data
                    .durability
                    .iter()
                    .find(|entry| entry.slot == slot)
                    .map(|entry| entry.current);
            }
            durability
                .slots
                .iter()
                .find(|entry| entry.slot == slot && entry.item_id == item_id)
                .map(|entry| entry.current)
        });
        if durability.slots != next {
            durability.slots = next;
        }
    }
}

fn persist_changed_durability(db: Option<Res<GameDatabase>>, players: ChangedDurabilityQuery) {
    let Some(db) = db.as_ref() else { return };
    for (character_id, durability) in &players {
        enqueue_durability(db, character_id.0, saved_entries(durability));
    }
}

fn enqueue_durability(db: &GameDatabase, character: u64, entries: Vec<DurabilityEntryData>) {
    db.writer.enqueue("character durability", move |db| {
        let mut data = persistence::load_character(db, character)
            .ok_or_else(|| format!("durability character {character} no longer exists"))?;
        data.durability = entries;
        persistence::save_character(db, character, &data);
        Ok(())
    });
}

fn saved_entries(durability: &EquippedDurability) -> Vec<DurabilityEntryData> {
    durability
        .slots
        .iter()
        .map(|entry| DurabilityEntryData {
            slot: entry.slot,
            current: entry.current,
            max: entry.max,
        })
        .collect()
}

/// Admin `set-durability`: every equipped item to `percent` of its max, on the live
/// player (persisted by `persist_changed_durability`) or in the saved character.
pub fn set_durability_percent(world: &mut World, character_id: u64, percent: u32) {
    let scale = |durability: &mut EquippedDurability| {
        for entry in &mut durability.slots {
            entry.current = entry.max * percent / 100;
        }
    };
    let mut players = world.query::<(&ClientId, &mut EquippedDurability)>();
    let mut live = false;
    for (id, mut durability) in players.iter_mut(world) {
        if id.0 == character_id {
            scale(&mut durability);
            live = true;
        }
    }
    if live {
        return;
    }
    let (Some(db), Some(rules)) = (
        world.get_resource::<GameDatabase>(),
        world.get_resource::<ItemDurability>(),
    ) else {
        return;
    };
    let Some(data) = persistence::load_character(&db.db, character_id) else {
        return;
    };
    let mut durability = load_equipped_durability(&data, rules);
    scale(&mut durability);
    enqueue_durability(db, character_id, saved_entries(&durability));
}

fn send_changed_durability_snapshot(
    players: ChangedDurabilitySnapshotQuery,
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    mut senders: Query<&mut MessageSender<DurabilityStateUpdate>>,
    rules: Res<ItemDurability>,
) {
    for (player_entity, durability) in &players {
        #[cfg(test)]
        CHANGED_DURABILITY_PLAYERS_VISITED.fetch_add(1, Ordering::SeqCst);

        if durability.is_added() {
            continue;
        }
        let Some(conn_entity) = connection_for_player(player_entity, &connections) else {
            continue;
        };
        send_update(
            conn_entity,
            build_snapshot(&durability, &rules),
            "durability updated",
            &mut senders,
        );
    }
}

fn connection_for_player(
    player_entity: Entity,
    connections: &Query<(Entity, &PlayerLink), With<Authenticated>>,
) -> Option<Entity> {
    connections
        .iter()
        .find_map(|(conn_entity, link)| (link.0 == player_entity).then_some(conn_entity))
}

fn apply_fractional_loss(durability: &mut EquippedDurability, fraction: f32) -> bool {
    let mut changed = false;
    for entry in &mut durability.slots {
        if entry.max == 0 {
            continue;
        }
        let loss = ((entry.max as f32) * fraction).ceil() as u32;
        let next = entry.current.saturating_sub(loss);
        if next != entry.current {
            entry.current = next;
            changed = true;
        }
    }
    changed
}

fn send_update(
    conn_entity: Entity,
    snapshot: DurabilitySnapshot,
    message: &str,
    senders: &mut Query<&mut MessageSender<DurabilityStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<DurabilityChannel>(DurabilityStateUpdate {
        snapshot: Some(snapshot),
        message: Some(message.into()),
        error: None,
    });
}

fn send_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<DurabilityStateUpdate>>,
    error: impl Into<String>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<DurabilityChannel>(DurabilityStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

fn slot_sort_key(slot: EquipmentVisualSlot) -> u8 {
    match slot {
        EquipmentVisualSlot::Head => 0,
        EquipmentVisualSlot::Shoulder => 1,
        EquipmentVisualSlot::Back => 2,
        EquipmentVisualSlot::Chest => 3,
        EquipmentVisualSlot::Shirt => 4,
        EquipmentVisualSlot::Tabard => 5,
        EquipmentVisualSlot::Wrist => 6,
        EquipmentVisualSlot::Hands => 7,
        EquipmentVisualSlot::Waist => 8,
        EquipmentVisualSlot::Legs => 9,
        EquipmentVisualSlot::Feet => 10,
        EquipmentVisualSlot::MainHand => 11,
        EquipmentVisualSlot::OffHand => 12,
        EquipmentVisualSlot::Ranged => 13,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lightyear::connection::host::HostClient;
    use lightyear::prelude::{AppMessageExt, Connected, PeerId, RemoteId};
    use shared::components::EquippedAppearanceEntry;

    use super::*;

    #[test]
    fn durability_save_enqueues_without_waiting_and_preserves_earlier_fifo_fields() {
        use std::time::Instant;
        let db = persistence::temp_database();
        let (character, _) = persistence::create_character(&db, 1, "Fifodamage", 1, 1).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(GameDatabase::new(db));
        app.add_systems(Update, persist_changed_durability);
        app.world_mut().spawn((
            Player {
                name: "Fifodamage".into(),
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
            ClientId(character),
            EquippedDurability {
                slots: vec![slot(
                    EquipmentVisualSlot::Chest,
                    TARNISHED_CHAIN_VEST,
                    17,
                    55,
                )],
            },
        ));
        let db = app.world().resource::<GameDatabase>();
        let held = db.db.begin_write().unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            held.abort().unwrap();
        });
        db.writer
            .enqueue("preceding character movement", move |db| {
                let mut data = persistence::load_character(db, character).unwrap();
                data.x = 42.0;
                persistence::save_character(db, character, &data);
                Ok(())
            });
        let started = Instant::now();
        app.update();
        let elapsed = started.elapsed();
        release.join().unwrap();
        assert!(
            elapsed < Duration::from_millis(50),
            "durability tick waited: {elapsed:?}"
        );
        let fence = app
            .world()
            .resource::<GameDatabase>()
            .writer
            .enqueue("durability fixture fence", |_| Ok(()));
        let deadline = Instant::now() + Duration::from_secs(3);
        while fence.result().is_none() {
            assert!(Instant::now() < deadline, "durability fence timeout");
            std::thread::sleep(Duration::from_millis(1));
        }
        fence.result().unwrap().unwrap();
        let data =
            persistence::load_character(&app.world().resource::<GameDatabase>().db, character)
                .unwrap();
        assert_eq!(data.x, 42.0);
        assert_eq!(data.durability[0].current, 17);
    }

    fn setup_changed_snapshot_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.add_plugins(lightyear::prelude::server::ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.register_message::<DurabilityStateUpdate>();
        app.insert_resource(starter_rules());
        app.add_systems(Update, send_changed_durability_snapshot);
        app.finish();
        app.cleanup();
        app
    }

    fn spawn_player(app: &mut App, durability: EquippedDurability) -> Entity {
        app.world_mut()
            .spawn((
                Player {
                    name: "Elara".into(),
                    race: 1,
                    class: 2,
                    appearance: shared::components::CharacterAppearance::default(),
                },
                durability,
            ))
            .id()
    }

    fn spawn_connection(app: &mut App, peer_id: u64, player: Entity) -> Entity {
        app.world_mut()
            .spawn((
                RemoteId(PeerId::Local(peer_id)),
                Connected,
                HostClient { buffer: Vec::new() },
                Authenticated {
                    account_id: peer_id,
                },
                PlayerLink(player),
                MessageSender::<DurabilityStateUpdate>::default(),
                MessageReceiver::<DurabilityStateUpdate>::default(),
            ))
            .id()
    }

    fn receive_updates(app: &mut App, connection: Entity) -> Vec<DurabilityStateUpdate> {
        app.world_mut()
            .entity_mut(connection)
            .get_mut::<MessageReceiver<DurabilityStateUpdate>>()
            .expect("connection should have a durability update receiver")
            .receive()
            .collect()
    }

    const TARNISHED_CHAIN_VEST: u32 = 2379;
    const LARGE_ROUND_SHIELD: u32 = 2129;

    /// AzerothCore MaxDurability 55 and 35; Retail item level 2, quality 1, mail
    /// chest and shield: DurabilityCosts[2] armor multiplier 1 times
    /// DurabilityQuality[4] 0.8 copper per point.
    fn starter_rules() -> ItemDurability {
        let info = |max| ItemDurabilityInfo {
            max,
            cost_per_point: Some(0.800_000_011_92),
        };
        ItemDurability(
            [
                (TARNISHED_CHAIN_VEST, info(55)),
                (LARGE_ROUND_SHIELD, info(35)),
            ]
            .into_iter()
            .collect(),
        )
    }

    fn slot(
        slot: EquipmentVisualSlot,
        item_id: u32,
        current: u32,
        max: u32,
    ) -> DurabilitySlotState {
        DurabilitySlotState {
            slot,
            item_id,
            current,
            max,
        }
    }

    fn sample_durability() -> EquippedDurability {
        EquippedDurability {
            slots: vec![
                slot(EquipmentVisualSlot::Chest, TARNISHED_CHAIN_VEST, 55, 55),
                slot(EquipmentVisualSlot::OffHand, LARGE_ROUND_SHIELD, 35, 35),
            ],
        }
    }

    #[test]
    fn unchanged_durability_player_is_not_visited_or_updated() {
        let _test_lock = CHANGED_DURABILITY_TEST_LOCK.lock().unwrap();
        CHANGED_DURABILITY_PLAYERS_VISITED.store(0, Ordering::SeqCst);

        let mut app = setup_changed_snapshot_app();
        let player = spawn_player(&mut app, sample_durability());
        let connection = spawn_connection(&mut app, 1, player);

        app.update();
        assert!(receive_updates(&mut app, connection).is_empty());

        CHANGED_DURABILITY_PLAYERS_VISITED.store(0, Ordering::SeqCst);
        app.update();

        assert_eq!(CHANGED_DURABILITY_PLAYERS_VISITED.load(Ordering::SeqCst), 0);
        assert!(receive_updates(&mut app, connection).is_empty());
    }

    #[test]
    fn newly_added_durability_is_excluded_from_changed_snapshot() {
        let _test_lock = CHANGED_DURABILITY_TEST_LOCK.lock().unwrap();
        CHANGED_DURABILITY_PLAYERS_VISITED.store(0, Ordering::SeqCst);

        let mut app = setup_changed_snapshot_app();
        app.update();
        let player = spawn_player(&mut app, sample_durability());
        let connection = spawn_connection(&mut app, 1, player);

        app.update();

        assert_eq!(CHANGED_DURABILITY_PLAYERS_VISITED.load(Ordering::SeqCst), 1);
        assert!(receive_updates(&mut app, connection).is_empty());
    }

    #[test]
    fn changed_durability_routes_one_snapshot_with_repair_costs() {
        let _test_lock = CHANGED_DURABILITY_TEST_LOCK.lock().unwrap();
        CHANGED_DURABILITY_PLAYERS_VISITED.store(0, Ordering::SeqCst);

        let mut app = setup_changed_snapshot_app();
        let player = spawn_player(&mut app, sample_durability());
        let connection = spawn_connection(&mut app, 1, player);
        let other_player = app
            .world_mut()
            .spawn(Player {
                name: "Other".into(),
                race: 1,
                class: 2,
                appearance: shared::components::CharacterAppearance::default(),
            })
            .id();
        let other_connection = spawn_connection(&mut app, 2, other_player);

        app.update();
        assert!(receive_updates(&mut app, connection).is_empty());
        assert!(receive_updates(&mut app, other_connection).is_empty());

        app.world_mut()
            .entity_mut(player)
            .get_mut::<EquippedDurability>()
            .expect("player should have durability")
            .slots[0]
            .current = 44;
        CHANGED_DURABILITY_PLAYERS_VISITED.store(0, Ordering::SeqCst);

        app.update();

        assert_eq!(CHANGED_DURABILITY_PLAYERS_VISITED.load(Ordering::SeqCst), 1);
        let updates = receive_updates(&mut app, connection);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].message.as_deref(), Some("durability updated"));
        assert_eq!(updates[0].error, None);
        let snapshot = updates[0]
            .snapshot
            .as_ref()
            .expect("durability update should include a snapshot");
        // 11 lost points * 0.8 = 8.8 -> 8 copper.
        assert_eq!(snapshot.total_repair_cost, 8);
        assert_eq!(snapshot.slots.len(), 2);
        assert_eq!(snapshot.slots[0].slot, EquipmentVisualSlot::Chest);
        assert_eq!(snapshot.slots[0].current, 44);
        assert_eq!(snapshot.slots[0].max, 55);
        assert_eq!(snapshot.slots[0].repair_cost, 8);
        assert_eq!(snapshot.slots[1].slot, EquipmentVisualSlot::OffHand);
        assert_eq!(snapshot.slots[1].repair_cost, 0);
        assert!(receive_updates(&mut app, other_connection).is_empty());
    }

    fn entry(slot: EquipmentVisualSlot, item_id: Option<u32>) -> EquippedAppearanceEntry {
        EquippedAppearanceEntry {
            slot,
            item_id,
            definition_source: item_id.map(|_| shared::item_data::ItemDefinitionSource::Retail),
            display_info_id: None,
            inventory_type: 0,
            hidden: false,
        }
    }

    #[test]
    fn items_without_max_durability_get_no_slot_and_saved_values_survive_login() {
        let data = CharacterData {
            name: "Alice".into(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            health: 100.0,
            max_health: 100.0,
            race: 1,
            class: 1,
            level: 1,
            appearance: shared::components::CharacterAppearance::default(),
            equipment_appearance: EquipmentAppearance {
                entries: vec![
                    entry(EquipmentVisualSlot::Chest, Some(TARNISHED_CHAIN_VEST)),
                    // Recruit's Boots (40): MaxDurability 0.
                    entry(EquipmentVisualSlot::Feet, Some(40)),
                    entry(EquipmentVisualSlot::OffHand, Some(LARGE_ROUND_SHIELD)),
                    entry(EquipmentVisualSlot::Shirt, None),
                ],
            },
            durability: vec![DurabilityEntryData {
                slot: EquipmentVisualSlot::Chest,
                current: 30,
                max: 55,
            }],
        };

        let durability = load_equipped_durability(&data, &starter_rules());

        assert_eq!(
            durability.slots,
            vec![
                slot(EquipmentVisualSlot::Chest, TARNISHED_CHAIN_VEST, 30, 55),
                slot(EquipmentVisualSlot::OffHand, LARGE_ROUND_SHIELD, 35, 35),
            ]
        );
    }

    #[test]
    fn death_loss_takes_ten_percent_rounded_up() {
        let mut durability = sample_durability();
        assert!(apply_death_loss(&mut durability));
        assert_eq!(durability.slots[0].current, 49);
        assert_eq!(durability.slots[1].current, 31);
    }

    #[test]
    fn repair_costs_follow_the_azerothcore_formula_with_a_one_copper_minimum() {
        let rules = starter_rules();
        assert_eq!(rules.repair_cost(TARNISHED_CHAIN_VEST, 0), Some(0));
        assert_eq!(rules.repair_cost(TARNISHED_CHAIN_VEST, 1), Some(1));
        assert_eq!(rules.repair_cost(TARNISHED_CHAIN_VEST, 55), Some(44));
        assert_eq!(rules.repair_cost(40, 5), None);
        let snapshot = build_snapshot(
            &EquippedDurability {
                slots: vec![
                    slot(EquipmentVisualSlot::Chest, TARNISHED_CHAIN_VEST, 0, 55),
                    slot(EquipmentVisualSlot::OffHand, LARGE_ROUND_SHIELD, 25, 35),
                ],
            },
            &rules,
        );
        assert_eq!(snapshot.slots[1].repair_cost, 8);
        assert_eq!(snapshot.total_repair_cost, 52);
    }

    #[test]
    fn world_db_costs_match_the_retail_db2_rows() {
        let path = crate::test_data::world_db_path();
        let conn =
            Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let rules = load_item_durability(&conn).unwrap();
        assert_eq!(rules.info(TARNISHED_CHAIN_VEST).unwrap().max, 55);
        assert_eq!(rules.info(LARGE_ROUND_SHIELD).unwrap().max, 35);
        assert_eq!(rules.repair_cost(TARNISHED_CHAIN_VEST, 11), Some(8));
        // Worn Shortsword (25): one-handed sword, Retail item level 1, max 20.
        let sword = rules.info(25).unwrap();
        assert_eq!(sword.max, 20);
        assert!(rules.info(40).is_none());
    }
}
