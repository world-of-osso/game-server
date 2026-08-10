use bevy::prelude::*;
use lightyear::prelude::*;
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
    pub current: u32,
    pub max: u32,
}

pub struct DurabilityPlugin;

#[cfg(test)]
static CHANGED_DURABILITY_PLAYERS_VISITED: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static CHANGED_DURABILITY_TEST_LOCK: Mutex<()> = Mutex::new(());

impl Plugin for DurabilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, send_initial_durability_snapshot);
        app.add_systems(Update, process_durability_queries);
        app.add_systems(Update, sync_durability_from_equipment_appearance);
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

pub fn load_equipped_durability(data: &CharacterData) -> EquippedDurability {
    let mut slots = active_durability_slots(&data.equipment_appearance)
        .into_iter()
        .map(|slot| {
            let (max, _) = slot_profile(slot);
            let current = data
                .durability
                .iter()
                .find(|entry| entry.slot == slot)
                .map(|entry| entry.current.min(max))
                .unwrap_or(max);
            DurabilitySlotState { slot, current, max }
        })
        .collect::<Vec<_>>();
    slots.sort_by_key(|entry| slot_sort_key(entry.slot));
    EquippedDurability { slots }
}

pub fn apply_death_loss(durability: &mut EquippedDurability) -> bool {
    apply_fractional_loss(durability, 0.10)
}

pub fn apply_spirit_healer_loss(durability: &mut EquippedDurability) -> bool {
    apply_fractional_loss(durability, 0.25)
}

pub fn build_snapshot(durability: &EquippedDurability) -> DurabilitySnapshot {
    let slots = durability
        .slots
        .iter()
        .map(|entry| DurabilitySlotSnapshot {
            slot: entry.slot,
            current: entry.current,
            max: entry.max,
            repair_cost: repair_cost_for_slot(entry.slot, entry.current, entry.max),
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
            durability,
            Some("durability loaded".into()),
            None,
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
            durability,
            Some("durability refreshed".into()),
            None,
            &mut senders,
        );
    }
}

fn sync_durability_from_equipment_appearance(mut players: DurabilityAppearanceQuery) {
    for (appearance, mut durability) in &mut players {
        let desired_slots = active_durability_slots(appearance);
        let mut next = desired_slots
            .into_iter()
            .map(|slot| {
                let (max, _) = slot_profile(slot);
                let current = durability
                    .slots
                    .iter()
                    .find(|entry| entry.slot == slot)
                    .map(|entry| entry.current.min(max))
                    .unwrap_or(max);
                DurabilitySlotState { slot, current, max }
            })
            .collect::<Vec<_>>();
        next.sort_by_key(|entry| slot_sort_key(entry.slot));
        if durability.slots != next {
            durability.slots = next;
        }
    }
}

fn persist_changed_durability(db: Option<Res<GameDatabase>>, players: ChangedDurabilityQuery) {
    let Some(db) = db.as_ref() else { return };
    for (character_id, durability) in &players {
        let Some(mut data) = persistence::load_character(&db.db, character_id.0) else {
            continue;
        };
        data.durability = durability
            .slots
            .iter()
            .map(|entry| DurabilityEntryData {
                slot: entry.slot,
                current: entry.current,
                max: entry.max,
            })
            .collect();
        persistence::save_character(&db.db, character_id.0, &data);
    }
}

