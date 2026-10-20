use super::*;
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use shared::loot::{LootMode, LootRollChoice};
use shared::protocol::{GroupLootChoice, GroupLootRoll};

const MOSS_AGATE: u32 = 1206;

#[test]
fn gathering_group_rolls_and_shared_slots_block_skinning() {
    let connection = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(crate::gathering::GatheringPlugin);
    app.insert_resource(crate::gathering::load_data(&connection).unwrap());
    let corpse = app
        .world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Npc {
                template_id: 525,
                name: "Mangy Wolf".into(),
            },
            Health {
                current: 0.0,
                max: 100.0,
            },
            Corpse { remaining: 60.0 },
            CorpseLoot::default(),
            group::CorpseGroupLoot(vec![group::GroupPool {
                members: vec![30, 31],
                slots: Vec::new(),
                pending: 1,
            }]),
        ))
        .id();
    app.update();
    assert!(
        app.world()
            .get::<shared::protocol::GatherableResource>(corpse)
            .is_none(),
        "pending group rolls must not advertise Skinning"
    );
    {
        let mut shared = app
            .world_mut()
            .get_mut::<group::CorpseGroupLoot>(corpse)
            .unwrap();
        shared.0[0].pending = 0;
        shared.0[0]
            .slots
            .push(Some(LootContent::Money { copper: 3 }));
    }
    app.update();
    assert!(
        app.world()
            .get::<shared::protocol::GatherableResource>(corpse)
            .is_none(),
        "untaken shared slots still precede Skinning"
    );
    app.world_mut()
        .get_mut::<group::CorpseGroupLoot>(corpse)
        .unwrap()
        .0[0]
        .slots[0] = None;
    app.update();
    assert_eq!(
        app.world()
            .get::<shared::protocol::GatherableResource>(corpse),
        Some(&shared::protocol::GatherableResource { parent_skill: 393 })
    );
}

#[test]
fn achievements_criteria_loot_money_group_shares_are_actual_awards_and_cap_failure_is_atomic() {
    let (mut h, other) = grouped();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    bevy::app::Plugin::build(&crate::achievement::AchievementPlugin, &mut h.app);
    for (looter, account) in [(h.looter, 1), (other, 2)] {
        h.app
            .world_mut()
            .entity_mut(looter.connection)
            .insert(crate::auth::Authenticated {
                account_id: account,
            });
        let player = looter.player;
        h.app
            .world_mut()
            .entity_mut(player)
            .insert(crate::achievement::PlayerAchievementState::default());
    }
    let corpse = h.kill_kobold();
    h.open(corpse, true);
    for character in [h.looter.character_id, other.character_id] {
        let db = &h.app.world().resource::<GameDatabase>().db;
        assert_eq!(persistence::load_inventory(db, character).gold, 5);
        assert_eq!(
            persistence::load_achievements(db, character)
                .criteria_progress
                .get(&4093),
            Some(&5)
        );
    }
    h.open(corpse, true);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let mut inventory = persistence::load_inventory(db, other.character_id);
    inventory.gold = MAX_MONEY;
    persistence::save_inventory(db, other.character_id, &inventory);
    let corpse2 = h.kill_kobold();
    h.open(corpse2, true);
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        persistence::load_inventory(db, h.looter.character_id).gold,
        5
    );
    assert_eq!(
        persistence::load_inventory(db, other.character_id).gold,
        MAX_MONEY
    );
    for character in [h.looter.character_id, other.character_id] {
        assert_eq!(
            persistence::load_achievements(db, character)
                .criteria_progress
                .get(&4093),
            Some(&5)
        );
    }
}

