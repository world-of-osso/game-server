//! Trade between two connected players through `TradePlugin`, with item templates
//! from data/world.db.
use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{Channel, Connected, PeerId, RemoteId};
use shared::components::{CharacterAppearance, Health, Player, Position};
use shared::protocol::*;

use super::super::*;
use crate::bank_test_harness::{COPPER_ORE, LINEN, PEACEBLOOM};
use crate::persistence::{self, ClientId, GameDatabase};

const ACCOUNT_A: u64 = 1;
const ACCOUNT_B: u64 = 2;

#[derive(Resource, Default)]
struct Updates(Vec<(Entity, TradeStateUpdate)>);

fn collect(
    mut receivers: Query<(Entity, &mut MessageReceiver<TradeStateUpdate>)>,
    mut updates: ResMut<Updates>,
) {
    for (connection, mut receiver) in &mut receivers {
        updates
            .0
            .extend(receiver.receive().map(|update| (connection, update)));
    }
}

struct Side {
    connection: Entity,
    player: Entity,
    character_id: u64,
}

struct TradeHarness {
    app: App,
    a: Side,
    b: Side,
}

fn pair<M: lightyear::prelude::Message>() -> (MessageSender<M>, MessageReceiver<M>) {
    (MessageSender::default(), MessageReceiver::default())
}

fn trade_harness() -> Option<TradeHarness> {
    let path = crate::test_data::world_db_path();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    app.insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    crate::appearance_collection::tests::install_database(&mut app);
    app.init_resource::<GoldUpdateQueue>();
    app.init_resource::<Updates>();
    app.add_systems(First, collect);
    app.add_plugins(TradePlugin);
    app.finish();
    app.cleanup();
    let a = spawn_side(&mut app, ACCOUNT_A, "Tradea", 1);
    let b = spawn_side(&mut app, ACCOUNT_B, "Tradeb", 1);
    app.update();
    Some(TradeHarness { app, a, b })
}

fn spawn_side(app: &mut App, account_id: u64, name: &str, race: u8) -> Side {
    let db = &app.world().resource::<GameDatabase>().db;
    let (character_id, _) =
        persistence::create_character_with_test_bags(db, account_id, name, race, 1).unwrap();
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: name.into(),
                race,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            ClientId(character_id),
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(account_id)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id },
            PlayerLink(player),
            (
                pair::<InitiateTrade>(),
                pair::<AcceptTrade>(),
                pair::<DeclineTrade>(),
                pair::<CancelTrade>(),
                pair::<SetTradeItem>(),
                pair::<ClearTradeItem>(),
                pair::<SetTradeMoney>(),
                pair::<ConfirmTrade>(),
                pair::<CancelTradeAccept>(),
                pair::<TradeStateUpdate>(),
            ),
        ))
        .id();
    Side {
        connection,
        player,
        character_id,
    }
}

impl TradeHarness {
    fn db(&self) -> &redb::Database {
        &self.app.world().resource::<GameDatabase>().db
    }

    fn send<M: lightyear::prelude::Message>(&mut self, from: Entity, message: M) {
        send_on::<M, TradeChannel>(&mut self.app, from, message);
        for _ in 0..3 {
            self.app.update();
        }
    }

    fn take(&mut self, connection: Entity) -> Vec<TradeStateUpdate> {
        let updates = std::mem::take(&mut self.app.world_mut().resource_mut::<Updates>().0);
        let (mine, others): (Vec<_>, Vec<_>) = updates
            .into_iter()
            .partition(|(target, _)| *target == connection);
        self.app.world_mut().resource_mut::<Updates>().0 = others;
        mine.into_iter().map(|(_, update)| update).collect()
    }

    fn errors(&mut self, connection: Entity) -> Vec<String> {
        self.take(connection)
            .into_iter()
            .filter_map(|update| update.error)
            .collect()
    }

    fn messages(&mut self, connection: Entity) -> Vec<String> {
        self.take(connection)
            .into_iter()
            .filter_map(|update| update.message)
            .collect()
    }

    fn open_trade(&mut self) {
        let (a, b) = (self.a.connection, self.b.connection);
        self.send(
            a,
            InitiateTrade {
                target_name: "tradeb".into(),
            },
        );
        let incoming = self.take(b).pop().unwrap().trade.unwrap();
        assert_eq!(incoming.phase, TradePhase::PendingIncoming);
        assert_eq!(incoming.other.name, "Tradea");
        self.send(b, AcceptTrade);
        self.take(a);
        self.take(b);
    }

