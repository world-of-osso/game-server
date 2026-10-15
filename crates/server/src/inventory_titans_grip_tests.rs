//! TC a352b1fa Player.cpp9275–9281,10866–10905,13142–13215.
use super::*;
use std::sync::OnceLock;

const TITANS_GRIP: u32 = 46917;
const RESTRICTED_GRIP: u32 = 120553;
const BATTLEAXE: u32 = 12282;
const STAFF: u32 = 35;

fn grip_inventory(spells: &[u32], items: &[u32]) -> (App, Client) {
    static NATIVE: OnceLock<ItemTemplates> = OnceLock::new();
    let native = NATIVE.get_or_init(|| {
        crate::item_templates::load_item_templates(&crate::test_data::world_db_path())
            .expect("native item definitions")
    });
    let mut app = setup_app(persistence::temp_database());
    for &id in items {
        app.world_mut()
            .resource_mut::<ItemTemplates>()
            .retail
            .insert(id, native.retail[&id]);
    }
    let client = spawn_client(&mut app, 1, "Fury");
    learn_spells(&mut app, &client, spells);
    for &id in items {
        grant_bag_item(&app, client.character_id, id, "Native weapon");
    }
    enter_world(&mut app, &client);
    (app, client)
}

fn equip_main_hand(app: &mut App, client: &Client, from: ItemLocation) {
    request(
        app,
        client,
        SwapItem {
            from,
            to: ItemLocation::Equipment(EquipmentSlot::MainHand),
        },
    );
    assert!(received::<InventoryError>(app, client).is_empty());
}

fn assert_both_axes(app: &App, client: &Client) {
    let record = load_inventory_record(db(app), client.character_id).unwrap();
    let hands: Vec<_> = record
        .equipped
        .iter()
        .filter(|entry| {
            matches!(
                entry.slot,
                EquipmentVisualSlot::MainHand | EquipmentVisualSlot::OffHand
            )
        })
        .collect();
    assert_eq!(hands.len(), 2, "{:?}", record.equipped);
    assert!(hands.iter().all(|entry| entry.item.item_id == BATTLEAXE));
    assert_ne!(hands[0].item.item_guid, hands[1].item.item_guid);
    assert!(
        record
            .inventory
            .items
            .iter()
            .all(|item| item.item_id != BATTLEAXE)
    );
}

#[test]
fn titans_grip_equips_two_native_two_handers_in_either_order() {
    for off_hand_first in [false, true] {
        let (mut app, client) = grip_inventory(&[DUAL_WIELD, TITANS_GRIP], &[BATTLEAXE, BATTLEAXE]);
        if off_hand_first {
            assert!(equip_off_hand(&mut app, &client, bag(4)).is_empty());
            equip_main_hand(&mut app, &client, bag(3));
        } else {
            equip_main_hand(&mut app, &client, bag(3));
            assert!(equip_off_hand(&mut app, &client, bag(4)).is_empty());
        }
        assert_both_axes(&app, &client);
    }
}

#[test]
fn titans_grip_auto_equip_retains_the_main_hand() {
    let (mut app, client) = grip_inventory(&[DUAL_WIELD, TITANS_GRIP], &[BATTLEAXE, BATTLEAXE]);
    equip_main_hand(&mut app, &client, bag(3));
    request(&mut app, &client, EquipItem { from: bag(4) });
    assert!(received::<InventoryError>(&mut app, &client).is_empty());
    assert_both_axes(&app, &client);
}

#[test]
fn titans_grip_requires_both_learned_capabilities() {
    for spells in [&[][..], &[DUAL_WIELD][..], &[TITANS_GRIP][..]] {
        let (mut app, client) = grip_inventory(spells, &[BATTLEAXE]);
        let before = persistence::load_inventory(db(&app), client.character_id);
        assert_eq!(equip_off_hand(&mut app, &client, bag(3)).len(), 1);
        assert_eq!(off_hand_item(&app, client.character_id), None);
        assert_eq!(
            persistence::load_inventory(db(&app), client.character_id),
            before
        );
    }
}

#[test]
fn titans_grip_honors_native_subclass_restrictions() {
    for (item, permitted) in [(BATTLEAXE, true), (STAFF, false)] {
        let (mut app, client) = grip_inventory(&[DUAL_WIELD, RESTRICTED_GRIP], &[item]);
        let errors = equip_off_hand(&mut app, &client, bag(3));
        assert_eq!(errors.is_empty(), permitted, "item {item}: {errors:?}");
        assert_eq!(
            off_hand_item(&app, client.character_id),
            permitted.then_some(item)
        );
    }
}

#[test]
fn titans_grip_never_offers_a_ranged_weapon_to_the_off_hand() {
    const WORN_SHORTBOW: u32 = 2504;
    let (mut app, client) = grip_inventory(&[DUAL_WIELD, TITANS_GRIP], &[WORN_SHORTBOW]);
    let before = persistence::load_inventory(db(&app), client.character_id);
    let errors = equip_off_hand(&mut app, &client, bag(3));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].reason, InventoryErrorReason::WrongSlot);
    assert_eq!(off_hand_item(&app, client.character_id), None);
    assert_eq!(
        persistence::load_inventory(db(&app), client.character_id),
        before
    );
}

#[test]
fn titans_grip_unlearning_is_observed_by_the_next_request() {
    let (mut app, client) = grip_inventory(&[DUAL_WIELD, TITANS_GRIP], &[BATTLEAXE, BATTLEAXE]);
    equip_main_hand(&mut app, &client, bag(3));
    learn_spells(&mut app, &client, &[DUAL_WIELD]);
    let before = persistence::load_inventory(db(&app), client.character_id);
    assert_eq!(equip_off_hand(&mut app, &client, bag(4)).len(), 1);
    assert_eq!(off_hand_item(&app, client.character_id), None);
    assert_eq!(
        persistence::load_inventory(db(&app), client.character_id),
        before
    );
}
