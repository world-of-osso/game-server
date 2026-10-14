//! Authentic local Retail candidates, spell data and TDB material ranges.
use super::test_harness::{Harness, harness_with_plugins};
use crate::persistence::{self, ClientId, GameDatabase};
use crate::profession::PlayerProfessions;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::CombatStatus;
use shared::item_data::ItemDefinitionSource::Retail;
use shared::protocol::{
    CombatChannel, LootChannel, LootContent, LootResponse, LootUnit, SpellCastIntent,
};
use shared::spell_data::CastFailReason;

#[derive(Resource, Default)]
struct LootInbox(Vec<LootResponse>);

fn collect_loot(
    mut inbox: ResMut<LootInbox>,
    mut receivers: Query<&mut MessageReceiver<LootResponse>>,
) {
    for mut receiver in &mut receivers {
        inbox.0.extend(receiver.receive());
    }
}

fn setup(item_id: u32, enchanting: bool) -> (Harness, tempfile::TempDir, u64, u32) {
    setup_with(item_id, enchanting, |_| {})
}

fn setup_with(
    item_id: u32,
    enchanting: bool,
    add_plugins: impl FnOnce(&mut App),
) -> (Harness, tempfile::TempDir, u64, u32) {
    let path = crate::test_data::world_db_path();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let mut h = harness_with_plugins(Vec::new(), &[13262], 2, false, |app| {
        add_plugins(app);
        app.add_plugins(crate::loot::LootPlugin);
        app.insert_resource(super::disenchant::load_data(&conn).unwrap());
        app.insert_resource(crate::loot::load_loot_tables(&conn).unwrap());
        app.insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
        app.insert_resource(crate::merchant::load_vendor_catalog(&conn).unwrap());
        app.init_resource::<crate::gold::GoldUpdateQueue>();
        app.init_resource::<LootInbox>()
            .add_systems(First, collect_loot);
    })
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("game.redb")).unwrap();
    persistence::create_tables(&db);
    let (id, _) = persistence::create_character(&db, 1, "Disenchanter", 1, 2).unwrap();
    assert_eq!(id, 1);
    let templates = h
        .app
        .world()
        .resource::<crate::item_templates::ItemTemplates>();
    let appearance = templates
        .get(Retail, item_id)
        .unwrap()
        .appearance_id
        .unwrap_or(0);
    crate::inventory::grant_item(
        &db,
        templates,
        h.app.world().resource::<crate::merchant::VendorCatalog>(),
        id,
        (item_id, 1),
    )
    .unwrap();
    let model = crate::inventory::load_model(&db, templates, id).unwrap();
    let guid = model
        .bag_items()
        .find(|item| item.item_id == item_id)
        .unwrap()
        .item_guid;
    assert!(!model.is_soulbound(guid), "candidate must be unbound");
    h.app.insert_resource(GameDatabase::new(db));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((ClientId(id), CombatStatus(false)));
    // The harness grants class spells; Disenchant must instead come from Enchanting.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    if enchanting {
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(PlayerProfessions {
                lines: vec![shared::profession::ProfessionSkillLine {
                    skill_line: 333,
                    step: 1,
                    rank: 1,
                    max_rank: 300,
                }],
                spells: vec![13262],
            });
    }
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<LootResponse>::default(),
        MessageReceiver::<LootResponse>::default(),
        MessageSender::<LootUnit>::default(),
        MessageReceiver::<LootUnit>::default(),
        MessageSender::<shared::protocol::LootSlotRequest>::default(),
        MessageReceiver::<shared::protocol::LootSlotRequest>::default(),
        MessageSender::<shared::protocol::LootRelease>::default(),
        MessageReceiver::<shared::protocol::LootRelease>::default(),
        MessageSender::<shared::protocol::LootClosed>::default(),
        MessageSender::<shared::protocol::LootSlotRemoved>::default(),
        MessageSender::<shared::protocol::LootFailed>::default(),
        MessageSender::<shared::protocol::CorpseLootable>::default(),
    ));
    (h, directory, guid, appearance)
}

