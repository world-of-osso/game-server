//! Real boss loot through death generation, client requests and inventory commits.
//! Corpse staging isolates the loot boundary; this is not complete-fight proof.
use super::*;

const RAGEFIRE_MAP: u32 = 389;
const SHADOWFANG_MAP: u32 = 33;
const SILVERLAINE: u32 = 3887;
const SILVERLAINE_BOSS_ITEMS: [u32; 2] = [6321, 6323];
const SILVERLAINE_POOL: [u32; 6] = [1205, 2589, 2592, 4606, 6321, 6323];

fn stage_encounter_corpse(h: &mut Harness, entry: u32, map_id: u32) -> Entity {
    let map = crate::maps::InMap {
        map_id,
        instance_id: 1,
    };
    h.app.world_mut().entity_mut(h.looter.player).insert(map);
    let corpse = h
        .app
        .world_mut()
        .spawn((
            map,
            Npc {
                template_id: entry,
                name: format!("Encounter loot fixture {entry}"),
            },
            Position {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 0.0,
                max: 100.0,
            },
            crate::zones::CreatureDifficulty(1),
            crate::creature_tap::CreatureTap::tapped_by(h.looter.character_id),
            RespawnDelay::for_spawn(7200, 1),
            CorpseDelay::for_rank(1),
        ))
        .id();
    mark_npc_dead_world(h.app.world_mut(), corpse);
    h.settle();
    corpse
}

fn read_corpse_rewards(h: &Harness, corpse: Entity) -> (u64, Vec<(u32, u32)>, usize) {
    let slots = &h.app.world().get::<CorpseLoot>(corpse).unwrap().looters[&h.looter.character_id];
    let copper = slots
        .iter()
        .flatten()
        .filter_map(|content| match content {
            LootContent::Money { copper } => Some(*copper),
            LootContent::Item { .. } => None,
        })
        .sum();
    let rewards = slots
        .iter()
        .flatten()
        .filter_map(|content| match content {
            LootContent::Item { item_id, count, .. } => Some((*item_id, *count)),
            LootContent::Money { .. } => None,
        })
        .collect();
    (copper, rewards, slots.len())
}

fn assert_authored_rewards(entry: u32, copper: u64, rewards: &[(u32, u32)]) {
    if entry == SILVERLAINE {
        // Current content alias20003887 preserves TDB3887's exact six rows.
        assert!((94..=279).contains(&copper));
        assert!(
            rewards
                .iter()
                .all(|(item, _)| SILVERLAINE_POOL.contains(item))
        );
        let boss_rewards: Vec<_> = rewards
            .iter()
            .filter(|(item, _)| SILVERLAINE_BOSS_ITEMS.contains(item))
            .collect();
        assert_eq!(boss_rewards.len(), 1, "authored100% group1 boss reward");
        assert_eq!(boss_rewards[0].1, 1);
    } else {
        assert!(
            (1..=2).contains(&rewards.len()),
            "{entry} has boss item rewards"
        );
        assert!(copper > 0, "{entry} has authored coins");
    }
}

fn take_all_slots(h: &mut Harness, corpse: Entity, slot_count: usize) {
    for slot in 0..slot_count {
        h.take_slot(corpse, slot as u8);
    }
}

fn assert_corpse_reward_commits_once(h: &mut Harness, entry: u32, map_id: u32) {
    let before = h.inventory();
    let corpse = stage_encounter_corpse(h, entry, map_id);
    let (copper, rewards, slot_count) = read_corpse_rewards(h, corpse);
    assert_authored_rewards(entry, copper, &rewards);
    let old_counts: Vec<_> = rewards.iter().map(|(item, _)| h.count_of(*item)).collect();
    h.open(corpse, false);
    assert!(h.failures().is_empty());
    take_all_slots(h, corpse, slot_count);
    assert!(h.failures().is_empty(), "{entry} corpse rewards admitted");
    let after = h.inventory();
    assert_eq!(after.gold, before.gold + copper);
    for ((item, count), old_count) in rewards.iter().zip(old_counts) {
        assert_eq!(
            h.count_of(*item),
            old_count + count,
            "{entry} item {item} persisted"
        );
    }
    assert!(corpse_loot_is_empty(h.app.world(), corpse));
    take_all_slots(h, corpse, slot_count);
    assert_eq!(
        h.inventory(),
        after,
        "retaking {entry} cannot duplicate rewards"
    );
    h.take::<LootFailed>();
}

#[test]
fn ragefire_completion_corpse_loot_commits_each_boss_reward_once() {
    let mut h = harness(true).expect("canonical boss loot tables");
    for entry in [61408, 61412, 61463, 61528] {
        assert_corpse_reward_commits_once(&mut h, entry, RAGEFIRE_MAP);
    }
}

#[test]
fn encounter_coverage_silverlaine_corpse_commits_authored_rewards_once() {
    let mut h = harness(true).expect("canonical Silverlaine loot tables");
    assert_corpse_reward_commits_once(&mut h, SILVERLAINE, SHADOWFANG_MAP);
}
