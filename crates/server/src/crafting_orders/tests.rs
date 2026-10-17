use super::*;
use crate::crafting_orders::{self as orders, PlaceOrder};

const RECIPE: u32 = 28208;
const OUTPUT: u32 = 22658;

fn order_harness() -> Harness {
    order_harness_in(None)
}

fn order_harness_in(path: Option<&std::path::Path>) -> Harness {
    order_harness_with(path, |_| {})
}

fn order_harness_with(path: Option<&std::path::Path>, extra: impl FnOnce(&mut App)) -> Harness {
    let mut h = harness_with(80, extra).unwrap();
    if let Some(path) = path {
        h.app.world_mut().remove_resource::<GameDatabase>();
        let db = redb::Database::create(path).unwrap();
        persistence::create_tables(&db);
        let (id, _) = persistence::create_character(&db, 1, "Crafter", 1, 1).unwrap();
        h.character_id = id;
        h.app.world_mut().get_mut::<ClientId>(h.player).unwrap().0 = id;
        h.app.insert_resource(GameDatabase::new(db));
    }
    h.conn.execute_batch("CREATE TEMP TABLE crafting_order (ID INTEGER PRIMARY KEY, SkillLineAbilityID INTEGER, CraftingOrderHouseID INTEGER, Field_10_0_2_46479_002 INTEGER)").unwrap();
    let csv = include_str!("CraftingOrder.csv");
    for line in csv.lines().skip(1) {
        let row: Vec<u32> = line.split(',').map(|v| v.parse().unwrap()).collect();
        h.conn
            .execute(
                "INSERT INTO crafting_order VALUES (?1,?2,?3,?4)",
                rusqlite::params![row[0], row[1], row[2], row[3]],
            )
            .unwrap();
    }
    let catalog = orders::load_catalog(&h.conn).unwrap();
    assert_eq!(catalog.0.len(), 89);
    h.app.insert_resource(catalog);
    let spells = world_db().unwrap();
    h.app.insert_resource(SpellStore::fixture(vec![
        spells.store.get(RECIPE).unwrap().clone(),
    ]));
    h.app
        .world_mut()
        .get_mut::<PlayerProfessions>(h.player)
        .unwrap()
        .spells
        .push(RECIPE);
    let db = &h.app.world().resource::<GameDatabase>().db;
    for (id, count) in [(22682, 5), (14048, 4), (7080, 2), (14227, 4)] {
        crate::inventory::grant_item(
            db,
            h.app.world().resource::<ItemTemplates>(),
            h.app.world().resource::<crate::merchant::VendorCatalog>(),
            h.character_id,
            (id, count),
        )
        .unwrap();
    }
    let mut inventory = persistence::load_inventory(db, h.character_id);
    inventory.gold = 10_000;
    persistence::save_inventory(db, h.character_id, &inventory);
    h
}

fn request(kind: u8) -> PlaceOrder {
    PlaceOrder {
        spell_id: RECIPE,
        kind,
        recipient: None,
        commission: 1000,
        supplied: vec![(22682, 5, 5), (14048, 4, 4), (7080, 2, 2), (14227, 4, 4)],
        duration_hours: 12,
        note: "Please craft my cloak".into(),
    }
}

#[test]
fn crafting_orders_place_holds_reagents_and_commission() {
    let mut h = order_harness();
    let now = crate::mail::unix_now();
    let id = orders::place(h.app.world_mut(), h.player, request(0), now).unwrap();
    assert!(id > 0);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let inventory = persistence::load_inventory(db, h.character_id);
    assert_eq!(inventory.gold, 9000);
    assert!(inventory.items.is_empty());
}

