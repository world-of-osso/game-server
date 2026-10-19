//! TC281–288/461–465/482–495: only the second actual partner death completes.
use super::herald_call::first_west;
use super::herald_intro::{actors_in_copy, kill_actor, spawn_source_copy};
use super::herald_intro2::{WEST, reach_center, received_casts};
use super::*;

#[derive(Clone, Copy)]
struct Pair {
    map: InMap,
    nazan: Entity,
    vazruden: Entity,
}

fn first_pair(difficulty: u32) -> (Harness, Pair) {
    let (h, map, nazan, vazruden) = first_west(difficulty);
    (
        h,
        Pair {
            map,
            nazan,
            vazruden,
        },
    )
}

fn release_pair(h: &mut Harness, pair: Pair) {
    let health = h.health(pair.nazan);
    // Real self-source nonlethal loss has native scale1; no HP/stat write.
    let target = (health.max * 0.29).round();
    crate::combat::deal_direct_damage(
        h.app.world_mut(),
        pair.nazan,
        pair.nazan,
        health.current - target,
    );
    assert!(h.health(pair.nazan).current > 1.0);
    assert!(h.health(pair.nazan).current < health.max * 0.3);
    for _ in 0..180 {
        h.advance(0.05);
        if h.app
            .world()
            .get::<crate::scripted_ai::hellfire_ramparts::HeraldUnkillable>(pair.nazan)
            .is_none()
        {
            assert_eq!(received_casts(h, pair.nazan, 20538), 1);
            assert_eq!(
                h.app.world().get::<ReactState>(pair.nazan),
                Some(&ReactState::Aggressive)
            );
            return;
        }
    }
    panic!("source physical descent/fall+2s must release before real lethal death");
}

fn assert_credit(h: &Harness, pair: Pair, listener: usize, done: bool) {
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(pair.map),
        if done { 1 << 1 } else { 0 },
        "installed1892/bit1 requires both actual partner deaths"
    );
    assert_eq!(
        get_boss_state(h.app.world(), pair.map, 1),
        Some(if done {
            EncounterState::Done
        } else {
            EncounterState::InProgress
        })
    );
    let success: Vec<_> = h
        .inbox(listener)
        .ends
        .iter()
        .filter(|end| end.success)
        .collect();
    assert_eq!(
        success.len(),
        usize::from(done),
        "exactly one success only after second death"
    );
    if let Some(end) = success.first() {
        assert_eq!(end.encounter_id, 1892);
    }
}

fn death_speech_count(h: &Harness, vazruden: Entity, listener: usize) -> usize {
    let texts = &h.app.world().resource::<CreatureTexts>().groups[&17537][&3];
    h.inbox(listener)
        .chat
        .iter()
        .filter(|line| {
            line.channel == ChatType::MonsterYell(vazruden.to_bits())
                && texts.iter().any(|text| {
                    line.content == text.male_text
                        || (!text.female_text.is_empty() && line.content == text.female_text)
                })
        })
        .count()
}

#[test]
fn encounter_coverage_herald_pair_native_death_call_qualification() {
    let (mut h, pair) = first_pair(1);
    let row = h
        .app
        .world()
        .resource::<MapStore>()
        .dungeon_encounter(1892, 543, 1)
        .unwrap();
    assert_eq!((row.id, row.map_id, row.bit), (1892, 543, 1));
    let spell = h.app.world().resource::<SpellStore>().get(30693).unwrap();
    const ALLOW_CAST_WHILE_DEAD: u32 = 0x0080_0000;
    assert_eq!(
        spell.misc.as_ref().unwrap().attributes[0] & ALLOW_CAST_WHILE_DEAD,
        ALLOW_CAST_WHILE_DEAD
    );
    assert_eq!(spell.effects[0].effect, 77);
    println!(
        "native death30693 {}",
        serde_json::to_string(spell).unwrap()
    );
    kill_actor(&mut h, pair.vazruden);
    h.advance(0.05);
    let before = received_casts(&h, pair.vazruden, 30693);
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        pair.vazruden,
        30693,
        Some(pair.vazruden),
        false,
    )
    .expect("actual authored dead-caster ordinary30693 admission, no flag override");
    h.advance(0.05);
    assert_eq!(received_casts(&h, pair.vazruden, 30693), before + 1);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(pair.vazruden)
            .is_some()
    );
    assert_credit(&h, pair, 0, false);
}

