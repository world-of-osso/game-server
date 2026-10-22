//! LittleWigs2777a9d BrewMasterAldryr.lua; source210271, native70448.
//! Difficulty205 projects the authored actor; source spawn excludes205.
use super::*;

const ALDRYR: u32 = 210271;
const MAP: u32 = 2661;
const BIT: u32 = 0;
const KEG_SMASH: u32 = 432229;

#[test]
#[ignore = "native RED: Aldryr has no registered source clock"]
fn encounter_coverage_aldryr_first_keg_smash_uses_current_lua_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = bound_boss_at_difficulty(ALDRYR, MAP, BIT, difficulty, None);
        assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
        let mut visibility = crate::interest::NetworkVisibility::default();
        visibility.gain_visibility(h.players[0].1);
        h.app.world_mut().entity_mut(boss).insert(visibility);
        let mut near = *h.app.world().get::<Position>(boss).unwrap();
        near.x += 3.0;
        let player = h.player();
        h.app.world_mut().entity_mut(player).insert(near);
        h.engage(boss);
        h.advance(4.9);
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
            .expect("source Keg Smash5.1s ordinary native cast");
        assert_eq!(cast.spell_id, KEG_SMASH, "difficulty{difficulty}");
        assert_eq!(cast.duration, 1.5, "actual70448 native cast time");
    }
}