    fn inventory(&self, character_id: u64) -> persistence::InventoryData {
        persistence::load_inventory(self.db(), character_id)
    }

    fn count(&self, character_id: u64, item_id: u32) -> u32 {
        self.inventory(character_id)
            .items
            .iter()
            .filter(|item| item.item_id == item_id)
            .map(|item| item.stack_count)
            .sum()
    }

    fn guid(&self, character_id: u64, item_id: u32) -> u64 {
        self.inventory(character_id)
            .items
            .iter()
            .find(|item| item.item_id == item_id)
            .unwrap()
            .item_guid
    }

    /// Run long enough for `FixedUpdate` (the range check) to step.
    fn run_fixed(&mut self) {
        self.app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_millis(50),
            ));
        for _ in 0..3 {
            self.app.update();
        }
        self.app
            .insert_resource(bevy::time::TimeUpdateStrategy::Automatic);
    }

    fn trading(&self, character_id: u64) -> bool {
        self.app
            .world()
            .resource::<ServerTradeState>()
            .manager
            .is_trading(character_id)
    }
}

#[test]
fn transmogacq_trading_an_unbound_boe_collects_for_neither_account() {
    let mut h = trade_harness().unwrap();
    let mut inventory = h.inventory(h.a.character_id);
    inventory.items.push(persistence::InventoryItemData {
        item_guid: 9_000_025,
        item_id: 25,
        name: "Worn Shortsword".into(),
        quality: 1,
        required_level: 1,
        stack_count: 1,
        vendor_sell_price: 1,
        unique_auction: false,
    });
    persistence::save_inventory(h.db(), h.a.character_id, &inventory);
    h.open_trade();
    h.send(
        h.a.connection,
        SetTradeItem {
            slot: 0,
            item_guid: 9_000_025,
            stack_count: 1,
        },
    );
    h.send(h.a.connection, ConfirmTrade);
    h.send(h.b.connection, ConfirmTrade);
    assert_eq!(h.count(h.a.character_id, 25), 0);
    assert_eq!(h.count(h.b.character_id, 25), 1);
    for account in [ACCOUNT_A, ACCOUNT_B] {
        assert!(
            !crate::persistence::appearance_store::load_account_appearances(h.db(), account)
                .has(154)
        );
    }
    crate::appearance_collection::tests::assert_restart(h.app, ACCOUNT_B, &[(154, false)]);
}

fn send_on<M: lightyear::prelude::Message, C: Channel>(app: &mut App, from: Entity, message: M) {
    app.world_mut()
        .get_mut::<MessageSender<M>>(from)
        .unwrap()
        .send::<C>(message);
}

fn offer(slot: u8, item_guid: u64, stack_count: u16) -> SetTradeItem {
    SetTradeItem {
        slot,
        item_guid,
        stack_count,
    }
}

#[test]
fn items_and_money_change_hands_in_both_directions() {
    let Some(mut h) = trade_harness() else { return };
    let (a, b) = (h.a.connection, h.b.connection);
    let (a_id, b_id) = (h.a.character_id, h.b.character_id);
    let (a_gold, b_gold) = (h.inventory(a_id).gold, h.inventory(b_id).gold);
    let (a_linen, a_peacebloom, a_ore) = (
        h.count(a_id, LINEN),
        h.count(a_id, PEACEBLOOM),
        h.count(a_id, COPPER_ORE),
    );
    let (b_linen, b_peacebloom) = (h.count(b_id, LINEN), h.count(b_id, PEACEBLOOM));
    h.open_trade();

    h.send(a, offer(0, h.guid(a_id, LINEN), 20));
    h.send(a, SetTradeMoney { copper: 1_500 });
    // The seventh slot shows the item without trading it.
    h.send(a, offer(6, h.guid(a_id, COPPER_ORE), 10));
    h.send(b, offer(0, h.guid(b_id, PEACEBLOOM), 4));
    h.send(b, SetTradeMoney { copper: 900 });
    let view = h.take(b).pop().unwrap().trade.unwrap();
    assert_eq!(view.other.gold, 1_500);
    assert_eq!(view.other.slots.len(), 7);
    assert_eq!(view.other.slots[0].as_ref().unwrap().item_id, LINEN);
    assert_eq!(view.other.slots[6].as_ref().unwrap().item_id, COPPER_ORE);
    assert_eq!(view.player.slots[0].as_ref().unwrap().stack_count, 4);
    h.send(a, ConfirmTrade);
    let accepted = h.take(b).pop().unwrap().trade.unwrap();
    assert!(accepted.other.accepted && !accepted.player.accepted);
    h.send(b, ConfirmTrade);

    assert_eq!(h.messages(a), vec!["Trade complete.".to_string()]);
    assert_eq!(h.messages(b), vec!["Trade complete.".to_string()]);
    assert!(!h.trading(a_id) && !h.trading(b_id));
    assert_eq!(h.inventory(a_id).gold, a_gold - 1_500 + 900);
    assert_eq!(h.inventory(b_id).gold, b_gold - 900 + 1_500);
    assert_eq!(h.count(a_id, LINEN), a_linen - 20);
    assert_eq!(h.count(b_id, LINEN), b_linen + 20);
    assert_eq!(h.count(a_id, PEACEBLOOM), a_peacebloom + 4);
    assert_eq!(h.count(b_id, PEACEBLOOM), b_peacebloom - 4);
    assert_eq!(h.count(a_id, COPPER_ORE), a_ore);
    let changed = h
        .app
        .world()
        .resource::<GoldUpdateQueue>()
        .changed_inventories();
    assert!(changed.contains(&a_id) && changed.contains(&b_id));
}

