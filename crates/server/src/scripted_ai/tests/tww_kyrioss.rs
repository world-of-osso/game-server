//! LittleWigs2777a9d5 WarWithin/TheRookery/Kyrioss.lua:35–53.
//! Native70448 spell starts/effects; no invented dummy children or phase energy.
use super::*;

const KYRIOSS: u32 = 209230;
const THUNDER: u32 = 1214325;
const WILD: u32 = 474018;
const TORRENT: u32 = 1214315;

fn fixture(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = encounter_fixture(
        KYRIOSS,
        2648,
        0,
        difficulty,
        None,
        EncounterState::NotStarted,
        false,
    );
    let boss = boss.unwrap();
    let player = h.player();
    let mut position = *h.app.world().get::<Position>(boss).unwrap();
    position.x += 3.0;
    h.app.world_mut().entity_mut(player).insert(position);
    h.engage(boss);
    (h, boss, map)
}

fn assert_cast(h: &Harness, boss: Entity, spell: u32, duration: f32) {
    let cast = h
        .app
        .world()
        .get::<CastState>(boss)
        .expect("sourced schedule starts an ordinary native cast");
    assert_eq!(cast.spell_id, spell);
    assert_eq!(cast.duration, duration, "unaltered native70448 cast time");
}

fn first_clock(difficulty: u32) {
    let (mut h, boss, map) = fixture(difficulty);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 0),
        Some(EncounterState::InProgress)
    );
    h.advance(4.95);
    assert!(h.app.world().get::<CastState>(boss).is_none());
    h.advance(0.2);
    assert_cast(&h, boss, THUNDER, 2.5);
    h.advance(4.1);
    assert_eq!(h.casts(0, boss, THUNDER), 1);
    assert!(h.app.world().get::<CastState>(boss).is_none());
    h.advance(0.2);
    assert_cast(&h, boss, WILD, 3.0);
    h.advance(6.35);
    assert_eq!(h.casts(0, boss, WILD), 1);
    assert!(h.app.world().get::<CastState>(boss).is_none());
    h.advance(0.3);
    assert_cast(&h, boss, TORRENT, 4.0);
    let before = h.health(h.player()).current;
    h.advance(7.5);
    assert_eq!(h.casts(0, boss, TORRENT), 1);
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.source == Some(boss.to_bits())
                && event.spell_id == Some(1214323)
                && event.target == Some(h.player().to_bits())
                && event.amount > 0)
    );
    assert!(
        h.health(h.player()).current < before,
        "native periodic1214315→1214323 damage"
    );
    assert!(
        h.inbox(1).log.is_empty(),
        "no combat projection into another map copy"
    );
    let player = h.player();
    crate::combat::kill_unit(h.app.world_mut(), player, boss);
    h.advance(0.1);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 0),
        Some(EncounterState::Done)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|event| event.encounter_id == 2816)
    );
    assert!(h.inbox(1).ends.is_empty());
}

#[test]
#[ignore = "RED first: Kyrioss has no sourced combat schedule port yet"]
fn encounter_coverage_tww_kyrioss_normal_first_clock_native_torrent_and_credit() {
    first_clock(1);
}

#[test]
#[ignore = "RED first: Kyrioss has no sourced combat schedule port yet"]
fn encounter_coverage_tww_kyrioss_follower205_first_clock_native_torrent_and_credit() {
    first_clock(205);
}
