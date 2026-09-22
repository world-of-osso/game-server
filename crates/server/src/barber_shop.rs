use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CharacterAppearance, Player};
use shared::protocol::{
    ApplyBarberShopChanges, BarberShopChannel, BarberShopSnapshot, BarberShopStateUpdate,
    QueryBarberShopStatus,
};

use crate::auth::{Authenticated, InWorld};
use crate::gold::GoldUpdateQueue;
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase};
use crate::persistence_types::InventoryData;

const BARBER_COST_PER_CHANGE: u32 = 10_000;

pub struct BarberShopPlugin;

impl Plugin for BarberShopPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, send_initial_barber_shop_snapshot);
        app.add_systems(Update, process_barber_shop_queries);
        app.add_systems(Update, process_barber_shop_apply_requests);
    }
}

type NewInWorldConnection<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static PlayerLink, &'static InWorld),
    (With<Authenticated>, Added<InWorld>),
>;

fn send_initial_barber_shop_snapshot(
    connections: NewInWorldConnection,
    players: Query<&Player>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<BarberShopStateUpdate>>,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, in_world) in &connections {
        send_barber_shop_snapshot(
            conn_entity,
            link.0,
            in_world.character_id,
            &players,
            &db,
            &mut senders,
        );
    }
}

fn process_barber_shop_queries(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<QueryBarberShopStatus>,
        ),
        With<Authenticated>,
    >,
    players: Query<&Player>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<BarberShopStateUpdate>>,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, in_world, mut receiver) in &mut receivers {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        send_barber_shop_snapshot(
            conn_entity,
            link.0,
            in_world.character_id,
            &players,
            &db,
            &mut senders,
        );
    }
}

fn send_barber_shop_snapshot(
    conn_entity: Entity,
    player_entity: Entity,
    character_id: u64,
    players: &Query<&Player>,
    db: &GameDatabase,
    senders: &mut Query<&mut MessageSender<BarberShopStateUpdate>>,
) {
    let Ok(player) = players.get(player_entity) else {
        send_barber_shop_error(conn_entity, senders, "barber shop unavailable for player");
        return;
    };

    let gold = persistence::load_inventory(&db.db, character_id).gold;
    send_barber_shop_update(
        conn_entity,
        &player.appearance,
        gold,
        Some("barber shop ready".into()),
        None,
        senders,
    );
}

fn process_barber_shop_apply_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<ApplyBarberShopChanges>,
        ),
        With<Authenticated>,
    >,
    mut players: Query<&mut Player>,
    db: Option<Res<GameDatabase>>,
    dirty_queue: Option<Res<ReplicationDirtyQueue>>,
    mut gold_updates: ResMut<GoldUpdateQueue>,
    mut senders: Query<&mut MessageSender<BarberShopStateUpdate>>,
) {
    let Some(db) = db else { return };
    for (conn_entity, link, in_world, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            handle_apply_request(
                BarberShopApplication {
                    connection: conn_entity,
                    player: link.0,
                    character_id: in_world.character_id,
                    changes: request,
                },
                &mut players,
                &db,
                dirty_queue.as_deref(),
                &mut gold_updates,
                &mut senders,
            );
        }
    }
}

struct BarberShopApplication {
    connection: Entity,
    player: Entity,
    character_id: u64,
    changes: ApplyBarberShopChanges,
}

fn handle_apply_request(
    application: BarberShopApplication,
    players: &mut Query<&mut Player>,
    db: &GameDatabase,
    dirty_queue: Option<&ReplicationDirtyQueue>,
    gold_updates: &mut GoldUpdateQueue,
    senders: &mut Query<&mut MessageSender<BarberShopStateUpdate>>,
) {
    let Ok(mut player) = players.get_mut(application.player) else {
        send_barber_shop_error(
            application.connection,
            senders,
            "barber shop unavailable for player",
        );
        return;
    };

    let applied = barber_shop_appearance(&player.appearance, application.changes.appearance);
    let cost = barber_shop_cost(&player.appearance, &applied);
    let mut inventory = persistence::load_inventory(&db.db, application.character_id);
    if inventory.gold < cost {
        send_barber_shop_update(
            application.connection,
            &player.appearance,
            inventory.gold,
            None,
            Some("not enough gold for barber shop".into()),
            senders,
        );
        return;
    }

    inventory.gold -= cost;
    let appearance_changed = player.appearance != applied;
    player.appearance = applied.clone();
    if appearance_changed && let Some(dirty_queue) = dirty_queue {
        dirty_queue.mark(application.player);
    }
    save_barber_shop_changes(
        &db.db,
        application.character_id,
        applied,
        &inventory,
        gold_updates,
    );
    send_barber_shop_update(
        application.connection,
        &player.appearance,
        inventory.gold,
        Some(barber_shop_success_message(cost)),
        None,
        senders,
    );
}