#[test]
fn a_full_bag_refuses_the_exchange_and_keeps_the_trade_open() {
    let Some(mut h) = trade_harness() else { return };
    let (a, b) = (h.a.connection, h.b.connection);
    let (a_id, b_id) = (h.a.character_id, h.b.character_id);
    // Retail IDs also captured in skyborne_kit_fixture.rs; none can stack with Linen.
    let filler_ids = [
        25, 35, 39, 40, 117, 159, 876, 2092, 2362, 2504, 2512, 2947, 3661, 4536, 4540, 58231,
    ];
    assert_eq!(filler_ids.len(), usize::from(BACKPACK_SLOTS));
    let catalog = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open real Retail catalog");
    let mut full = h.inventory(b_id);
    full.items = filler_ids
        .into_iter()
        .enumerate()
        .map(|(index, item_id)| {
            catalog
                .query_row(
                    "SELECT Name, Quality, RequiredLevel, SellPrice FROM content_item WHERE ID = ?1",
                    [item_id],
                    |row| {
                        Ok(persistence::InventoryItemData {
                            item_guid: 50_000 + index as u64,
                            item_id,
                            name: row.get(0)?,
                            quality: row.get(1)?,
                            required_level: row.get(2)?,
                            stack_count: 1,
                            vendor_sell_price: row.get(3)?,
                            unique_auction: false,
                        })
                    },
                )
                .expect("real Retail filler definition")
        })
        .collect();
    persistence::save_inventory(h.db(), b_id, &full);
    let model = crate::inventory::load_model(
        h.db(),
        h.app
            .world()
            .resource::<crate::item_templates::ItemTemplates>(),
        b_id,
    )
    .expect("full backpack hydrates from real Retail definitions");
    assert_eq!(model.free_cells(), 0);
    h.open_trade();
    h.send(a, offer(0, h.guid(a_id, LINEN), 20));
    h.send(a, ConfirmTrade);
    h.send(b, ConfirmTrade);

    assert_eq!(
        h.errors(b),
        vec!["Trade failed, you don't have enough space.".to_string()]
    );
    let seen = h.take(a);
    let refused = seen.last().unwrap();
    assert_eq!(
        refused.error.as_deref(),
        Some("Trade failed, target doesn't have enough space.")
    );
    let view = refused.trade.as_ref().unwrap();
    assert!(!view.player.accepted && !view.other.accepted);
    assert!(h.trading(a_id));
    assert_eq!(h.count(a_id, LINEN), 20);
    assert_eq!(h.count(b_id, LINEN), 0);
}

#[test]
fn soulbound_items_stay_out_of_the_traded_slots() {
    let Some(mut h) = trade_harness() else { return };
    let a = h.a.connection;
    let a_id = h.a.character_id;
    let linen = h.guid(a_id, LINEN);
    {
        use crate::persistence::inventory_store::{ITEM_INSTANCES, ItemInstanceData};
        let txn = h.db().begin_write().unwrap();
        txn.open_table(ITEM_INSTANCES)
            .unwrap()
            .insert(
                linen,
                bitcode::encode(&ItemInstanceData { soulbound: true }).as_slice(),
            )
            .unwrap();
        txn.commit().unwrap();
    }
    h.open_trade();

    h.send(a, offer(0, linen, 1));
    assert_eq!(
        h.errors(a),
        vec!["You can't trade a soulbound item.".to_string()]
    );
    h.send(a, offer(6, linen, 1));
    assert!(h.errors(a).is_empty());
    h.send(a, offer(1, 999_999, 1));
    assert_eq!(h.errors(a), vec!["The item was not found.".to_string()]);
}

