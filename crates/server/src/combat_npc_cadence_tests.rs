//! Ordinary, source-spawned NPC white swings use creature_template.BaseAttackTime.
//! TC a352b1fa Creature.cpp640–642 and Unit.cpp670–672.

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::components::{Health, Npc, Position};

use super::{CombatLog, CombatTarget};
use crate::spell_cast::test_harness::{Harness, harness};
use crate::world_data::WorldData;
use crate::zones::{CreatureSpawnContext, TempSummon};

fn source_cadence_ms(entry: u32) -> u32 {
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    conn.query_row(
        "SELECT BaseAttackTime FROM content_creature_template WHERE entry=?1",
        [entry],
        |row| row.get(0),
    )
    .unwrap()
}

fn spawn_ordinary(h: &mut Harness, entry: u32) -> Entity {
    let data = crate::test_data::world_data();
    let spawn = data
        .spawns
        .iter()
        .find(|spawn| spawn.template_entry == entry && spawn.map == 0)
        .expect("real Eastern Kingdoms source spawn");
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    let mut context = CreatureSpawnContext {
        map: crate::maps::InMap::continent(0),
        difficulty_id: 0,
        ground: None,
        temporary: false,
    };
    let npc = crate::zones::spawn_creature(
        &mut commands,
        data,
        &data.templates[&entry],
        &mut context,
        false,
        spawn,
        &mut rand::thread_rng(),
    )
    .expect("authored tuned creature can spawn without guessed terrain context");
    queue.apply(h.app.world_mut());
    assert_eq!(h.app.world().get::<Npc>(npc).unwrap().template_id, entry);
    assert!(h.app.world().get::<TempSummon>(npc).is_none());
    npc
}

fn cadence_fixture(entry: u32) -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[], 1).unwrap();
    h.app
        .insert_resource::<WorldData>(crate::test_data::world_data().clone());
    let npc = spawn_ordinary(&mut h, entry);
    let position = *h.app.world().get::<Position>(npc).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert((
        Position {
            x: position.x + 1.0,
            ..position
        },
        Health {
            current: 1_000_000.0,
            max: 1_000_000.0,
        },
    ));
    h.app
        .world_mut()
        .entity_mut(npc)
        .insert(CombatTarget(Some(h.caster)));
    (h, npc)
}

fn observed_white_swing_times(h: &Harness, npc: Entity) -> Vec<f64> {
    h.app
        .world()
        .resource::<CombatLog>()
        .range(0.0, f64::MAX)
        .into_iter()
        .filter(|entry| {
            entry.event.attacker == npc.to_bits()
                && entry.event.target == h.caster.to_bits()
                && entry.event.spell_id == 0
        })
        .map(|entry| entry.timestamp)
        .collect()
}

fn assert_source_cadence(entry: u32, authored_ms: u32) {
    assert_eq!(source_cadence_ms(entry), authored_ms);
    let (mut h, npc) = cadence_fixture(entry);
    // Ordinary50ms fixture updates, no timer writes or forced extra attacks.
    h.advance(200);
    let times = observed_white_swing_times(&h, npc);
    assert!(times.len() >= 3, "entry{entry}: only {times:?}");
    let expected = f64::from(authored_ms) / 1000.0;
    for pair in times.windows(2) {
        let interval = pair[1] - pair[0];
        assert!(
            interval >= expected - 0.001 && interval <= expected + 0.051,
            "entry{entry} authored{authored_ms}ms, observed{interval:.6}s: {times:?}"
        );
    }
    assert!(h.app.world().get::<Health>(h.caster).unwrap().current > 0.0);
}

#[test]
#[ignore = "RED: source Prowler118 has1500ms but ordinary NPC swings at hardcoded2s"]
fn ordinary_npc_authored_cadence_prowler_1500ms() {
    assert_source_cadence(118, 1500);
}

#[test]
#[ignore = "RED: source Murloc Warrior171 has3000ms but ordinary NPC swings at hardcoded2s"]
fn ordinary_npc_authored_cadence_murloc_warrior_3000ms() {
    assert_source_cadence(171, 3000);
}