fn cast_item(h: &mut Harness, guid: u64) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<SpellCastIntent>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(SpellCastIntent {
            spell_id: Some(13262),
            spell: String::new(),
            target_entity: None,
            target_item_guid: Some(guid),
            destination: None,
            witness: None,
        });
    h.advance(100);
}

fn owns(h: &Harness, guid: u64) -> bool {
    crate::inventory::load_model(
        &h.app.world().resource::<GameDatabase>().db,
        h.app
            .world()
            .resource::<crate::item_templates::ItemTemplates>(),
        1,
    )
    .unwrap()
    .bag_item(guid)
    .is_some()
}

#[test]
fn disenchant_unbound_uncommon_rolls_real_materials_and_persists_appearance() {
    // Notched Shortsword727: uncommon BoE, Retail eligibility ID3 (not legacy21).
    let (mut h, directory, guid, appearance) = setup(727, true);
    assert!(appearance > 0);
    assert!(
        !persistence::appearance_store::load_account_appearances(
            &h.app.world().resource::<GameDatabase>().db,
            1
        )
        .has(appearance)
    );
    cast_item(&mut h, guid);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(!owns(&h, guid), "Disenchant must destroy the source item");
    let inbox = h.app.world().resource::<LootInbox>();
    let response = inbox.0.last().expect("material loot window").clone();
    assert!(!response.slots.is_empty());
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    for slot in &response.slots {
        let LootContent::Item { item_id, count, .. } = slot.content else {
            panic!("Disenchant yields materials, not coins")
        };
        let (min, max): (u32, u32) = conn.query_row("SELECT MinCount, MaxCount FROM tdb_disenchant_loot_template WHERE Entry=3 AND Item=?1", [item_id], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert!(
            (min..=max).contains(&count),
            "{item_id} count {count} outside authentic ranges"
        );
    }
    h.app
        .world_mut()
        .get_mut::<MessageSender<LootUnit>>(h.connection)
        .unwrap()
        .send::<LootChannel>(LootUnit {
            corpse: response.corpse,
            auto: true,
        });
    h.advance(5);
    let templates = h
        .app
        .world()
        .resource::<crate::item_templates::ItemTemplates>();
    let bags =
        crate::inventory::load_model(&h.app.world().resource::<GameDatabase>().db, templates, 1)
            .unwrap();
    for slot in &response.slots {
        let LootContent::Item { item_id, count, .. } = slot.content else {
            unreachable!()
        };
        assert_eq!(bags.bag_count(Retail, item_id), count);
    }
    drop(h);
    let db = redb::Database::open(directory.path().join("game.redb")).unwrap();
    assert!(persistence::appearance_store::load_account_appearances(&db, 1).has(appearance));
    assert!(
        persistence::load_inventory(&db, 1)
            .items
            .iter()
            .all(|item| item.item_guid != guid)
    );
}

#[test]
fn disenchant_materials_can_be_taken_manually() {
    let (mut h, _directory, guid, _) = setup(727, true);
    cast_item(&mut h, guid);
    let response = h
        .app
        .world()
        .resource::<LootInbox>()
        .0
        .last()
        .unwrap()
        .clone();
    for slot in &response.slots {
        h.app
            .world_mut()
            .get_mut::<MessageSender<shared::protocol::LootSlotRequest>>(h.connection)
            .unwrap()
            .send::<LootChannel>(shared::protocol::LootSlotRequest {
                corpse: response.corpse,
                slot: slot.slot,
            });
    }
    h.advance(5);
    let model = crate::inventory::load_model(
        &h.app.world().resource::<GameDatabase>().db,
        h.app
            .world()
            .resource::<crate::item_templates::ItemTemplates>(),
        1,
    )
    .unwrap();
    for slot in response.slots {
        let LootContent::Item { item_id, count, .. } = slot.content else {
            unreachable!()
        };
        assert_eq!(model.bag_count(Retail, item_id), count);
    }
    assert!(
        h.app
            .world()
            .get::<crate::loot::ActiveLoot>(h.connection)
            .is_none()
    );
}

/// TC Player::StoreLootItem: taken disenchant materials credit GetLootByType
/// LOOT_DISENCHANTING (criterion5304, statistic183 "Materials produced from disenchanting").
#[test]
fn achievements_criteria_disenchant_materials_credit_disenchant_loot_type_when_taken() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    let (mut h, _directory, guid, _) = setup_with(727, true, |app| {
        app.add_plugins(AchievementPlugin);
    });
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(PlayerAchievementState::default());
    let progress = |h: &Harness| {
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&5304)
            .copied()
    };
    cast_item(&mut h, guid);
    let response = h
        .app
        .world()
        .resource::<LootInbox>()
        .0
        .last()
        .unwrap()
        .clone();
    assert_eq!(progress(&h), None, "opening the window credits nothing");
    let mut materials = 0;
    for slot in &response.slots {
        let LootContent::Item { count, .. } = slot.content else {
            unreachable!()
        };
        materials += u64::from(count);
        h.app
            .world_mut()
            .get_mut::<MessageSender<shared::protocol::LootSlotRequest>>(h.connection)
            .unwrap()
            .send::<LootChannel>(shared::protocol::LootSlotRequest {
                corpse: response.corpse,
                slot: slot.slot,
            });
    }
    h.advance(5);
    assert!(materials > 0);
    assert_eq!(progress(&h), Some(materials));
}