fn assert_both_orders(difficulty: u32, same_frame: bool) {
    for nazan_first in [true, false] {
        let (mut h, pair) = first_pair(difficulty);
        release_pair(&mut h, pair);
        let player = h.player();
        let position = *h.app.world().get::<Position>(pair.vazruden).unwrap();
        h.app.world_mut().entity_mut(player).insert(Position {
            x: position.x + 3.0,
            ..position
        });
        assert_credit(&h, pair, 0, false);
        let (first, second) = if nazan_first {
            (pair.nazan, pair.vazruden)
        } else {
            (pair.vazruden, pair.nazan)
        };
        let time = h.app.world().resource::<Time<Fixed>>().elapsed();
        let call_before = received_casts(&h, pair.vazruden, 30693);
        kill_actor(&mut h, first);
        assert_credit(&h, pair, 0, false);
        let unrelated = actors_in_copy(&mut h, 17259, pair.map)
            .into_iter()
            .find(|actor| {
                h.app
                    .world()
                    .get::<crate::zones::CreatureOrigin>(*actor)
                    .is_some_and(|origin| origin.guid == 20_202_607)
            })
            .expect("actual source Bonechewer Hungerer20202607, not an encounter partner");
        kill_actor(&mut h, unrelated);
        assert_credit(&h, pair, 0, false);
        if !same_frame {
            h.advance(0.05);
            assert_credit(&h, pair, 0, false);
        }
        kill_actor(&mut h, second);
        if same_frame {
            assert_eq!(h.app.world().resource::<Time<Fixed>>().elapsed(), time);
        }
        h.advance(0.05);
        assert_credit(&h, pair, 0, true);
        assert_eq!(
            death_speech_count(&h, pair.vazruden, 0),
            1,
            "sourceTalk3 on actual Vazruden death"
        );
        assert_eq!(
            received_casts(&h, pair.vazruden, 30693),
            call_before + 1,
            "ordinary deathCall30693 once"
        );
        for actor in [first, second] {
            kill_actor(&mut h, actor);
        }
        h.advance(0.05);
        assert_credit(&h, pair, 0, true);
        assert_eq!(death_speech_count(&h, pair.vazruden, 0), 1);
        assert_eq!(received_casts(&h, pair.vazruden, 30693), call_before + 1);
        assert!(
            h.inbox(1).ends.iter().all(|end| !end.success),
            "no other-copy success"
        );
    }
}

fn second_pair(h: &mut Harness, difficulty: u32) -> Pair {
    let map = spawn_source_copy(h, 12001, difficulty);
    let nazan = actors_in_copy(h, 17307, map)[0];
    let sentries = actors_in_copy(h, 17517, map);
    let near = *h.app.world().get::<Position>(sentries[0]).unwrap();
    let (x, y, z) = crate::terrain_height::bevy_to_wow(near.x, near.y, near.z);
    let (player, connection) = spawn_player(&mut h.app, map, [x, y, z, 0.0], 12001);
    h.players.push((player, connection, Inbox::default()));
    reach_center(h, nazan, &sentries);
    h.advance(1.0);
    for _ in 0..100 {
        if h.app.world().get::<Position>(nazan) == Some(&WEST) {
            break;
        }
        h.advance(0.05);
    }
    assert_eq!(h.app.world().get::<Position>(nazan), Some(&WEST));
    let vazruden = actors_in_copy(h, 17537, map);
    assert_eq!(vazruden.len(), 1);
    Pair {
        map,
        nazan,
        vazruden: vazruden[0],
    }
}

fn assert_crossed_copies(difficulty: u32) {
    let (mut h, left) = first_pair(difficulty);
    let right = second_pair(&mut h, difficulty);
    assert_ne!(left.map, right.map);
    release_pair(&mut h, left);
    release_pair(&mut h, right);
    kill_actor(&mut h, left.nazan);
    kill_actor(&mut h, right.vazruden);
    h.advance(0.05);
    assert_credit(&h, left, 0, false);
    assert_credit(&h, right, 2, false);
    kill_actor(&mut h, left.vazruden);
    h.advance(0.05);
    assert_credit(&h, left, 0, true);
    assert_credit(&h, right, 2, false);
    kill_actor(&mut h, right.nazan);
    h.advance(0.05);
    assert_credit(&h, left, 0, true);
    assert_credit(&h, right, 2, true);
    for actor in [left.nazan, left.vazruden, right.nazan, right.vazruden] {
        kill_actor(&mut h, actor);
    }
    h.advance(0.05);
    assert_credit(&h, left, 0, true);
    assert_credit(&h, right, 2, true);
}

#[test]
#[ignore = "HERALD_PAIRED_CREDIT_UNIMPLEMENTED: ignored RED first"]
fn encounter_coverage_herald_pair_orders_normal() {
    assert_both_orders(1, false);
}
#[test]
#[ignore = "HERALD_PAIRED_CREDIT_UNIMPLEMENTED: ignored RED first"]
fn encounter_coverage_herald_pair_orders_heroic() {
    assert_both_orders(2, false);
}
#[test]
#[ignore = "HERALD_PAIRED_CREDIT_UNIMPLEMENTED: ignored RED first"]
fn encounter_coverage_herald_pair_same_frame_normal() {
    assert_both_orders(1, true);
}
#[test]
#[ignore = "HERALD_PAIRED_CREDIT_UNIMPLEMENTED: ignored RED first"]
fn encounter_coverage_herald_pair_same_frame_heroic() {
    assert_both_orders(2, true);
}
#[test]
#[ignore = "HERALD_PAIRED_CREDIT_UNIMPLEMENTED: ignored RED first"]
fn encounter_coverage_herald_pair_crossed_copies_normal() {
    assert_crossed_copies(1);
}
#[test]
#[ignore = "HERALD_PAIRED_CREDIT_UNIMPLEMENTED: ignored RED first"]
fn encounter_coverage_herald_pair_crossed_copies_heroic() {
    assert_crossed_copies(2);
}