fn grouped() -> (Harness, Looter) {
    let mut h = harness(false).expect("canonical read-only world fixture");
    let other = spawn_looter(&mut h.app, "Other", 2);
    let online: OnlineView = [(h.looter, "Looter"), (other, "Other")]
        .into_iter()
        .map(|(looter, name)| {
            (
                looter.character_id,
                OnlineMember {
                    entity: looter.player.to_bits(),
                    name: name.into(),
                    class: 1,
                    level: 1,
                    portrait: Default::default(),
                    dungeon_difficulty: 1,
                    raid_difficulty: Default::default(),
                },
            )
        })
        .collect();
    let mut registry = h.app.world_mut().resource_mut::<GroupRegistry>();
    registry.sync_presence(&online);
    registry.invite(h.looter.character_id, "Other", &online);
    registry.respond_invite(other.character_id, true);
    registry.set_loot_method(h.looter.character_id, LootMode::GroupLoot);
    let mut tables = h.app.world_mut().resource_mut::<LootTables>();
    tables.money.insert(
        KOBOLD_VERMIN,
        tables::CreatureLoot {
            loot_id: KOBOLD_VERMIN,
            min_gold: 10,
            max_gold: 10,
        },
    );
    tables.creature.insert(
        KOBOLD_VERMIN,
        tables::LootTemplate {
            entries: vec![tables::LootRow {
                item: MOSS_AGATE,
                reference: 0,
                chance: 100.0,
                needs_quest: false,
                group: 0,
                min_count: 1,
                max_count: 1,
            }],
            groups: Vec::new(),
        },
    );
    h.app
        .world_mut()
        .insert_resource(group::GroupRollRng(Box::new(|| 42)));
    (h, other)
}

fn owned(h: &Harness, character: u64, item: u32) -> u32 {
    persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character)
        .items
        .iter()
        .filter(|entry| entry.item_id == item)
        .map(|entry| entry.stack_count)
        .sum()
}

#[test]
fn need_wins_delivers_once_and_one_coin_click_pays_both_members() {
    let (mut h, other) = grouped();
    let corpse = h.kill_kobold();
    let roll = h.take::<GroupLootRoll>().remove(0);
    h.send::<_, LootChannel>(
        h.looter,
        GroupLootChoice {
            roll_id: roll.roll_id,
            choice: LootRollChoice::Greed,
        },
    );
    assert_eq!(owned(&h, other.character_id, MOSS_AGATE), 0);
    h.send::<_, LootChannel>(
        other,
        GroupLootChoice {
            roll_id: roll.roll_id,
            choice: LootRollChoice::Need,
        },
    );
    assert_eq!(owned(&h, other.character_id, MOSS_AGATE), 1);
    assert_eq!(owned(&h, h.looter.character_id, MOSS_AGATE), 0);
    let resolved = h.take::<GroupLootRoll>();
    assert!(
        resolved
            .iter()
            .any(|roll| roll.complete && roll.delivered && roll.winner == Some(other.character_id))
    );
    h.send::<_, LootChannel>(
        other,
        GroupLootChoice {
            roll_id: roll.roll_id,
            choice: LootRollChoice::Need,
        },
    );
    assert_eq!(owned(&h, other.character_id, MOSS_AGATE), 1);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let before_a = persistence::load_inventory(db, h.looter.character_id).gold;
    let before_b = persistence::load_inventory(db, other.character_id).gold;
    h.open(corpse, true);
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        persistence::load_inventory(db, h.looter.character_id).gold,
        before_a + 5
    );
    assert_eq!(
        persistence::load_inventory(db, other.character_id).gold,
        before_b + 5
    );
    h.send::<_, LootChannel>(
        other,
        LootUnit {
            corpse: corpse.to_bits(),
            auto: true,
        },
    );
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        persistence::load_inventory(db, other.character_id).gold,
        before_b + 5
    );
}

#[test]
fn all_pass_unlocks_one_shared_item_for_exactly_one_looter() {
    let (mut h, other) = grouped();
    let corpse = h.kill_kobold();
    let roll = h.take::<GroupLootRoll>().remove(0);
    for looter in [h.looter, other] {
        h.send::<_, LootChannel>(
            looter,
            GroupLootChoice {
                roll_id: roll.roll_id,
                choice: LootRollChoice::Pass,
            },
        );
    }
    let resolved = h.take::<GroupLootRoll>();
    assert!(
        resolved
            .iter()
            .any(|roll| roll.complete && roll.winner.is_none())
    );
    h.open(corpse, true);
    h.send::<_, LootChannel>(
        other,
        LootUnit {
            corpse: corpse.to_bits(),
            auto: true,
        },
    );
    assert_eq!(owned(&h, h.looter.character_id, MOSS_AGATE), 1);
    assert_eq!(owned(&h, other.character_id, MOSS_AGATE), 0);
}