fn crafter(h: &mut Harness, name: &str) -> (Entity, u64) {
    let db = &h.app.world().resource::<GameDatabase>().db;
    let (id, _) = persistence::create_character(db, 2, name, 1, 1).unwrap();
    let profs = h
        .app
        .world()
        .get::<PlayerProfessions>(h.player)
        .unwrap()
        .clone();
    let player = h
        .app
        .world_mut()
        .spawn((
            Player {
                name: name.into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            ClientId(id),
            Level(80),
            Health {
                current: 100.0,
                max: 100.0,
            },
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            UnitFactionTemplate(1),
            crate::maps::InMap::continent(0),
            Moving(false),
            profs,
        ))
        .id();
    (player, id)
}

#[test]
fn achievements_criteria_crafting_orders_all_types_credit_only_committed_fulfillment() {
    for kind in [0, 1, 2] {
        complete_order(kind);
    }
}

fn complete_order(kind: u8) {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    let mut h = order_harness_with(None, |app| {
        app.add_plugins(AchievementPlugin);
    });
    h.app
        .insert_resource(load_achievement_catalog(&h.conn).unwrap());
    let now = crate::mail::unix_now();
    let (worker, worker_id) = crafter(&mut h, "Worker");
    h.app
        .world_mut()
        .entity_mut(worker)
        .insert(PlayerAchievementState::default());
    h.app.world_mut().spawn((
        Authenticated { account_id: 2 },
        PlayerLink(worker),
        InWorld {
            character_id: worker_id,
        },
    ));
    h.app.update();
    let mut req = request(kind);
    if kind == 1 {
        let db = &h.app.world().resource::<GameDatabase>().db;
        persistence::set_character_guild_by_id(db, h.character_id, 7, "Guild").unwrap();
        persistence::set_character_guild_by_id(db, worker_id, 7, "Guild").unwrap();
    }
    if kind == 2 {
        req.recipient = Some("Worker".into());
    }
    let id = orders::place(h.app.world_mut(), h.player, req, now).unwrap();
    assert!(
        orders::browse(h.app.world(), worker, 197, kind, false, now)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        orders::browse(h.app.world(), worker, 2540, kind, false, now)
            .unwrap()
            .len(),
        1
    );
    orders::claim(h.app.world_mut(), worker, id, now).unwrap();
    assert!(orders::cancel(h.app.world_mut(), h.player, id, now).is_err());
    orders::begin_fulfill(h.app.world_mut(), worker, id, now).unwrap();
    assert!(h.app.world().get::<CastState>(worker).is_some());
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert!(persistence::mail_store::load_mailbox(db, h.character_id).is_empty());
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(worker)
            .unwrap()
            .criteria_progress
            .contains_key(&56767),
        "cast admission is not fulfillment"
    );
    h.advance(400);
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        orders::all(db).unwrap()[0].status,
        2,
        "native settlement committed"
    );
    let delivery = persistence::mail_store::load_mailbox(db, h.character_id);
    assert_eq!(delivery.len(), 1);
    assert_eq!(delivery[0].items[0].item.item_id, OUTPUT);
    assert_eq!(delivery[0].items[0].item.stack_count, 1);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(worker)
            .unwrap()
            .criteria_progress
            .get(&56767),
        Some(&1)
    );
    let typed = [56768, 56769, 56770];
    for (index, criterion) in typed.into_iter().enumerate() {
        assert_eq!(
            h.app
                .world()
                .get::<PlayerAchievementState>(worker)
                .unwrap()
                .criteria_progress
                .get(&criterion)
                .copied(),
            (index == usize::from(kind)).then_some(1)
        );
    }
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        persistence::load_achievements(db, worker_id)
            .criteria_progress
            .get(&56767),
        Some(&1)
    );
    assert!(
        !persistence::load_achievements(db, h.character_id)
            .criteria_progress
            .contains_key(&56767),
        "customer did not fulfill the order"
    );
    let payment = persistence::mail_store::load_mailbox(db, worker_id);
    assert_eq!(payment.len(), 1);
    assert_eq!(payment[0].money, 1000);
    assert!(persistence::load_inventory(db, worker_id).items.is_empty());
    assert!(orders::begin_fulfill(h.app.world_mut(), worker, id, now).is_err());
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(worker)
            .unwrap()
            .criteria_progress
            .get(&56767),
        Some(&1)
    );
}

