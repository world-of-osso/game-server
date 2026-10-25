use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CharacterAppearance, Player};
use shared::protocol::{
    ApplyBarberShopChanges, BarberShopChannel, BarberShopSnapshot, BarberShopStateUpdate,
    QueryBarberShopStatus,
};

use crate::auth::{Authenticated, InWorld};
use crate::gold::{GoldUpdateQueue, format_money};
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
    mut commands: Commands,
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
                &mut commands,
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
    commands: &mut Commands,
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
    if inventory.gold < u64::from(cost) {
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

    inventory.gold -= u64::from(cost);
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
    // TC CharacterHandler.cpp:1867-1879: paid copper, then the haircut itself.
    let player = application.player;
    commands.queue(move |world: &mut World| {
        use crate::achievement::{CriteriaEvent, record_player_event};
        if cost > 0 {
            record_player_event(
                world,
                player,
                CriteriaEvent::MoneySpentAtBarberShop(u64::from(cost)),
            );
        }
        record_player_event(world, player, CriteriaEvent::GotHaircut);
    });
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
        format!(
            "barber shop changes applied for {}",
            format_money(u64::from(cost))
        )
    }
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
        visage: current.visage.clone(),
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
    gold: u64,
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
            visage: None,
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

    /// TC HandleAlterAppearance: a paid haircut credits GotHaircut48 and its copper65;
    /// an unaffordable request credits nothing.
    #[test]
    fn achievements_criteria_paid_haircut_credits_haircut_and_copper_not_refusals() {
        use crate::achievement::PlayerAchievementState;
        use lightyear::connection::host::HostClient;
        use lightyear::prelude::{Connected, PeerId, RemoteId};
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.add_plugins(lightyear::prelude::server::ServerPlugins {
            tick_duration: std::time::Duration::from_millis(50),
        });
        app.add_plugins(shared::protocol::ProtocolPlugin);
        app.add_plugins(BarberShopPlugin);
        app.init_resource::<GoldUpdateQueue>();
        app.init_resource::<crate::maps::ServerClock>();
        let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
        app.insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
        let db = crate::persistence::temp_database();
        let (character_id, _) = crate::persistence::create_character(&db, 1, "Alice", 1, 1)
            .expect("character should be created");
        let poor = InventoryData {
            gold: 5_000,
            items: Vec::new(),
        };
        crate::persistence::save_inventory(&db, character_id, &poor);
        app.insert_resource(GameDatabase::new(db));
        app.finish();
        app.cleanup();
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
                PlayerAchievementState::default(),
            ))
            .id();
        let connection = app
            .world_mut()
            .spawn((
                RemoteId(PeerId::Local(1)),
                Connected,
                HostClient { buffer: Vec::new() },
                Authenticated { account_id: 1 },
                InWorld { character_id },
                PlayerLink(player),
                MessageSender::<ApplyBarberShopChanges>::default(),
                MessageReceiver::<ApplyBarberShopChanges>::default(),
                MessageSender::<BarberShopStateUpdate>::default(),
                MessageReceiver::<BarberShopStateUpdate>::default(),
            ))
            .id();
        app.update();
        let request = |app: &mut App| {
            app.world_mut()
                .get_mut::<MessageSender<ApplyBarberShopChanges>>(connection)
                .unwrap()
                .send::<BarberShopChannel>(ApplyBarberShopChanges {
                    appearance: CharacterAppearance {
                        hair_color: 9,
                        ..appearance()
                    },
                });
            for _ in 0..3 {
                app.update();
            }
        };
        // Statistic1147 criterion3357 (copper) and Shave and a Haircut545 criterion2341.
        let counters = |app: &App| {
            let state = app.world().get::<PlayerAchievementState>(player).unwrap();
            (
                state.criteria_progress.get(&3357).copied(),
                state.criteria_progress.get(&2341).copied(),
                state.earned_ids.contains(&545),
            )
        };
        request(&mut app);
        assert_eq!(counters(&app), (None, None, false));
        let rich = InventoryData {
            gold: 100_000,
            items: Vec::new(),
        };
        crate::persistence::save_inventory(
            &app.world().resource::<GameDatabase>().db,
            character_id,
            &rich,
        );
        request(&mut app);
        assert_eq!(counters(&app), (Some(10_000), Some(1), true));
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
            visage: None,
        };

        let applied = barber_shop_appearance(&current, requested);

        assert_eq!(applied.sex, current.sex);
        assert_eq!(applied.eye_color, current.eye_color);
        assert_eq!(applied.hair_style, 2);
        assert_eq!(applied.facial_style, 4);
    }

    #[test]
    fn barber_shop_preserves_additional_customization_choices() {
        let current = CharacterAppearance {
            customization_choices: vec![shared::components::CustomizationChoiceSelection {
                option_id: 87,
                choice_id: 1734,
            }],
            ..appearance()
        };
        let requested = CharacterAppearance {
            hair_style: 9,
            ..appearance()
        };
        let applied = barber_shop_appearance(&current, requested);
        assert_eq!(applied.customization_choices, current.customization_choices);
        assert_eq!(applied.hair_style, 9);
    }

    #[test]
    fn barber_shop_success_message_formats_money_cost() {
        assert_eq!(
            barber_shop_success_message(12_345),
            "barber shop changes applied for 1g 23s 45c"
        );
    }
}
