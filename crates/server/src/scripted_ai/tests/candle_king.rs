//! TC a352b1fa boss_the_candle_king.cpp; native70448 source actor208745.
use super::*;

const CANDLE_KING: u32 = 208745;
const MAP: u32 = 2651;
const BIT: u32 = 0;
const MOLDS: u32 = 420659;

fn candle_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(CANDLE_KING, MAP, BIT, difficulty, None);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    let mut near = *h.app.world().get::<Position>(boss).unwrap();
    near.x += 3.0;
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(near);
    (h, boss, map)
}

#[test]
#[ignore = "RED: TC Candle King handler not ported"]
fn encounter_coverage_candle_king_first_molds_uses_tc_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = candle_copy(difficulty);
        h.engage(boss);
        h.advance(5.8);
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(boss)
                .is_none()
        );
        h.advance(0.3);
        let cast = h
            .app
            .world()
            .get::<shared::casting::CastState>(boss)
            .expect("TC first Molds6s must enter ordinary native cast");
        assert_eq!(cast.spell_id, MOLDS, "difficulty{difficulty}");
        assert_eq!(cast.duration, 1.5, "native70448 CastingTimeIndex243");
    }
}