fn save_barber_shop_changes(
    db: &redb::Database,
    character_id: u64,
    appearance: CharacterAppearance,
    inventory: &InventoryData,
    gold_updates: &mut GoldUpdateQueue,
) {
    let Some(mut data) = persistence::load_character(db, character_id) else {
        persistence::save_inventory(db, character_id, inventory);
        gold_updates.record(character_id, inventory.gold);
        return;
    };
    data.appearance = appearance;
    persistence::save_character(db, character_id, &data);
    persistence::save_inventory(db, character_id, inventory);
    gold_updates.record(character_id, inventory.gold);
}

fn barber_shop_success_message(cost: u32) -> String {
    if cost == 0 {
        "barber shop changes applied".into()
    } else {
        format!("barber shop changes applied for {}", format_money(cost))
    }
}

fn format_money(copper: u32) -> String {
    let gold = copper / 10_000;
    let silver = (copper % 10_000) / 100;
    let copper = copper % 100;
    format!("{gold}g {silver}s {copper}c")
}

fn barber_shop_appearance(
    current: &CharacterAppearance,
    requested: CharacterAppearance,
) -> CharacterAppearance {
    CharacterAppearance {
        sex: current.sex,
        skin_color: requested.skin_color,
        face: requested.face,
        eye_color: current.eye_color,
        hair_style: requested.hair_style,
        hair_color: requested.hair_color,
        facial_style: requested.facial_style,
        customization_choices: current.customization_choices.clone(),
    }
}

fn barber_shop_cost(current: &CharacterAppearance, requested: &CharacterAppearance) -> u32 {
    let changed_fields = [
        current.skin_color != requested.skin_color,
        current.face != requested.face,
        current.hair_style != requested.hair_style,
        current.hair_color != requested.hair_color,
        current.facial_style != requested.facial_style,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count() as u32;
    changed_fields * BARBER_COST_PER_CHANGE
}

fn send_barber_shop_update(
    conn_entity: Entity,
    appearance: &CharacterAppearance,
    gold: u32,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<BarberShopStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<BarberShopChannel>(BarberShopStateUpdate {
        snapshot: Some(BarberShopSnapshot {
            appearance: appearance.clone(),
            gold,
        }),
        message,
        error,
    });
}

fn send_barber_shop_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<BarberShopStateUpdate>>,
    error: impl Into<String>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<BarberShopChannel>(BarberShopStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn appearance() -> CharacterAppearance {
        CharacterAppearance {
            sex: 1,
            skin_color: 1,
            face: 2,
            eye_color: 3,
            hair_style: 4,
            hair_color: 5,
            facial_style: 6,
            customization_choices: Vec::new(),
        }
    }

    #[test]
    fn barber_shop_persistence_updates_replicated_gold_after_save() {
        let db = crate::persistence::temp_database();
        let (character_id, _) = crate::persistence::create_character(&db, 1, "Alice", 1, 1)
            .expect("character should be created");
        let inventory = InventoryData {
            gold: 90_000,
            items: Vec::new(),
        };

        let mut app = App::new();
        app.add_plugins(crate::gold::GoldPlugin);
        let player = app
            .world_mut()
            .spawn((
                Player {
                    name: "Alice".into(),
                    race: 1,
                    class: 1,
                    appearance: appearance(),
                },
                crate::persistence::ClientId(character_id),
                shared::components::Gold(100_000),
            ))
            .id();

        {
            let mut queue = app
                .world_mut()
                .resource_mut::<crate::gold::GoldUpdateQueue>();
            save_barber_shop_changes(&db, character_id, appearance(), &inventory, &mut queue);
        }

        app.update();

        assert_eq!(
            crate::persistence::load_inventory(&db, character_id).gold,
            90_000
        );
        assert_eq!(
            app.world()
                .entity(player)
                .get::<shared::components::Gold>()
                .unwrap()
                .0,
            90_000
        );
    }

    #[test]
    fn barber_shop_cost_counts_mutable_field_changes() {
        let current = appearance();
        let requested = CharacterAppearance {
            skin_color: 7,
            hair_color: 2,
            ..current.clone()
        };

        assert_eq!(barber_shop_cost(&current, &requested), 20_000);
    }

    #[test]
    fn barber_shop_appearance_preserves_sex_and_eye_color() {
        let current = appearance();
        let requested = CharacterAppearance {
            sex: 0,
            skin_color: 4,
            face: 1,
            eye_color: 0,
            hair_style: 2,
            hair_color: 3,
            facial_style: 4,
            customization_choices: Vec::new(),
        };

        let applied = barber_shop_appearance(&current, requested);

        assert_eq!(applied.sex, current.sex);
        assert_eq!(applied.eye_color, current.eye_color);
        assert_eq!(applied.hair_style, 2);
        assert_eq!(applied.facial_style, 4);
    }

    #[test]
    fn barber_shop_success_message_formats_money_cost() {
        assert_eq!(
            barber_shop_success_message(12_345),
            "barber shop changes applied for 1g 23s 45c"
        );
    }
}
