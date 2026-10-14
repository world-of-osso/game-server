//! Real boss loot through death generation, client requests and inventory commits.
//! Corpse staging isolates the loot boundary; this is not complete-fight proof.
use super::*;

fn stage_ragefire_corpse(h: &mut Harness, entry: u32) -> Entity {
    let map = crate::maps::InMap {
        map_id: 389,
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
                name: format!("Ragefire loot fixture {entry}"),
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
            RespawnDelay::for_spawn(7200, 389),
            CorpseDelay::for_rank(1),
        ))
        .id();
    mark_npc_dead_world(h.app.world_mut(), corpse);
    h.settle();
    corpse
}

#[test]
fn ragefire_completion_corpse_loot_commits_each_boss_reward_once() {
    let mut h = harness(true).expect("canonical boss loot tables");
    for entry in [61408, 61412, 61463, 61528] {
        let before = h.inventory();
        let corpse = stage_ragefire_corpse(&mut h, entry);
        let slots = h.app.world().get::<CorpseLoot>(corpse).unwrap().looters
            [&h.looter.character_id]
            .clone();
        let mut copper = 0;
        let mut rewards = Vec::new();
        for content in slots.iter().flatten() {
            match content {
                LootContent::Money { copper: amount } => copper += amount,
                LootContent::Item { item_id, count, .. } => rewards.push((*item_id, *count)),
            }
        }
        assert!(
            (1..=2).contains(&rewards.len()),
            "{entry} has boss item rewards"
        );
        assert!(copper > 0, "{entry} has authored coins");
        let old_counts: Vec<_> = rewards.iter().map(|(item, _)| h.count_of(*item)).collect();
        h.open(corpse, false);
        assert!(h.failures().is_empty());
        for slot in 0..slots.len() {
            h.take_slot(corpse, slot as u8);
        }
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
        for slot in 0..slots.len() {
            h.take_slot(corpse, slot as u8);
        }
        assert_eq!(
            h.inventory(),
            after,
            "retaking {entry} cannot duplicate rewards"
        );
        h.take::<LootFailed>();
    }
}
