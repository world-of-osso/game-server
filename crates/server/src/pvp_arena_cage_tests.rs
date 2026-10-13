use super::*;
use crate::spell_cast::test_harness::{Harness, harness};
use rusqlite::OpenFlags;

const CAGE: InMap = InMap {
    map_id: MAP,
    instance_id: 0,
};
/// TDB areatrigger spawns 269/270 ("Entrance Purple"/"Entrance Gold").
const PURPLE_ENTRANCE: Vec3 = Vec3::new(443.727, 314.306, -24.5936);
const GOLD_ENTRANCE: Vec3 = Vec3::new(443.885, 456.386, -24.5629);

fn cage_harness(team: BgTeam) -> Harness {
    let mut h = harness(
        Vec::new(),
        &[STARTING_AREA_MARKER, WARNING, WARNING_TELEPORT],
        1,
    )
    .unwrap();
    let db = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app.insert_resource(Catalog::load(&db).unwrap());
    let battleground = shared::protocol::BattlegroundSnapshot {
        match_id: 1,
        battleground_id: 1117,
        name: "Cage of Carnage".into(),
        map_id: MAP,
        team,
        phase: BgPhase::InProgress,
        accepted: true,
        invitation_time_left_secs: None,
        score_a: 0,
        score_b: 0,
        flags: Default::default(),
        world_states: Vec::new(),
        outcome: None,
        players: Vec::new(),
        honor_reward: None,
        arena: None,
    };
    h.app.world_mut().entity_mut(h.caster).insert((
        CAGE,
        super::super::PlayerPvpState {
            battleground: Some(battleground),
            ..Default::default()
        },
    ));
    h
}

fn place(h: &mut Harness, point: Vec3) {
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::game_objects::bevy_position(point));
}

fn location(h: &Harness) -> Vec3 {
    crate::game_objects::wow_position(h.app.world().get::<Position>(h.caster).unwrap())
}

fn has(h: &Harness, spell_id: u32) -> bool {
    crate::auras::has_aura(h.app.world(), h.caster, spell_id)
}

#[test]
fn arena_cage_entrance_marker_warns_after_start_then_teleports_alliance_to_purple_exit() {
    let mut h = cage_harness(BgTeam::A);
    let mut clock = Clock::default();
    place(&mut h, PURPLE_ENTRANCE);
    update_presence(h.app.world_mut(), CAGE, &mut clock);
    assert!(has(&h, STARTING_AREA_MARKER));
    update_warnings(h.app.world_mut(), CAGE, 4.999, &mut clock);
    assert!(
        !has(&h, WARNING),
        "first source check is 5s after the start"
    );
    update_warnings(h.app.world_mut(), CAGE, 5.0, &mut clock);
    assert!(has(&h, WARNING));
    // Warning lasts DurationIndex28 = 5000ms; its removal while marked teleports.
    h.advance(110);
    assert!(!has(&h, WARNING));
    let at = location(&h);
    assert!(
        at.distance(PURPLE_WARNING_TELEPORT.0) < 0.01,
        "Alliance camper must land at the Purple exit, got {at:?}"
    );
}

#[test]
fn arena_cage_leaving_gold_entrance_drops_marker_so_warning_expiry_does_not_teleport() {
    let mut h = cage_harness(BgTeam::B);
    let mut clock = Clock::default();
    place(&mut h, GOLD_ENTRANCE);
    update_presence(h.app.world_mut(), CAGE, &mut clock);
    update_warnings(h.app.world_mut(), CAGE, 5.0, &mut clock);
    assert!(has(&h, STARTING_AREA_MARKER) && has(&h, WARNING));
    // Gold box: facing 4.743, half-length 35, so 56yd along -Y is outside it.
    let outside = GOLD_ENTRANCE - Vec3::Y * 56.0;
    place(&mut h, outside);
    update_presence(h.app.world_mut(), CAGE, &mut clock);
    assert!(!has(&h, STARTING_AREA_MARKER));
    h.advance(110);
    assert!(!has(&h, WARNING));
    assert!(location(&h).distance(outside) < 0.01);
    update_warnings(h.app.world_mut(), CAGE, 10.0, &mut clock);
    assert!(!has(&h, WARNING), "unmarked players are not warned again");
}

#[test]
fn arena_cage_gold_camper_teleports_to_gold_exit() {
    let mut h = cage_harness(BgTeam::B);
    let mut clock = Clock::default();
    place(&mut h, GOLD_ENTRANCE + Vec3::new(3.0, 20.0, 0.0));
    update_presence(h.app.world_mut(), CAGE, &mut clock);
    update_warnings(h.app.world_mut(), CAGE, 5.0, &mut clock);
    h.advance(110);
    assert!(location(&h).distance(GOLD_WARNING_TELEPORT.0) < 0.01);
}