fn send_changed_durability_snapshot(
    players: ChangedDurabilitySnapshotQuery,
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    mut senders: Query<&mut MessageSender<DurabilityStateUpdate>>,
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
            &durability,
            Some("durability updated".into()),
            None,
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

fn active_durability_slots(appearance: &EquipmentAppearance) -> Vec<EquipmentVisualSlot> {
    let mut slots = appearance
        .entries
        .iter()
        .map(|entry| entry.slot)
        .collect::<Vec<_>>();
    slots.sort_by_key(|slot| slot_sort_key(*slot));
    slots.dedup();
    slots
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
    durability: &EquippedDurability,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<DurabilityStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<DurabilityChannel>(DurabilityStateUpdate {
        snapshot: Some(build_snapshot(durability)),
        message,
        error,
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

fn repair_cost_for_slot(slot: EquipmentVisualSlot, current: u32, max: u32) -> u32 {
    let missing = max.saturating_sub(current);
    if missing == 0 {
        return 0;
    }
    let (_, cost_per_point) = slot_profile(slot);
    missing.saturating_mul(cost_per_point)
}

fn slot_profile(slot: EquipmentVisualSlot) -> (u32, u32) {
    match slot {
        EquipmentVisualSlot::Head => (70, 15),
        EquipmentVisualSlot::Shoulder => (60, 12),
        EquipmentVisualSlot::Back => (40, 8),
        EquipmentVisualSlot::Chest => (80, 18),
        EquipmentVisualSlot::Shirt => (0, 0),
        EquipmentVisualSlot::Tabard => (0, 0),
        EquipmentVisualSlot::Wrist => (50, 10),
        EquipmentVisualSlot::Hands => (60, 12),
        EquipmentVisualSlot::Waist => (60, 12),
        EquipmentVisualSlot::Legs => (75, 16),
        EquipmentVisualSlot::Feet => (60, 12),
        EquipmentVisualSlot::MainHand => (90, 20),
        EquipmentVisualSlot::OffHand => (80, 18),
    }
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
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lightyear::connection::host::HostClient;
    use lightyear::prelude::{AppMessageExt, Connected, PeerId, RemoteId};
    use shared::components::EquippedAppearanceEntry;

    use super::*;

    fn setup_changed_snapshot_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(lightyear::prelude::server::ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.register_message::<DurabilityStateUpdate>();
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

    fn sample_durability() -> EquippedDurability {
        EquippedDurability {
            slots: vec![
                DurabilitySlotState {
                    slot: EquipmentVisualSlot::Head,
                    current: 70,
                    max: 70,
                },
                DurabilitySlotState {
                    slot: EquipmentVisualSlot::Chest,
                    current: 80,
                    max: 80,
                },
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
            .current = 60;
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
        assert_eq!(snapshot.total_repair_cost, 150);
        assert_eq!(snapshot.slots.len(), 2);
        assert_eq!(snapshot.slots[0].slot, EquipmentVisualSlot::Head);
        assert_eq!(snapshot.slots[0].current, 60);
        assert_eq!(snapshot.slots[0].max, 70);
        assert_eq!(snapshot.slots[0].repair_cost, 150);
        assert_eq!(snapshot.slots[1].slot, EquipmentVisualSlot::Chest);
        assert_eq!(snapshot.slots[1].current, 80);
        assert_eq!(snapshot.slots[1].max, 80);
        assert_eq!(snapshot.slots[1].repair_cost, 0);
        assert!(receive_updates(&mut app, other_connection).is_empty());
    }

    fn make_appearance() -> EquipmentAppearance {
        EquipmentAppearance {
            entries: vec![
                EquippedAppearanceEntry {
                    slot: EquipmentVisualSlot::Head,
                    item_id: Some(100),
                    display_info_id: Some(200),
                    inventory_type: 1,
                    hidden: false,
                },
                EquippedAppearanceEntry {
                    slot: EquipmentVisualSlot::Chest,
                    item_id: Some(101),
                    display_info_id: Some(201),
                    inventory_type: 5,
                    hidden: false,
                },
            ],
        }
    }

    #[test]
    fn load_equipped_durability_preserves_saved_values() {
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
            equipment_appearance: make_appearance(),
            durability: vec![DurabilityEntryData {
                slot: EquipmentVisualSlot::Head,
                current: 55,
                max: 70,
            }],
        };

        let durability = load_equipped_durability(&data);

        assert_eq!(durability.slots.len(), 2);
        assert_eq!(durability.slots[0].slot, EquipmentVisualSlot::Head);
        assert_eq!(durability.slots[0].current, 55);
        assert_eq!(durability.slots[1].slot, EquipmentVisualSlot::Chest);
        assert_eq!(durability.slots[1].current, 80);
    }

    #[test]
    fn death_loss_reduces_current_durability() {
        let mut durability = EquippedDurability {
            slots: vec![DurabilitySlotState {
                slot: EquipmentVisualSlot::Chest,
                current: 80,
                max: 80,
            }],
        };

        assert!(apply_death_loss(&mut durability));
        assert_eq!(durability.slots[0].current, 72);
    }

    #[test]
    fn build_snapshot_sums_total_repair_cost() {
        let durability = EquippedDurability {
            slots: vec![
                DurabilitySlotState {
                    slot: EquipmentVisualSlot::Head,
                    current: 60,
                    max: 70,
                },
                DurabilitySlotState {
                    slot: EquipmentVisualSlot::Chest,
                    current: 70,
                    max: 80,
                },
            ],
        };

        let snapshot = build_snapshot(&durability);

        assert_eq!(snapshot.slots.len(), 2);
        assert_eq!(snapshot.slots[0].repair_cost, 150);
        assert_eq!(snapshot.slots[1].repair_cost, 180);
        assert_eq!(snapshot.total_repair_cost, 330);
    }
}