#[test]
fn disenchant_release_finishes_personal_loot_and_allows_the_next_item() {
    // TC a352b1fa LootHandler.cpp:310-335 discards unclaimed non-openable item loot.
    let (mut h, _directory, guid, _) = setup(727, true);
    cast_item(&mut h, guid);
    let response = h
        .app
        .world()
        .resource::<LootInbox>()
        .0
        .last()
        .unwrap()
        .clone();
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::LootRelease>>(h.connection)
        .unwrap()
        .send::<LootChannel>(shared::protocol::LootRelease {
            corpse: response.corpse,
        });
    h.advance(5);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let templates = h
        .app
        .world()
        .resource::<crate::item_templates::ItemTemplates>();
    let model = crate::inventory::load_model(db, templates, 1).unwrap();
    for slot in response.slots {
        let LootContent::Item { item_id, .. } = slot.content else {
            unreachable!()
        };
        assert_eq!(model.bag_count(Retail, item_id), 0);
    }
    crate::inventory::grant_item(
        db,
        templates,
        h.app.world().resource::<crate::merchant::VendorCatalog>(),
        1,
        (727, 1),
    )
    .unwrap();
    let next = crate::inventory::load_model(db, templates, 1)
        .unwrap()
        .bag_items()
        .find(|item| item.item_id == 727)
        .unwrap()
        .item_guid;
    cast_item(&mut h, next);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(!owns(&h, next));
    assert_eq!(h.app.world().resource::<LootInbox>().0.len(), 2);
}

#[test]
fn disenchant_missing_retail_materials_preserves_item_and_allows_next_cast() {
    // Dragonmaw Shortsword753 selects TDB14; all three materials were removed
    // from Retail12.1.0.69933. Do not replace them with fabricated templates.
    let (mut h, _directory, guid, appearance) = setup(753, true);
    let templates = h
        .app
        .world()
        .resource::<crate::item_templates::ItemTemplates>();
    for material in [11082, 11083, 11084] {
        assert!(templates.get(Retail, material).is_none());
    }
    cast_item(&mut h, guid);
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::CantBeDisenchanted);
    assert!(owns(&h, guid));
    assert!(h.app.world().resource::<LootInbox>().0.is_empty());
    assert!(
        h.app
            .world()
            .get::<crate::loot::ItemLoot>(h.caster)
            .is_none()
    );
    assert!(
        h.app
            .world()
            .get::<crate::loot::ActiveLoot>(h.connection)
            .is_none()
    );
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert!(!persistence::appearance_store::load_account_appearances(db, 1).has(appearance));
    let templates = h
        .app
        .world()
        .resource::<crate::item_templates::ItemTemplates>();
    crate::inventory::grant_item(
        db,
        templates,
        h.app.world().resource::<crate::merchant::VendorCatalog>(),
        1,
        (727, 1),
    )
    .unwrap();
    let next = crate::inventory::load_model(db, templates, 1)
        .unwrap()
        .bag_items()
        .find(|item| item.item_id == 727)
        .unwrap()
        .item_guid;
    cast_item(&mut h, next);
    assert_eq!(
        h.inbox.failed.len(),
        1,
        "valid next cast must not be locked"
    );
    assert!(!owns(&h, next));
    assert!(owns(&h, guid));
    let response = h.app.world().resource::<LootInbox>().0.last().unwrap();
    assert!(!response.slots.is_empty());
}