#[test]
fn crafting_orders_guild_and_personal_browse_and_claim_restrict_identity() {
    let mut h = order_harness();
    let now = crate::mail::unix_now();
    let (worker, worker_id) = crafter(&mut h, "Worker");
    let (outsider, _) = crafter(&mut h, "Outsider");
    let db = &h.app.world().resource::<GameDatabase>().db;
    persistence::set_character_guild_by_id(db, h.character_id, 7, "Guild").unwrap();
    persistence::set_character_guild_by_id(db, worker_id, 7, "Guild").unwrap();
    let id = orders::place(h.app.world_mut(), h.player, request(1), now).unwrap();
    assert!(
        orders::browse(h.app.world(), outsider, 2540, 1, false, now)
            .unwrap()
            .is_empty()
    );
    assert!(orders::claim(h.app.world_mut(), outsider, id, now).is_err());
    orders::claim(h.app.world_mut(), worker, id, now).unwrap();
    orders::release(h.app.world_mut(), worker, id).unwrap();
    orders::cancel(h.app.world_mut(), h.player, id, now).unwrap();
    // New materials after the previous order's refund is held in mail.
    let db = &h.app.world().resource::<GameDatabase>().db;
    for (item, count) in [(22682, 5), (14048, 4), (7080, 2), (14227, 4)] {
        crate::inventory::grant_item(
            db,
            h.app.world().resource::<ItemTemplates>(),
            h.app.world().resource::<crate::merchant::VendorCatalog>(),
            h.character_id,
            (item, count),
        )
        .unwrap();
    }
    let mut req = request(2);
    req.recipient = Some("wOrKeR".into());
    let personal = orders::place(h.app.world_mut(), h.player, req, now).unwrap();
    assert!(
        orders::browse(h.app.world(), outsider, 2540, 2, false, now)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        orders::browse(h.app.world(), worker, 2540, 2, false, now)
            .unwrap()
            .len(),
        1
    );
    assert!(orders::claim(h.app.world_mut(), outsider, personal, now).is_err());
    orders::claim(h.app.world_mut(), worker, personal, now).unwrap();
}

#[test]
fn crafting_orders_cancel_and_expiry_refund_exact_escrow_once() {
    for expired in [false, true] {
        let mut h = order_harness();
        let now = crate::mail::unix_now();
        let before = persistence::load_inventory(
            &h.app.world().resource::<GameDatabase>().db,
            h.character_id,
        );
        let id = orders::place(h.app.world_mut(), h.player, request(0), now).unwrap();
        if expired {
            orders::expire(
                &h.app.world().resource::<GameDatabase>().db,
                now + 12 * 3600,
            )
            .unwrap();
            orders::expire(
                &h.app.world().resource::<GameDatabase>().db,
                now + 12 * 3600 + 1,
            )
            .unwrap();
        } else {
            orders::cancel(h.app.world_mut(), h.player, id, now).unwrap();
        }
        assert!(orders::cancel(h.app.world_mut(), h.player, id, now).is_err());
        let db = &h.app.world().resource::<GameDatabase>().db;
        let mails = persistence::mail_store::load_mailbox(db, h.character_id);
        assert_eq!(mails.len(), 1);
        assert_eq!(mails[0].money, 1000);
        let mut refunded: Vec<_> = mails[0].items.iter().map(|v| v.item.clone()).collect();
        let mut supplied = before.items;
        refunded.sort_by_key(|item| item.item_guid);
        supplied.sort_by_key(|item| item.item_guid);
        assert_eq!(refunded, supplied);
        assert_eq!(orders::all(db).unwrap()[0].status, 3);
    }
}

#[test]
fn crafting_orders_reject_invalid_recipe_reagents_and_escrow_without_mutation() {
    let mut h = order_harness();
    let now = crate::mail::unix_now();
    let before =
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, h.character_id);
    for invalid in 0..5 {
        let mut req = request(0);
        match invalid {
            0 => req.spell_id = 2963,
            1 => req.supplied[0].1 = 6,
            2 => req.supplied[0].2 = 6,
            3 => req.commission = 10001,
            _ => req.duration_hours = 0,
        }
        assert!(orders::place(h.app.world_mut(), h.player, req, now).is_err());
        assert_eq!(
            persistence::load_inventory(
                &h.app.world().resource::<GameDatabase>().db,
                h.character_id
            ),
            before
        );
    }
}