#[test]
fn greed_only_item_disables_need_for_every_member() {
    let (mut h, _) = grouped();
    h.app
        .world_mut()
        .resource_mut::<ItemTemplates>()
        .retail
        .get_mut(&MOSS_AGATE)
        .unwrap()
        .scaling
        .flags2 |= 0x100; // ITEM_FLAG2_CAN_ONLY_ROLL_GREED
    h.kill_kobold();
    let roll = h.take::<GroupLootRoll>().remove(0);
    assert!(roll.players.iter().all(|player| !player.can_need
        && player.need_reason.as_deref() == Some("Need rolls are disabled for this item.")));
}

#[test]
fn cloth_need_uses_retail_class_armor_mask_not_required_equip_level() {
    let (mut h, other) = grouped();
    h.app
        .world_mut()
        .resource_mut::<LootTables>()
        .creature
        .get_mut(&KOBOLD_VERMIN)
        .unwrap()
        .entries[0]
        .item = 983; // Red Linen Sash: Retail Uncommon cloth (3985 is now class 15 misc)
    h.app
        .world_mut()
        .get_mut::<Player>(other.player)
        .unwrap()
        .class = 8;
    h.kill_kobold();
    let roll = h.take::<GroupLootRoll>().remove(0);
    assert!(
        !roll
            .players
            .iter()
            .find(|player| player.character_id == h.looter.character_id)
            .unwrap()
            .can_need
    );
    assert!(
        roll.players
            .iter()
            .find(|player| player.character_id == other.character_id)
            .unwrap()
            .can_need
    );
}

#[test]
fn timeout_keeps_the_real_decay_boundary_lootable_until_all_pass_items_are_claimed() {
    let (mut h, _) = grouped();
    h.app.add_systems(
        Update,
        crate::combat::corpse_decay_system
            .after(generate_corpse_loot)
            .before(group::process_rolls),
    );
    let corpse = h.kill_kobold();
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(Duration::from_secs(90));
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs(61),
        ));
    h.app.update();
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::ZERO,
        ));
    h.settle();
    assert!(
        h.app.world().get::<Corpse>(corpse).is_some(),
        "pending rolls hold the corpse's decay boundary"
    );
    let completed = h.take::<GroupLootRoll>();
    assert!(completed.iter().any(|roll| roll.complete
        && roll.winner.is_none()
        && roll.players.iter().all(|player| {
            player
                .response
                .is_some_and(|entry| entry.choice == LootRollChoice::Pass)
        })));
    h.open(corpse, true);
    assert_eq!(h.count_of(MOSS_AGATE), 1);
}

#[test]
fn ineligible_need_is_refused_without_auto_passing_the_member() {
    let (mut h, other) = grouped();
    h.app
        .world_mut()
        .resource_mut::<ItemTemplates>()
        .retail
        .get_mut(&MOSS_AGATE)
        .unwrap()
        .allowable_class = 1;
    h.app
        .world_mut()
        .get_mut::<Player>(other.player)
        .unwrap()
        .class = 8;
    h.kill_kobold();
    let roll = h.take::<GroupLootRoll>().remove(0);
    let guest = roll
        .players
        .iter()
        .find(|player| player.character_id == other.character_id)
        .unwrap();
    assert!(!guest.can_need);
    assert_eq!(
        guest.need_reason.as_deref(),
        Some("Your class may not roll need on this item.")
    );
    h.send::<_, LootChannel>(
        other,
        GroupLootChoice {
            roll_id: roll.roll_id,
            choice: LootRollChoice::Need,
        },
    );
    h.send::<_, LootChannel>(
        h.looter,
        GroupLootChoice {
            roll_id: roll.roll_id,
            choice: LootRollChoice::Pass,
        },
    );
    assert_eq!(owned(&h, other.character_id, MOSS_AGATE), 0);
    h.send::<_, LootChannel>(
        other,
        GroupLootChoice {
            roll_id: roll.roll_id,
            choice: LootRollChoice::Greed,
        },
    );
    assert_eq!(owned(&h, other.character_id, MOSS_AGATE), 1);
}