#[test]
fn initiation_is_refused_for_dead_flying_other_faction_and_busy_targets() {
    let Some(mut h) = trade_harness() else { return };
    let (a, b) = (h.a.connection, h.b.connection);
    let c = spawn_side(&mut h.app, 3, "Grunt", 2);
    let request = |name: &str| InitiateTrade {
        target_name: name.into(),
    };
    h.send(a, request("Grunt"));
    assert_eq!(h.errors(a), vec!["Target is unfriendly.".to_string()]);
    h.send(a, request("Nobody"));
    assert_eq!(
        h.errors(a),
        vec!["No player named 'Nobody' is currently playing.".to_string()]
    );
    let b_player = h.b.player;
    h.app.world_mut().entity_mut(b_player).insert(Health {
        current: 0.0,
        max: 100.0,
    });
    h.send(a, request("Tradeb"));
    assert_eq!(
        h.errors(a),
        vec!["You can't trade with dead players.".to_string()]
    );
    h.app.world_mut().entity_mut(b_player).insert(Health {
        current: 100.0,
        max: 100.0,
    });
    h.app
        .world_mut()
        .entity_mut(b_player)
        .insert(crate::taxi::TaxiFlight {
            points: Vec::new(),
            next: 0,
        });
    h.send(a, request("Tradeb"));
    assert_eq!(
        h.errors(a),
        vec!["Trade target is too far away.".to_string()]
    );
    h.app
        .world_mut()
        .entity_mut(b_player)
        .remove::<crate::taxi::TaxiFlight>();
    h.open_trade();
    h.app.world_mut().entity_mut(c.player).insert(Player {
        name: "Tradec".into(),
        race: 1,
        class: 1,
        appearance: CharacterAppearance::default(),
    });
    h.send(c.connection, request("Tradeb"));
    assert_eq!(
        h.errors(c.connection),
        vec!["Tradeb is busy right now.".to_string()]
    );
    assert!(h.errors(b).is_empty());
}