#[test]
fn disenchant_partial_missing_group_rejects_an_empty_roll() {
    // Real TDB5 has 20%10939 + 75%10940, but its 5%10978 is absent.
    let (mut h, _directory, guid, appearance) = setup(1190, true);
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let tables = crate::loot::load_disenchant_tables(&conn).unwrap();
    let templates = h
        .app
        .world()
        .resource::<crate::item_templates::ItemTemplates>();
    assert!(tables.disenchant_has_materials(5, templates));
    let mut high_roll = rand::rngs::mock::StepRng::new(u64::MAX, 0);
    assert!(
        tables
            .roll_disenchant(5, templates, &mut high_roll)
            .is_none()
    );

    let mut spell = super::test_harness::world_db()
        .unwrap()
        .store
        .get(13262)
        .unwrap()
        .clone();
    spell.cast_target_item_guid = Some(guid);
    let mut high_roll = rand::rngs::mock::StepRng::new(u64::MAX, 0);
    let result =
        super::disenchant::roll_cast_loot(h.app.world_mut(), h.caster, &spell, &mut high_roll);
    assert!(matches!(
        result,
        Err(super::CastFailure::CantBeDisenchanted)
    ));
    assert!(owns(&h, guid));
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert!(!persistence::appearance_store::load_account_appearances(db, 1).has(appearance));
    assert!(h.app.world().resource::<LootInbox>().0.is_empty());
}

#[test]
fn disenchant_grey_refuses_and_keeps_item() {
    let (mut h, _directory, guid, _) = setup(25, true);
    cast_item(&mut h, guid);
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::CantBeDisenchanted
    );
    assert!(owns(&h, guid));
    assert!(h.app.world().resource::<LootInbox>().0.is_empty());
}

#[test]
fn disenchant_without_enchanting_refuses_and_keeps_item() {
    let (mut h, _directory, guid, _) = setup(727, false);
    cast_item(&mut h, guid);
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::NotKnown
    );
    assert!(owns(&h, guid));
}

#[test]
fn disenchant_rechecks_item_ownership_at_cast_completion() {
    let (mut h, _directory, guid, _) = setup(727, true);
    h.app
        .world_mut()
        .get_mut::<MessageSender<SpellCastIntent>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(SpellCastIntent {
            spell_id: Some(13262),
            spell: String::new(),
            target_entity: None,
            target_item_guid: Some(guid),
            destination: None,
            witness: None,
        });
    h.advance(4);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_some()
    );
    let db = &h.app.world().resource::<GameDatabase>().db;
    let mut model = crate::inventory::load_model(
        db,
        h.app
            .world()
            .resource::<crate::item_templates::ItemTemplates>(),
        1,
    )
    .unwrap();
    // Transfer, not disposal: no appearance receipt.
    model.take_bag_item(guid, 0, &mut || 999).unwrap();
    persistence::inventory_store::save_inventory_record(db, 1, &model.into_record());
    h.advance(100);
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::InvalidTarget
    );
    assert!(h.app.world().resource::<LootInbox>().0.is_empty());
}
