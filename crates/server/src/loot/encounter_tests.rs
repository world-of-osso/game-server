//! Real boss loot through death generation, client requests and inventory commits.
//! Corpse staging isolates the loot boundary; this is not complete-fight proof.
use super::*;

const RAGEFIRE_MAP: u32 = 389;
const SHADOWFANG_MAP: u32 = 33;
const SILVERLAINE: u32 = 3887;
const SILVERLAINE_BOSS_ITEMS: [u32; 2] = [6321, 6323];
const SILVERLAINE_POOL: [u32; 6] = [1205, 2589, 2592, 4606, 6321, 6323];
const WAILING_MAP: u32 = 43;
const WAILING_PRIMARIES: [u32; 8] = [3671, 3669, 3653, 3670, 3674, 3673, 5775, 3654];

/// Current authored pools, including optional food/cloth/quest and animal drops.
fn assert_wailing_rewards(entry: u32, copper: u64, rewards: &[(u32, u32)]) {
    let (coins, pool, boss_items): (_, &[u32], &[u32]) = match entry {
        3671 => (
            56..=180,
            &[1179, 2589, 2592, 4537, 5404, 6446, 9739, 10412],
            &[5404, 6446, 10412],
        ),
        3669 => (
            87..=168,
            &[1179, 2589, 2592, 4537, 6460, 6465, 9738, 10410],
            &[6460, 6465, 10410],
        ),
        3653 => (
            0..=0,
            &[4555, 4556, 5523, 6361, 6443, 6447, 6470, 6471, 13245],
            &[6447, 13245],
        ),
        3670 => (
            135..=181,
            &[1179, 2589, 2592, 4537, 6472, 6473, 9740],
            &[6472, 6473],
        ),
        3674 => (
            0..=0,
            &[5125, 5128, 5470, 6443, 6448, 6449, 6470, 6471],
            &[6448, 6449],
        ),
        3673 => (
            156..=255,
            &[1179, 2589, 2592, 4537, 5970, 6459, 6469, 9741, 10411],
            &[5970, 6459, 6469, 10411],
        ),
        5775 => (
            211..=281,
            &[
                765, 785, 1179, 2447, 2449, 2450, 2452, 2453, 4605, 6443, 6470, 6471, 6629, 6630,
                6631,
            ],
            &[6629, 6630, 6631],
        ),
        3654 => (
            226..=300,
            &[
                730, 1179, 1468, 5524, 5784, 6289, 6443, 6461, 6463, 6470, 6471, 10441, 17057,
            ],
            &[6461, 6463],
        ),
        _ => panic!("not a Wailing primary: {entry}"),
    };
    // Project contract auto-sells poor-quality drops (docs/specs/loot.md).
    // Exact current70391 SellPrice sums; each authored poor drop has count1.
    let (sale_bonuses, grey_items): (&[u64], &[u32]) = match entry {
        3653 => (&[0, 155, 903, 1058], &[4555, 4556]),
        3674 => (&[0, 155, 202, 357], &[5125, 5128]),
        3654 => (&[0, 7, 16, 23], &[730, 17057]),
        _ => (&[0], &[]),
    };
    let authored_coins_with_sales = sale_bonuses.iter().any(|bonus| {
        copper
            .checked_sub(*bonus)
            .is_some_and(|base_coins| coins.contains(&base_coins))
    });
    assert!(
        authored_coins_with_sales,
        "{entry} copper{copper} outside authored coins plus grey sales"
    );
    assert!(
        rewards.iter().all(|(item, _)| !grey_items.contains(item)),
        "poor drops converted to coins, not inventory items"
    );
    assert!(
        rewards.iter().all(|(item, _)| pool.contains(item)),
        "{entry} authored item pool"
    );
    let rare_rewards: Vec<_> = rewards
        .iter()
        .filter(|(item, _)| boss_items.contains(item))
        .collect();
    if entry == 3654 {
        // Source group1 has only two33.3333% rows, not a guaranteed boss item.
        assert!(rare_rewards.len() <= 1);
    } else {
        assert_eq!(rare_rewards.len(), 1, "{entry} authored100% group1 reward");
    }
    assert!(rare_rewards.iter().all(|(_, count)| *count == 1));
}

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
    if WAILING_PRIMARIES.contains(&entry) {
        assert_wailing_rewards(entry, copper, rewards);
    } else if entry == SILVERLAINE {
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
fn encounter_coverage_wailing_corpse_commits_all_eight_authored_rewards_once() {
    for entry in WAILING_PRIMARIES {
        let mut h = harness(true).expect("canonical Wailing loot tables");
        assert_corpse_reward_commits_once(&mut h, entry, WAILING_MAP);
    }
}

#[test]
fn encounter_coverage_silverlaine_corpse_commits_authored_rewards_once() {
    let mut h = harness(true).expect("canonical Silverlaine loot tables");
    assert_corpse_reward_commits_once(&mut h, SILVERLAINE, SHADOWFANG_MAP);
}
