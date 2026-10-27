//! Powerful Eversong Diamond scales its crit effects by unique equipped gem colors.
use super::*;
use crate::auras::{ActiveAuras, AuraSource};

#[test]
#[ignore = "requires private267 native player inputs"]
fn eversong_diamond_crit_bonus_counts_four_unique_gem_colors() {
    let _seed = crate::random::seed_scope(70448);
    let run = prepare_literal(267, 1).unwrap();
    let world = run.app.world();
    let per_color = world
        .resource::<crate::spell_info::SpellStore>()
        .get(1258209)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    let driver = world
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1258209 && matches!(aura.source, AuraSource::Equipment(_)))
        .expect("267 head socket holds Powerful Eversong Diamond")
        .clone();
    // 267 sockets Amethyst240900, Garnet240906, Lapis240914 and Peridot240890:
    // SimC76f8118 unique_gear_midnight.cpp54–55,469–477 counts all four colors.
    for (index, aura_type) in [(1, 163), (2, 50)] {
        let effect = driver.effects.iter().find(|e| e.index == index).unwrap();
        assert_eq!(effect.aura_type, aura_type);
        assert!(
            (effect.amount - per_color * 4.0).abs() < 1e-4,
            "effect{index} must be {per_color} per unique color, got {}",
            effect.amount
        );
    }
}
