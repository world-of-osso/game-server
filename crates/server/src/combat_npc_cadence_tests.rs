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

fn cadence_fixture(entry: u32, spells: &[u32]) -> (Harness, Entity) {
    let mut h = harness(Vec::new(), spells, 1).unwrap();
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
    let (mut h, npc) = cadence_fixture(entry, &[]);
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

fn apply_native_haste(h: &mut Harness, npc: Entity, spell_id: u32, amount: f32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: npc,
            target: npc,
            spell_id,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(h.app.world(), npc, spell_id));
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::StatModifiers>(npc)
            .unwrap()
            .haste_pct,
        amount,
        "native source haste amount must qualify before the cadence assertion"
    );
}

fn assert_modified_cadence(spell_id: u32, amount: f32, factor: f64) {
    assert_eq!(source_cadence_ms(118), 1500);
    let (mut h, npc) = cadence_fixture(118, &[spell_id]);
    apply_native_haste(&mut h, npc, spell_id, amount);
    // Native246 lasts6000ms: measure only its first5000ms, not expired swings.
    h.advance(100);
    assert!(crate::auras::has_aura(h.app.world(), npc, spell_id));
    let times = observed_white_swing_times(&h, npc);
    assert!(times.len() >= 2, "only {times:?}");
    let expected = 1.5 * factor;
    for pair in times.windows(2) {
        let interval = pair[1] - pair[0];
        assert!(
            interval >= expected - 0.001 && interval <= expected + 0.051,
            "source{spell_id}/{amount} expected{expected:.6}s, observed{interval:.6}s: {times:?}"
        );
    }
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

#[test]
#[ignore = "RED: native Bloodlust2825 effect0 must modify Prowler's authored1500ms"]
fn ordinary_npc_authored_cadence_native_positive_haste() {
    // Native2825 effect0: MELEE_SLOW193 +30; TC time ×100/130.
    assert_modified_cadence(2825, 30.0, 100.0 / 130.0);
}

#[test]
#[ignore = "RED: native246 effect0 slows authored1500ms by35%, not inverse haste"]
fn ordinary_npc_authored_cadence_native_negative_haste() {
    // Native246 effect0: MOD_MELEE_HASTE138 -35; TC time ×135/100.
    assert_modified_cadence(246, -35.0, 135.0 / 100.0);
}

#[test]
#[ignore = "RED: TC Unit::ApplyAttackTimePercentMod preserves remaining swing progress"]
fn ordinary_npc_authored_cadence_native_haste_change_preserves_progress() {
    assert_eq!(source_cadence_ms(118), 1500);
    let (mut h, npc) = cadence_fixture(118, &[246]);
    h.advance(1);
    let first = observed_white_swing_times(&h, npc);
    assert_eq!(first.len(), 1, "first real white swing: {first:?}");
    h.advance(10);
    let changed_at = h.app.world().resource::<Time>().elapsed_secs_f64();
    let elapsed = changed_at - first[0];
    assert!((elapsed - 0.5).abs() < 0.001, "elapsed{elapsed}");
    apply_native_haste(&mut h, npc, 246, -35.0);
    let expected_next = changed_at + (1.5 - elapsed) * 1.35;
    h.advance(80);
    let times = observed_white_swing_times(&h, npc);
    assert!(times.len() >= 2, "after native haste change: {times:?}");
    assert!(
        times[1] >= expected_next - 0.001 && times[1] <= expected_next + 0.051,
        "source246 at{changed_at:.6}, expected next{expected_next:.6}, observed{times:?}"
    );
}
