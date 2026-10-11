use super::*;
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use shared::loot::{LootMode, LootRollChoice};
use shared::protocol::{GroupLootChoice, GroupLootRoll};

const MOSS_AGATE: u32 = 1206;

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