#[test]
fn completed_last_roll_removes_the_corpses_group_sparkles() {
    let (mut h, other) = grouped();
    let corpse = h.kill_kobold();
    let roll = h.take::<GroupLootRoll>().remove(0);
    h.open(corpse, true); // claim shared coins while the item is still blocked
    h.take::<CorpseLootable>();
    h.send::<_, LootChannel>(
        h.looter,
        GroupLootChoice {
            roll_id: roll.roll_id,
            choice: LootRollChoice::Greed,
        },
    );
    h.send::<_, LootChannel>(
        other,
        GroupLootChoice {
            roll_id: roll.roll_id,
            choice: LootRollChoice::Need,
        },
    );
    assert_eq!(
        h.take::<CorpseLootable>(),
        [lootable(corpse, false), lootable(corpse, false)]
    );
}

#[test]
fn transmog_prefers_appropriate_armor_class_over_higher_random_roll() {
    const RAGECLAW_BOOTS: u32 = 15379;
    let (mut h, other) = grouped();
    for (looter, account_id) in [(h.looter, 1), (other, 2)] {
        h.app
            .world_mut()
            .entity_mut(looter.connection)
            .insert(crate::auth::Authenticated { account_id });
    }
    // Retail leather: Night Elf Druid preferred; Human Warrior can collect it.
    {
        let mut druid = h.app.world_mut().get_mut::<Player>(other.player).unwrap();
        druid.class = 11;
        druid.race = 4;
    }
    h.app
        .world_mut()
        .resource_mut::<LootTables>()
        .creature
        .get_mut(&KOBOLD_VERMIN)
        .unwrap()
        .entries[0]
        .item = RAGECLAW_BOOTS;
    let mut values = std::collections::VecDeque::from([99, 1]);
    h.app
        .world_mut()
        .insert_resource(group::GroupRollRng(Box::new(move || {
            values
                .pop_front()
                .expect("two authoritative transmog rolls")
        })));
    h.kill_kobold();
    let roll = h.take::<GroupLootRoll>().remove(0);
    assert!(roll.players.iter().all(|player| player.can_transmog));
    for looter in [h.looter, other] {
        h.send::<_, LootChannel>(
            looter,
            GroupLootChoice {
                roll_id: roll.roll_id,
                choice: LootRollChoice::Transmog,
            },
        );
    }
    assert_eq!(owned(&h, other.character_id, RAGECLAW_BOOTS), 1);
    assert_eq!(owned(&h, h.looter.character_id, RAGECLAW_BOOTS), 0);
    assert!(h.take::<GroupLootRoll>().iter().any(|roll| {
        roll.complete && roll.delivered && roll.winner == Some(other.character_id)
    }));
}