#[test]
fn crafting_orders_interrupted_cast_preserves_order_escrow() {
    let mut h = order_harness();
    let now = crate::mail::unix_now();
    let (worker, worker_id) = crafter(&mut h, "Worker");
    let id = orders::place(h.app.world_mut(), h.player, request(0), now).unwrap();
    orders::claim(h.app.world_mut(), worker, id, now).unwrap();
    orders::begin_fulfill(h.app.world_mut(), worker, id, now).unwrap();
    h.app.world_mut().get_mut::<Moving>(worker).unwrap().0 = true;
    h.advance(5);
    h.app.world_mut().get_mut::<Moving>(worker).unwrap().0 = false;
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(orders::all(db).unwrap()[0].status, 1);
    assert!(persistence::mail_store::load_mailbox(db, worker_id).is_empty());
    orders::release(h.app.world_mut(), worker, id).unwrap();
    orders::cancel(h.app.world_mut(), h.player, id, now).unwrap();
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        persistence::mail_store::load_mailbox(db, h.character_id)[0].money,
        1000
    );
}

#[test]
fn crafting_orders_partial_customer_reagents_charge_crafter_only_on_completion() {
    let mut h = order_harness();
    let now = crate::mail::unix_now();
    let (worker, worker_id) = crafter(&mut h, "Worker");
    let mut req = request(0);
    req.supplied[0].2 = 2;
    let id = orders::place(h.app.world_mut(), h.player, req, now).unwrap();
    orders::claim(h.app.world_mut(), worker, id, now).unwrap();
    assert!(orders::begin_fulfill(h.app.world_mut(), worker, id, now).is_err());
    let db = &h.app.world().resource::<GameDatabase>().db;
    crate::inventory::grant_item(
        db,
        h.app.world().resource::<ItemTemplates>(),
        h.app.world().resource::<crate::merchant::VendorCatalog>(),
        worker_id,
        (22682, 3),
    )
    .unwrap();
    orders::begin_fulfill(h.app.world_mut(), worker, id, now).unwrap();
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        persistence::load_inventory(db, worker_id).items[0].stack_count,
        3
    );
    h.advance(400);
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert!(persistence::load_inventory(db, worker_id).items.is_empty());
    let customer = persistence::load_inventory(db, h.character_id);
    assert_eq!(customer.items.len(), 1);
    assert_eq!(customer.items[0].stack_count, 3);
    assert_eq!(
        persistence::mail_store::load_mailbox(db, worker_id)[0].money,
        1000
    );
}

#[test]
fn crafting_orders_claim_and_escrow_survive_database_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orders.redb");
    let mut h = order_harness_in(Some(&path));
    let now = crate::mail::unix_now();
    let (worker, worker_id) = crafter(&mut h, "Worker");
    let id = orders::place(h.app.world_mut(), h.player, request(0), now).unwrap();
    orders::claim(h.app.world_mut(), worker, id, now).unwrap();
    h.app.world_mut().remove_resource::<GameDatabase>();
    let db = redb::Database::open(&path).unwrap();
    persistence::create_tables(&db);
    assert_eq!(orders::all(&db).unwrap()[0].status, 1);
    h.app.insert_resource(GameDatabase::new(db));
    orders::begin_fulfill(h.app.world_mut(), worker, id, now).unwrap();
    h.advance(400);
    h.app.world_mut().remove_resource::<GameDatabase>();
    let db = redb::Database::open(&path).unwrap();
    persistence::create_tables(&db);
    assert_eq!(orders::all(&db).unwrap()[0].status, 2);
    assert_eq!(
        persistence::mail_store::load_mailbox(&db, worker_id)[0].money,
        1000
    );
    assert_eq!(
        persistence::mail_store::load_mailbox(&db, h.character_id)[0].items[0]
            .item
            .item_id,
        OUTPUT
    );
}

