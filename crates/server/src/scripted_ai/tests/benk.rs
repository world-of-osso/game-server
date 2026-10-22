//! LittleWigs2777a9d BenkBuzzbee.lua; source218002, native70448.
use super::*;

const BENK: u32 = 218002;
const MAP: u32 = 2661;
const BIT: u32 = 3;
const SNACK_TIME: u32 = 438025;

#[test]
#[ignore = "native RED: Benk has no registered source clock"]
fn encounter_coverage_benk_first_snack_uses_current_lua_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = bound_boss_at_difficulty(BENK, MAP, BIT, difficulty, None);
        assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
        let mut visibility = crate::interest::NetworkVisibility::default();
        visibility.gain_visibility(h.players[0].1);
        h.app.world_mut().entity_mut(boss).insert(visibility);
        let mut near = *h.app.world().get::<Position>(boss).unwrap();
        near.x += 3.0;
        let player = h.player();
        h.app.world_mut().entity_mut(player).insert(near);
        h.engage(boss);
        h.advance(2.8);
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
            .expect("source Snack Time3s ordinary native cast");
        assert_eq!(cast.spell_id, SNACK_TIME, "difficulty{difficulty}");
        assert_eq!(cast.duration, 2.0, "actual70448 native cast time");
    }
}