#[test]
fn transmog_beats_higher_greed_and_completed_history_replays_to_new_connection() {
    const RAGECLAW_BOOTS: u32 = 15379;
    const BOOTS_APPEARANCE: u32 = 2478;
    let (mut h, other) = grouped();
    for (looter, account_id) in [(h.looter, 1), (other, 2)] {
        h.app
            .world_mut()
            .entity_mut(looter.connection)
            .insert(crate::auth::Authenticated { account_id });
    }
    let mut learned = shared::transmog::AppearanceCollection::default();
    learned.learn_many([BOOTS_APPEARANCE]);
    crate::persistence::appearance_store::save_account_appearances(
        &h.app.world().resource::<GameDatabase>().db,
        1,
        &learned,
    );
    h.app
        .world_mut()
        .resource_mut::<LootTables>()
        .creature
        .get_mut(&KOBOLD_VERMIN)
        .unwrap()
        .entries[0]
        .item = RAGECLAW_BOOTS;
    let mut values = std::collections::VecDeque::from([99, 1]);
    h.app
        .world_mut()
        .insert_resource(group::GroupRollRng(Box::new(move || {
            values.pop_front().expect("two authoritative rolls")
        })));
    h.kill_kobold();
    let roll = h.take::<GroupLootRoll>().remove(0);
    assert!(
        !roll
            .players
            .iter()
            .find(|p| p.character_id == h.looter.character_id)
            .unwrap()
            .can_transmog
    );
    assert!(
        roll.players
            .iter()
            .find(|p| p.character_id == other.character_id)
            .unwrap()
            .can_transmog
    );
    h.send::<_, LootChannel>(
        h.looter,
        GroupLootChoice {
            roll_id: roll.roll_id,
            choice: LootRollChoice::Greed,
        },
    );
    h.send::<_, LootChannel>(
        other,
        GroupLootChoice {
            roll_id: roll.roll_id,
            choice: LootRollChoice::Transmog,
        },
    );
    assert_eq!(owned(&h, h.looter.character_id, RAGECLAW_BOOTS), 0);
    assert_eq!(owned(&h, other.character_id, RAGECLAW_BOOTS), 1);
    let completed = h
        .take::<GroupLootRoll>()
        .into_iter()
        .find(|r| r.complete)
        .unwrap();
    assert_eq!(completed.winner, Some(other.character_id));
    assert!(completed.delivered);
    h.app.world_mut().despawn(other.connection);
    let connection = h
        .app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(2)),
            Connected,
            HostClient { buffer: Vec::new() },
            InWorld {
                character_id: other.character_id,
            },
            PlayerLink(other.player),
            crate::auth::Authenticated { account_id: 2 },
            pair::<shared::protocol::GroupLootHistoryRequest>(),
            pair::<GroupLootRoll>(),
        ))
        .id();
    let reconnected = Looter {
        connection,
        ..other
    };
    h.send::<_, LootChannel>(reconnected, shared::protocol::GroupLootHistoryRequest);
    let history = h.take::<GroupLootRoll>();
    assert_eq!(history, [completed]);
    assert_eq!(owned(&h, other.character_id, RAGECLAW_BOOTS), 1);
}

#[test]
fn group_kill_broadcasts_one_shared_item_roll_not_two_personal_items() {
    let (mut h, other) = grouped();
    let corpse = h.kill_kobold();
    let rolls = h.take::<GroupLootRoll>();
    assert_eq!(rolls.len(), 2, "both eligible members receive the roll");
    assert_eq!(rolls[0].roll_id, rolls[1].roll_id);
    assert_eq!(rolls[0].players.len(), 2);
    assert!(!rolls[0].complete);
    assert!(
        rolls[0]
            .players
            .iter()
            .any(|p| p.character_id == other.character_id)
    );
    let loot = h.app.world().get::<CorpseLoot>(corpse).unwrap();
    assert!(!loot.looters.values().flatten().any(|content| matches!(
        content,
        Some(LootContent::Item {
            item_id: MOSS_AGATE,
            ..
        })
    )));
}

#[test]
#[ignore = "aura-world native RED/GREEN gate"]
fn aura_world_group_loot_money_modifies_each_recipients_share_before_cap() {
    let (mut h, other) = grouped();
    let auras = crate::auras::world_xp_tests::native_auras(291619);
    h.app.world_mut().entity_mut(h.looter.player).insert(auras);
    {
        let mut tables = h.app.world_mut().resource_mut::<LootTables>();
        let money = tables.money.get_mut(&KOBOLD_VERMIN).unwrap();
        money.min_gold = 1000;
        money.max_gold = 1000;
    }
    let corpse = h.kill_kobold();
    h.open(corpse, true);
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        persistence::load_inventory(db, h.looter.character_id).gold,
        510
    );
    assert_eq!(
        persistence::load_inventory(db, other.character_id).gold,
        500
    );
    let mut inventory = persistence::load_inventory(db, h.looter.character_id);
    inventory.gold = MAX_MONEY - 505;
    persistence::save_inventory(db, h.looter.character_id, &inventory);
    let corpse = h.kill_kobold();
    h.open(corpse, true);
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        persistence::load_inventory(db, h.looter.character_id).gold,
        MAX_MONEY - 505
    );
    assert_eq!(
        persistence::load_inventory(db, other.character_id).gold,
        500,
        "bonus cap failure must preserve peer wallet"
    );
}
