use super::*;
use crate::crafting_orders::{self as orders, PlaceOrder};

const RECIPE: u32 = 28208;
const OUTPUT: u32 = 22658;

fn order_harness() -> Harness {
    let mut h = harness(80).unwrap();
    h.conn.execute_batch("CREATE TEMP TABLE crafting_order (ID INTEGER PRIMARY KEY, SkillLineAbilityID INTEGER, CraftingOrderHouseID INTEGER, Field_10_0_2_46479_002 INTEGER)").unwrap();
    let csv = std::fs::read_to_string("/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/crafting-orders-srv-2026-10-11/CraftingOrder.csv").unwrap();
    for line in csv.lines().skip(1) {
        let row: Vec<u32> = line.split(',').map(|v| v.parse().unwrap()).collect();
        h.conn
            .execute(
                "INSERT INTO crafting_order VALUES (?1,?2,?3,?4)",
                rusqlite::params![row[0], row[1], row[2], row[3]],
            )
            .unwrap();
    }
    h.app
        .insert_resource(orders::load_catalog(&h.conn).unwrap());
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
            Moving(false),
            profs,
        ))
        .id();
    (player, id)
}

#[test]
fn crafting_orders_public_claim_fulfil_uses_cast_and_delivers_mail() {
    let mut h = order_harness();
    let now = crate::mail::unix_now();
    let (worker, worker_id) = crafter(&mut h, "Worker");
    let id = orders::place(h.app.world_mut(), h.player, request(0), now).unwrap();
    assert_eq!(
        orders::browse(h.app.world(), worker, 197, 0, false, now)
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
    h.advance(400);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let customer = persistence::mail_store::load_mailbox(db, h.character_id);
    assert_eq!(customer.len(), 1);
    assert_eq!(customer[0].items[0].item.item_id, OUTPUT);
    assert_eq!(customer[0].items[0].item.stack_count, 1);
    let payment = persistence::mail_store::load_mailbox(db, worker_id);
    assert_eq!(payment.len(), 1);
    assert_eq!(payment[0].money, 1000);
    assert_eq!(orders::all(db).unwrap()[0].status, 2);
    assert!(persistence::load_inventory(db, worker_id).items.is_empty());
    assert!(orders::begin_fulfill(h.app.world_mut(), worker, id, now).is_err());
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
        orders::browse(h.app.world(), outsider, 197, 1, false, now)
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
        orders::browse(h.app.world(), outsider, 197, 2, false, now)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        orders::browse(h.app.world(), worker, 197, 2, false, now)
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
        assert_eq!(
            mails[0]
                .items
                .iter()
                .map(|v| v.item.clone())
                .collect::<Vec<_>>(),
            before.items
        );
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
