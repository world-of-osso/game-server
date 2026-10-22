//! FOLLOWER_EXPECTED_STAT_PLAYER_BUDGET is project policy, not Retail NPC gear.
use bevy::prelude::*;
use shared::components::{CombatRatings, DerivedStats, Level, UnitStats};

use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use crate::spell_cast::test_harness::{harness, world_db};
use crate::stats::{CharacterStats, PrimaryStat, StatsPlugin};

fn main_stat(stats: &CharacterStats, primary: PrimaryStat) -> f32 {
    match primary {
        PrimaryStat::Strength => stats.primary.strength,
        PrimaryStat::Agility => stats.primary.agility,
        PrimaryStat::Intellect => stats.primary.intellect,
    }
}

#[test]
fn follower_power_budget_all_five_initial_sheets_use_level_driven_primary_only() {
    let mut h = harness(vec![], &[], 8).unwrap();
    h.app.insert_resource(world_db().unwrap().classes.clone());
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = super::catalog::load_catalog(&db).unwrap();
    for (entry, primary) in [
        (209057, PrimaryStat::Strength),
        (209072, PrimaryStat::Intellect),
        (209053, PrimaryStat::Intellect),
        (209059, PrimaryStat::Intellect),
        (209065, PrimaryStat::Agility),
    ] {
        let profile = catalog.profiles.iter().find(|p| p.entry == entry).unwrap();
        // DB2 70448 ExpectedStat rows35/70, default ExpansionID -2.
        for (level, expected) in [(35, 180.685135_f32), (70, 375.046387_f32)] {
            let (stats, _, _) =
                super::spawning::class_stats(h.app.world(), profile, level).unwrap();
            assert_eq!(main_stat(&stats, primary), expected, "{entry} level{level}");
            assert_eq!(stats.ratings, CombatRatings::default());
            let base = crate::stats::base_primary_stats(profile.class_id, 0, level).unwrap();
            assert_eq!(stats.primary.stamina, base.stamina);
            assert_eq!(
                stats.base_mana,
                crate::stats::base_mana(profile.class_id, level).unwrap_or(0.0)
            );
            if level == 70 {
                assert_eq!(stats.max_health, 27534.0, "no HP budget change");
            }
        }
    }
}

#[test]
fn follower_power_budget_survives_passive_refresh_and_recomputes_on_level_change() {
    let (mut h, unit, profile) = spawn_profile_fixture_with_plugins(209059, |app| {
        app.add_plugins(StatsPlugin);
    });
    h.app.world_mut().get_mut::<Level>(h.caster).unwrap().0 = 70;
    h.app.world_mut().entity_mut(h.caster).insert((
        UnitStats::default(),
        CombatRatings::default(),
        DerivedStats::default(),
    ));
    super::spawning::initialize(h.app.world_mut(), h.caster, unit, profile).unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<CharacterStats>(unit)
            .unwrap()
            .primary
            .intellect,
        375.046387_f32
    );
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<CharacterStats>(unit)
            .unwrap()
            .primary
            .intellect,
        375.046387_f32,
        "ordinary passive refresh must not revert to naked class Intellect"
    );
    assert_eq!(
        h.app
            .world()
            .get::<CharacterStats>(h.caster)
            .unwrap()
            .primary
            .intellect,
        145.0,
        "the owner's naked player sheet must not receive the follower policy"
    );
    h.app.world_mut().get_mut::<Level>(unit).unwrap().0 = 35;
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<CharacterStats>(unit)
            .unwrap()
            .primary
            .intellect,
        180.685135_f32,
        "level changes must read the corresponding ExpectedStat budget"
    );
}
