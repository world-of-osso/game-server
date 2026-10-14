//! Native AP166/167 and ChrClasses ranged AP, TC a352b1fa StatSystem.cpp354–414.
use super::test_harness::*;
use crate::stats::CharacterStats;
use shared::components::Level;

const BATTLE_SHOUT: u32 = 6673;
const AIMED_SHOT: u32 = 19434;

fn actor(class: u8) -> Harness {
    let mut h = harness_with(
        Vec::new(),
        &[BATTLE_SHOUT, AIMED_SHOT, 1271909],
        class,
        true,
    )
    .unwrap();
    h.advance(5);
    h.inbox = Inbox::default();
    h
}

#[test]
fn class_completeness_native_battle_shout_increases_computed_melee_attack_power() {
    let mut h = actor(1);
    let before = h
        .app
        .world()
        .get::<CharacterStats>(h.caster)
        .unwrap()
        .attack_power;
    assert!(before > 0.0);
    h.cast_and_deliver(BATTLE_SHOUT, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(3);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        BATTLE_SHOUT
    ));
    let after = h
        .app
        .world()
        .get::<CharacterStats>(h.caster)
        .unwrap()
        .attack_power;
    assert!(
        (after - before * 1.05).abs() < 0.001,
        "native166: before={before} after={after}"
    );
}

#[test]
fn class_completeness_native_aimed_shot_uses_source_ranged_ap_not_melee_ap() {
    let mut h = actor(3);
    let conn = rusqlite::Connection::open_with_flags(
        world_db().unwrap().powers_db.clone(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let coefficient: f32 = conn
        .query_row(
            "SELECT RangedAttackPowerPerAgility FROM chr_classes WHERE ID=3",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let stats = h.app.world().get::<CharacterStats>(h.caster).unwrap();
    let level = h.app.world().get::<Level>(h.caster).unwrap().0;
    let ranged_ap = (f32::from(level) + stats.primary.agility.max(0.0)) * coefficient;
    assert!(ranged_ap > stats.attack_power);
    let ap_coefficient =
        world_db().unwrap().store.get(AIMED_SHOT).unwrap().effects[0].bonus_coefficient_from_ap;
    let expected = varied(ranged_ap * ap_coefficient);
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 0.0;
    let before = h.health(h.npc);
    h.cast_and_deliver(AIMED_SHOT, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(70);
    let damage = before - h.health(h.npc);
    assert!(
        expected.contains(&damage),
        "native ranged AP {ranged_ap}: expected {expected:?}, damage={damage}"
    );
}
