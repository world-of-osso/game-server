//! Source MaxCount is ownership, not stack size or an equipped-only limit.
use super::inventory_model::InventoryModel;
use crate::bank_test_harness::harness;
use crate::item_templates::{ItemTemplate, ItemTemplates};
use crate::persistence::{self, InventoryItemData};
use shared::item_data::ItemDefinitionSource::Retail;

const UNIQUE_PRODUCT: u32 = 202310;

fn product(guid: u64) -> InventoryItemData {
    InventoryItemData {
        item_guid: guid,
        item_id: UNIQUE_PRODUCT,
        name: "Defective Doomsday Device".into(),
        quality: 0,
        required_level: 0,
        stack_count: 1,
        vendor_sell_price: 0,
        unique_auction: false,
    }
}

fn assert_unique_refusal(model: &mut InventoryModel, template: ItemTemplate) {
    let mut mint = || panic!("a refused unique product must not allocate another stack GUID");
    let error = model
        .store_item(product(9002), Retail, template, &mut mint)
        .expect_err("source MaxCount1 must reject a second owned product");
    assert_eq!(
        error.reason,
        shared::protocol::InventoryErrorReason::ItemMaxCount
    );
    assert_eq!(
        error.reason.message(),
        "You can't carry any more of those items."
    );
}

#[test]
fn inventory_max_count_refuses_second_bag_product_with_original_guid_and_record_unchanged() {
    let h = harness().expect("world-backed inventory harness");
    let templates = h.app.world().resource::<ItemTemplates>();
    let catalog = crate::merchant::load_vendor_catalog(&h.conn).unwrap();
    crate::inventory::grant_item(
        h.db(),
        templates,
        &catalog,
        h.character_id,
        (UNIQUE_PRODUCT, 1),
    )
    .unwrap();
    let before =
        persistence::inventory_store::load_inventory_record(h.db(), h.character_id).unwrap();
    let original = before
        .inventory
        .items
        .iter()
        .find(|item| item.item_id == UNIQUE_PRODUCT)
        .unwrap()
        .item_guid;
    let mut model = super::load_model(h.db(), templates, h.character_id).unwrap();
    let template = *templates.get(Retail, UNIQUE_PRODUCT).unwrap();
    assert_unique_refusal(&mut model, template);
    assert_eq!(model.into_record(), before);
    assert_eq!(h.guid_of(UNIQUE_PRODUCT), original);
    assert_eq!(h.count_of(UNIQUE_PRODUCT), 1);
}

#[test]
fn inventory_max_count_counts_closed_personal_bank_without_authorizing_bank_moves() {
    use crate::bank_storage::BankTabData;
    use crate::persistence::banks::{BankData, BankOwner};
    let h = harness().expect("world-backed inventory harness");
    let templates = h.app.world().resource::<ItemTemplates>();
    let bank = BankData {
        tabs: vec![BankTabData {
            slots: vec![Some(product(9001))],
            ..BankTabData::new("Unique products".into())
        }],
        ..Default::default()
    };
    persistence::banks::save_bank_move(
        h.db(),
        h.character_id,
        BankOwner::Character(h.character_id, &bank),
        None,
    );
    let before =
        persistence::inventory_store::load_inventory_record(h.db(), h.character_id).unwrap();
    let mut model = super::load_model(h.db(), templates, h.character_id).unwrap();
    assert_unique_refusal(&mut model, *templates.get(Retail, UNIQUE_PRODUCT).unwrap());
    assert_eq!(model.into_record(), before);
    assert_eq!(
        persistence::banks::load_character_bank(h.db(), h.character_id),
        bank
    );
    let error = super::apply_item_move(h.db(), templates, h.character_id, true, false, |_| Ok(()))
        .expect_err("bank ownership visibility must not authorize a bank move");
    assert_eq!(
        error.reason,
        shared::protocol::InventoryErrorReason::CantDoRightNow
    );
}

#[test]
fn inventory_max_count_targeted_and_starter_store_refuse_atomically() {
    use shared::protocol::{InventoryErrorReason, ItemLocation};
    for targeted in [true, false] {
        let h = harness().expect("world-backed inventory harness");
        let templates = h.app.world().resource::<ItemTemplates>();
        let catalog = crate::merchant::load_vendor_catalog(&h.conn).unwrap();
        crate::inventory::grant_item(
            h.db(),
            templates,
            &catalog,
            h.character_id,
            (UNIQUE_PRODUCT, 1),
        )
        .unwrap();
        let before =
            persistence::inventory_store::load_inventory_record(h.db(), h.character_id).unwrap();
        let mut model = super::load_model(h.db(), templates, h.character_id).unwrap();
        let mut template = *templates.get(Retail, UNIQUE_PRODUCT).unwrap();
        template.inventory_type = 21;
        let mut mint = || panic!("refused creation must not mint a split GUID");
        let result = if targeted {
            model.store_item_at(
                product(9002),
                Retail,
                template,
                ItemLocation::Bag { bag: 0, slot: 1 },
                &mut mint,
            )
        } else {
            model.store_new_item_in_best_slots(product(9002), Retail, template, &mut mint)
        };
        assert_eq!(
            result.unwrap_err().reason,
            InventoryErrorReason::ItemMaxCount
        );
        assert_eq!(model.into_record(), before);
    }
}

#[test]
fn inventory_max_count_nonpositive_caps_remain_unlimited() {
    for maximum in [0, -1] {
        let h = harness().expect("world-backed inventory harness");
        let templates = h.app.world().resource::<ItemTemplates>();
        let mut template = *templates.get(Retail, UNIQUE_PRODUCT).unwrap();
        template.max_count = maximum;
        let mut model = super::load_model(h.db(), templates, h.character_id).unwrap();
        let mut mint = || panic!("single-unit stacks keep their provided GUID");
        model
            .store_item(product(9001), Retail, template, &mut mint)
            .unwrap();
        model
            .store_item(product(9002), Retail, template, &mut mint)
            .unwrap();
        let record = model.into_record();
        let products: Vec<_> = record
            .inventory
            .items
            .iter()
            .filter(|item| item.item_id == UNIQUE_PRODUCT)
            .map(|item| (item.item_guid, item.stack_count))
            .collect();
        assert_eq!(products, vec![(9001, 1), (9002, 1)]);
    }
}

#[test]
fn inventory_max_count_own_bank_withdrawal_keeps_one_guid_and_one_owned_unit() {
    use crate::bank_storage::BankTabData;
    use crate::persistence::banks::{BankData, BankOwner};
    let h = harness().expect("world-backed inventory harness");
    let templates = h.app.world().resource::<ItemTemplates>();
    let bank = BankData {
        tabs: vec![BankTabData {
            slots: vec![Some(product(9001))],
            ..BankTabData::new("Unique products".into())
        }],
        ..Default::default()
    };
    persistence::banks::save_bank_move(
        h.db(),
        h.character_id,
        BankOwner::Character(h.character_id, &bank),
        None,
    );
    let mut model = super::load_model(h.db(), templates, h.character_id).unwrap();
    let mut mint = || panic!("own bank move must preserve its GUID");
    model
        .store_item(
            product(9001),
            Retail,
            *templates.get(Retail, UNIQUE_PRODUCT).unwrap(),
            &mut mint,
        )
        .unwrap();
    assert_eq!(model.owned_count(Retail, UNIQUE_PRODUCT), 1);
    assert_eq!(
        model
            .into_record()
            .inventory
            .items
            .iter()
            .find(|item| item.item_id == UNIQUE_PRODUCT)
            .unwrap()
            .item_guid,
        9001
    );
}
