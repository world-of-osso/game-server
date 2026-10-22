//! LittleWigs2777a9d AraKara/KikatalTheHarvester.lua; source215407, native70448.
use super::*;

const KIKATAL: u32 = 215407;
const MAP: u32 = 2660;
const BIT: u32 = 0;
const WEBS: u32 = 432130;

#[test]
#[ignore = "native RED: Kikatal has no registered source clock"]
fn encounter_coverage_kikatal_first_webs_uses_current_lua_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = bound_boss_at_difficulty(KIKATAL, MAP, BIT, difficulty, None);
        assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
        let mut visibility = crate::interest::NetworkVisibility::default();
        visibility.gain_visibility(h.players[0].1);
        h.app.world_mut().entity_mut(boss).insert(visibility);
        let mut near = *h.app.world().get::<Position>(boss).unwrap();
        near.x += 3.0;
        let player = h.player();
        h.app.world_mut().entity_mut(player).insert(near);
        h.engage(boss);
        h.advance(5.9);
        assert!(
            h.app
                .world()
                .get::<crate::spell_cast::CastState>(boss)
                .is_none()
        );
        h.advance(0.3);
        let cast = h
            .app
            .world()
            .get::<crate::spell_cast::CastState>(boss)
            .expect("source Erupting Webs6.1s ordinary native cast");
        assert_eq!(cast.spell_id, WEBS, "difficulty{difficulty}");
    }
}
