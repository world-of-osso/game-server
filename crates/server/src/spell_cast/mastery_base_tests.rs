//! TC a352b1fa StatSystem.cpp549–559: mastery is aura318 plus rating, once.
use super::test_harness::*;
use crate::class_progression::{ActiveSpec, KnownSpells};
use crate::stats::CharacterStats;
use shared::components::Level;

const MASTERY: u32 = 114585;

#[test]
fn native_mastery_base_at_zero_rating_comes_only_from_the_known_aura() {
    // Real native passive and StatsPlugin; no injected CharacterStats or aura amount.
    for (class, spec) in [(1, 71), (10, 269), (8, 62)] {
        let mut h = harness_with(Vec::new(), &[MASTERY], class, true).unwrap();
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert((Level(90), ActiveSpec(spec)));
        h.advance(6);
        let data = world_db().unwrap();
        let base = data.store.get(MASTERY).unwrap().effects[0].base_points;
        assert_eq!(base, 8.0, "native70448 mastery318 fixture");
        assert!(crate::auras::has_aura(h.app.world(), h.caster, MASTERY));
        let stats = h.app.world().get::<CharacterStats>(h.caster).unwrap();
        assert_eq!(stats.ratings.mastery, 0.0, "unequipped native actor");
        assert_eq!(stats.mastery_points, base, "class{class}/spec{spec}");

        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(KnownSpells::fixture(Vec::new()));
        h.advance(6);
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, MASTERY));
        assert_eq!(
            h.app
                .world()
                .get::<CharacterStats>(h.caster)
                .unwrap()
                .mastery_points,
            0.0,
            "unlearning mastery must not leave an implicit base"
        );
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(KnownSpells::fixture(vec![MASTERY]));
        h.advance(6);
        assert_eq!(
            h.app
                .world()
                .get::<CharacterStats>(h.caster)
                .unwrap()
                .mastery_points,
            base,
            "relearning restores exactly one base"
        );
    }
}