#[test]
fn crafting_orders_wire_customer_query_place_and_cancel() {
    use shared::protocol::{
        CraftingOrderKind as Kind, CraftingOrderReagent, CraftingOrderRequest as Request,
        CraftingOrderStateUpdate as Reply,
    };
    let mut h = order_harness();
    orders::CraftingOrdersPlugin.build(&mut h.app);
    add_inbox::<Reply>(&mut h.app);
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert((pair::<Request>(), pair::<Reply>()));
    h.app
        .world_mut()
        .get_mut::<PlayerProfessions>(h.player)
        .unwrap()
        .spells
        .clear();
    h.app
        .world_mut()
        .get_mut::<PlayerProfessions>(h.player)
        .unwrap()
        .lines
        .clear();
    // Customer broker asks for all professions without requiring a learned skill.
    h.send::<Request, ProfessionChannel>(Request::Query {
        skill_line: 0,
        kind: Kind::Public,
        mine: true,
    });
    let reply = h.take::<Reply>().pop().unwrap();
    assert!(reply.error.is_none());
    let snapshot = reply.snapshot.unwrap();
    let recipe = snapshot
        .recipes
        .iter()
        .find(|recipe| recipe.spell_id == RECIPE)
        .unwrap();
    assert_eq!(recipe.item_id, OUTPUT);
    assert_eq!(recipe.skill_line, 2540);
    let reagents = recipe
        .reagents
        .iter()
        .map(|reagent| CraftingOrderReagent {
            item_id: reagent.item_id,
            quantity: reagent.quantity,
            supplied: reagent.quantity,
        })
        .collect();
    let place = Request::Place {
        spell_id: RECIPE,
        kind: Kind::Public,
        recipient: None,
        commission: 1000,
        reagents,
        duration_hours: 12,
        note: "Wire order".into(),
    };
    let before =
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, h.character_id);
    h.send::<Request, ProfessionChannel>(place.clone());
    assert!(h.take::<Reply>().pop().unwrap().error.is_some());
    assert_eq!(
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, h.character_id),
        before
    );
    let (name, flags, faction): (String, u64, u32) = h
        .conn
        .query_row(
            "SELECT name,npcflag,faction FROM tdb_creature_template WHERE entry=215258",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let position = *h
        .app
        .world()
        .get::<shared::components::Position>(h.player)
        .unwrap();
    let broker = h
        .app
        .world_mut()
        .spawn((
            shared::components::Npc {
                template_id: 215258,
                name,
            },
            shared::protocol::NpcFlags(flags),
            position,
            shared::components::Health {
                current: 100.0,
                max: 100.0,
            },
            shared::components::UnitFactionTemplate(faction),
            crate::maps::InMap::continent(0),
        ))
        .id();
    h.send::<shared::protocol::InteractNpc, shared::protocol::InteractionChannel>(
        shared::protocol::InteractNpc {
            npc: broker.to_bits(),
        },
    );
    h.send::<shared::protocol::SelectGossipOption, shared::protocol::InteractionChannel>(
        shared::protocol::SelectGossipOption {
            npc: broker.to_bits(),
            option_id: 0,
        },
    );
    h.send::<Request, ProfessionChannel>(place);
    let reply = h.take::<Reply>().pop().unwrap();
    assert!(reply.error.is_none(), "{:?}", reply.error);
    let order = reply.snapshot.unwrap().orders.pop().unwrap();
    assert!(order.can_cancel);
    assert!(!order.can_claim);
    h.send::<Request, ProfessionChannel>(Request::Query {
        skill_line: 0,
        kind: Kind::Public,
        mine: true,
    });
    let snapshot = h.take::<Reply>().pop().unwrap().snapshot.unwrap();
    assert_eq!(snapshot.skill_line, 0);
    assert!(snapshot.orders.iter().any(|row| row.id == order.id));
    assert!(snapshot.recipes.iter().any(|row| row.spell_id == RECIPE));
    h.send::<Request, ProfessionChannel>(Request::Cancel { order_id: order.id });
    let reply = h.take::<Reply>().pop().unwrap();
    assert!(reply.error.is_none());
    assert_eq!(
        reply.snapshot.unwrap().orders[0].status,
        shared::protocol::CraftingOrderStatus::Cancelled
    );
}