#[test]
fn walking_away_or_leaving_cancels_the_trade() {
    let Some(mut h) = trade_harness() else { return };
    let (a, b) = (h.a.connection, h.b.connection);
    h.open_trade();
    let b_player = h.b.player;
    h.app.world_mut().entity_mut(b_player).insert(Position {
        x: 12.0,
        y: 0.0,
        z: 0.0,
    });
    h.run_fixed();
    assert_eq!(
        h.messages(a),
        vec!["Trade target is too far away.".to_string()]
    );
    assert_eq!(
        h.messages(b),
        vec!["Trade target is too far away.".to_string()]
    );
    assert!(!h.trading(h.a.character_id));

    h.app.world_mut().entity_mut(b_player).insert(Position {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    h.open_trade();
    h.app.world_mut().despawn(b_player);
    h.run_fixed();
    assert!(!h.trading(h.a.character_id));
    assert_eq!(
        h.messages(a),
        vec!["Trade target is too far away.".to_string()]
    );
}

#[test]
fn cancel_trade_accept_withdraws_only_the_players_accept() {
    let Some(mut h) = trade_harness() else { return };
    let (a, b) = (h.a.connection, h.b.connection);
    h.open_trade();
    h.send(a, ConfirmTrade);
    h.send(b, SetTradeMoney { copper: 5 });
    h.send(a, ConfirmTrade);
    h.take(b);
    h.send(a, CancelTradeAccept);

    let view = h.take(b).pop().unwrap().trade.unwrap();
    assert!(!view.other.accepted);
    assert!(h.trading(h.a.character_id));
}

#[test]
fn receiving_money_past_the_gold_cap_refuses_the_exchange() {
    let Some(mut h) = trade_harness() else { return };
    let (a, b) = (h.a.connection, h.b.connection);
    let (a_id, b_id) = (h.a.character_id, h.b.character_id);
    let mut rich = h.inventory(b_id);
    rich.gold = crate::gold::MAX_MONEY - 10;
    persistence::save_inventory(h.db(), b_id, &rich);
    h.open_trade();
    h.send(a, SetTradeMoney { copper: 11 });
    h.send(a, ConfirmTrade);
    h.send(b, ConfirmTrade);

    assert_eq!(h.errors(b), vec!["At gold limit".to_string()]);
    assert!(h.trading(a_id));
    assert_eq!(h.inventory(b_id).gold, crate::gold::MAX_MONEY - 10);
    h.take(a);
}

#[test]
fn initiation_refusals_follow_azerothcore_order() {
    let ok = Trader {
        alive: true,
        stunned: false,
        in_flight: false,
        faction: 1,
    };
    let dead = Trader { alive: false, ..ok };
    let stunned = Trader {
        stunned: true,
        ..ok
    };
    let horde = Trader { faction: 2, ..ok };
    assert_eq!(initiation_refusal(ok, ok), None);
    assert_eq!(
        initiation_refusal(dead, horde),
        Some("You can't do that when you're dead.")
    );
    assert_eq!(
        initiation_refusal(stunned, ok),
        Some("You can't do that right now.")
    );
    assert_eq!(initiation_refusal(ok, stunned), Some("Target is stunned"));
    assert_eq!(initiation_refusal(ok, horde), Some("Target is unfriendly."));
}

/// Linen and gold held by both players together.
fn totals(h: &TradeHarness) -> (u32, u64) {
    let (a_id, b_id) = (h.a.character_id, h.b.character_id);
    (
        h.count(a_id, LINEN) + h.count(b_id, LINEN),
        h.inventory(a_id).gold + h.inventory(b_id).gold,
    )
}

#[test]
fn any_offer_change_after_an_accept_resets_both_accepts() {
    let Some(mut h) = trade_harness() else { return };
    let (a, b) = (h.a.connection, h.b.connection);
    let (a_id, b_id) = (h.a.character_id, h.b.character_id);
    let (a_linen, b_linen) = (h.count(a_id, LINEN), h.count(b_id, LINEN));
    h.open_trade();
    h.send(a, offer(0, h.guid(a_id, LINEN), 5));
    h.send(a, ConfirmTrade);
    h.send(b, ConfirmTrade);
    h.take(a);
    h.take(b);
    // Completed above; reopen and accept once, then each kind of change.
    assert_eq!(h.count(b_id, LINEN), b_linen + 5);
    h.open_trade();
    let linen = h.guid(a_id, LINEN);
    for change in 0..3 {
        h.send(a, offer(0, linen, 1));
        h.send(a, ConfirmTrade);
        let seen = h.take(b).pop().unwrap().trade.unwrap();
        assert!(seen.other.accepted, "change {change}: accept shown");
        match change {
            0 => h.send(a, SetTradeMoney { copper: 3 }),
            1 => h.send(b, offer(1, h.guid(b_id, LINEN), 1)),
            _ => h.send(a, ClearTradeItem { slot: 0 }),
        }
        let view = h.take(b).pop().unwrap().trade.unwrap();
        assert!(
            !view.player.accepted && !view.other.accepted,
            "change {change}: both accepts reset"
        );
        h.take(a);
        // The other side's accept alone completes nothing.
        h.send(b, ConfirmTrade);
        assert!(h.trading(a_id));
        h.send(b, CancelTradeAccept);
        h.send(b, ClearTradeItem { slot: 1 });
        h.take(a);
        h.take(b);
    }
    assert_eq!(h.count(a_id, LINEN), a_linen - 5);
    assert_eq!(h.count(b_id, LINEN), b_linen + 5);
}

#[test]
fn an_offered_stack_that_left_the_bags_refuses_without_moving_anything() {
    let Some(mut h) = trade_harness() else { return };
    let (a, b) = (h.a.connection, h.b.connection);
    let (a_id, b_id) = (h.a.character_id, h.b.character_id);
    h.open_trade();
    h.send(a, offer(0, h.guid(a_id, LINEN), 20));
    h.send(a, SetTradeMoney { copper: 100 });
    h.send(b, offer(0, h.guid(b_id, PEACEBLOOM), 4));
    // The offered linen is destroyed while the window is open.
    let mut gone = h.inventory(a_id);
    gone.items.retain(|item| item.item_id != LINEN);
    persistence::save_inventory(h.db(), a_id, &gone);
    let before = (h.inventory(a_id), h.inventory(b_id));
    h.send(a, ConfirmTrade);
    h.send(b, ConfirmTrade);

    assert_eq!(h.errors(a), vec!["The item was not found.".to_string()]);
    assert_eq!(h.errors(b), vec!["The item was not found.".to_string()]);
    assert!(h.trading(a_id));
    assert_eq!((h.inventory(a_id), h.inventory(b_id)), before);
}

#[test]
fn ahserver_bound_item_after_offer_refuses_trade_without_moving_anything() {
    let mut h = trade_harness().expect("trade harness");
    let (a, b) = (h.a.connection, h.b.connection);
    let (a_id, b_id) = (h.a.character_id, h.b.character_id);
    let linen = h.guid(a_id, LINEN);
    h.open_trade();
    h.send(a, offer(0, linen, 20));
    h.send(a, SetTradeMoney { copper: 100 });
    h.send(b, offer(0, h.guid(b_id, PEACEBLOOM), 4));
    {
        use crate::persistence::inventory_store::{ITEM_INSTANCES, ItemInstanceData};
        let txn = h.db().begin_write().unwrap();
        txn.open_table(ITEM_INSTANCES)
            .unwrap()
            .insert(
                linen,
                bitcode::encode(&ItemInstanceData { soulbound: true }).as_slice(),
            )
            .unwrap();
        txn.commit().unwrap();
    }
    let before = (h.inventory(a_id), h.inventory(b_id));
    h.send(a, ConfirmTrade);
    h.send(b, ConfirmTrade);
    assert_eq!(
        h.errors(a),
        vec!["You can't trade a soulbound item.".to_string()]
    );
    assert_eq!(
        h.errors(b),
        vec!["You can't trade a soulbound item.".to_string()]
    );
    assert!(h.trading(a_id));
    assert_eq!((h.inventory(a_id), h.inventory(b_id)), before);
}

#[test]
fn offered_money_spent_before_the_exchange_refuses_it() {
    let Some(mut h) = trade_harness() else { return };
    let (a, b) = (h.a.connection, h.b.connection);
    let (a_id, b_id) = (h.a.character_id, h.b.character_id);
    let mut rich = h.inventory(a_id);
    rich.gold = 1_000;
    persistence::save_inventory(h.db(), a_id, &rich);
    h.open_trade();
    h.send(a, SetTradeMoney { copper: 1_000 });
    h.send(b, offer(0, h.guid(b_id, PEACEBLOOM), 4));
    let mut spent = h.inventory(a_id);
    spent.gold = 400;
    persistence::save_inventory(h.db(), a_id, &spent);
    let before = (h.inventory(a_id), h.inventory(b_id));
    h.send(a, ConfirmTrade);
    h.send(b, ConfirmTrade);

    assert_eq!(
        h.errors(a),
        vec!["You don't have enough money.".to_string()]
    );
    assert_eq!(h.errors(b), vec!["Trade failed.".to_string()]);
    assert!(h.trading(a_id));
    assert_eq!((h.inventory(a_id), h.inventory(b_id)), before);
}

#[test]
fn one_stack_in_two_slots_or_a_part_stack_never_duplicates() {
    let Some(mut h) = trade_harness() else { return };
    let (a, b) = (h.a.connection, h.b.connection);
    let (a_id, b_id) = (h.a.character_id, h.b.character_id);
    let total = totals(&h);
    let a_linen = h.count(a_id, LINEN);
    let linen = h.guid(a_id, LINEN);
    h.open_trade();
    h.send(a, offer(0, linen, 7));
    h.send(a, offer(1, linen, 7));
    assert_eq!(h.errors(a), vec!["The item was not found.".to_string()]);
    // More than the stack holds is refused too.
    h.send(a, offer(2, linen, u16::try_from(a_linen + 1).unwrap()));
    assert_eq!(h.errors(a), vec!["The item was not found.".to_string()]);
    h.send(a, ConfirmTrade);
    h.send(b, ConfirmTrade);

    assert_eq!(h.messages(b), vec!["Trade complete.".to_string()]);
    assert_eq!(h.count(a_id, LINEN), a_linen - 7);
    assert_eq!(h.count(b_id, LINEN), total.0 - a_linen + 7);
    assert_eq!(totals(&h), total);
    // The part left behind keeps its instance; the 7 moved as a new one.
    assert_eq!(h.guid(a_id, LINEN), linen);
    assert!(
        h.inventory(b_id)
            .items
            .iter()
            .all(|item| item.item_guid != linen)
    );
}
