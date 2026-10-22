//! LittleWigs2777a9d CaptainDailcry.lua; source207946, native70448.
//! Difficulty205 here projects the authored actor; source spawn excludes205.
use super::*;

const DAILCRY: u32 = 207946;
const MAP: u32 = 2649;
const BIT: u32 = 1;
const PIERCE: u32 = 424414;

#[test]
#[ignore = "native RED: Dailcry has no registered source clock"]
fn encounter_coverage_dailcry_first_pierce_uses_current_lua_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = bound_boss_at_difficulty(DAILCRY, MAP, BIT, difficulty, None);
        assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
        let mut visibility = crate::interest::NetworkVisibility::default();
        visibility.gain_visibility(h.players[0].1);
        h.app.world_mut().entity_mut(boss).insert(visibility);
        let mut near = *h.app.world().get::<Position>(boss).unwrap();
        near.x += 3.0;
        let player = h.player();
        h.app.world_mut().entity_mut(player).insert(near);
        h.engage(boss);
        h.advance(5.0);
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
            .expect("source Pierce Armor5.2s ordinary native cast");
        assert_eq!(cast.spell_id, PIERCE, "difficulty{difficulty}");
        assert_eq!(cast.duration, 2.5, "actual70448 native cast time");
    }
}
