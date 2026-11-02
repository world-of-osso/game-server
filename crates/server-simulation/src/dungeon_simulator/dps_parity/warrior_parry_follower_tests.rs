//! Protection Warrior's actual follower stats respond to Strength and critical rating.
use super::*;
use crate::stats::CharacterStats;

fn apply_aura(app: &mut App, actor: Entity, spell: u32) {
    crate::auras::apply_aura(
        app.world_mut(),
        &crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: spell,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(app.world(), actor, spell));
    for _ in 0..5 {
        app.update();
    }
}

fn simc_parry(stats: &CharacterStats, base_strength: f32, denominator: f32, rating: f32) -> f32 {
    // SimC 76f8118 player.cpp:1644–1683,5275–5298, sc_extra_data.inc:1798–1820.
    let bonus =
        (stats.primary.strength - base_strength) / denominator + stats.ratings.parry / rating;
    3.0 + base_strength / denominator + bonus / (bonus * 0.01 + 1.0 / 0.94)
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn warrior_parry_follower_tracks_strength_and_crit_at_both_levels() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    for (level, map, base_strength, denominator, rating) in [
        (70, 2515, 147.0, 43.48131646, 12.52136772),
        (90, 2813, 622.0, 191.29742347, 52.0),
    ] {
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, map).unwrap();
        let (mut app, actor, _, _) =
            follower_dps_diagnostic::prepare_class_follower(app, copy, 73, level);
        for _ in 0..5 {
            app.update();
        }
        let stats = |app: &App| *app.world().get::<CharacterStats>(actor).unwrap();
        let initial = stats(&app);
        assert!(crate::auras::has_aura(app.world(), actor, 384067));
        assert!(
            (initial.defense.parry - simc_parry(&initial, base_strength, denominator, rating))
                .abs()
                < 0.001
        );

        apply_aura(&mut app, actor, 16612); // Agamaggan's Strength: native +10% Strength.
        let strength = stats(&app);
        assert!(strength.primary.strength > initial.primary.strength);
        assert!(strength.defense.parry > initial.defense.parry);
        assert!(
            (strength.defense.parry - simc_parry(&strength, base_strength, denominator, rating))
                .abs()
                < 0.001
        );

        apply_aura(&mut app, actor, 28755); // Rapid Fire: native +30 critical rating.
        let critical = stats(&app);
        assert_eq!(critical.ratings.crit, strength.ratings.crit + 30.0);
        assert_eq!(critical.ratings.parry, strength.ratings.parry + 30.0);
        assert!(critical.defense.parry > strength.defense.parry);
        assert!(
            (critical.defense.parry - simc_parry(&critical, base_strength, denominator, rating))
                .abs()
                < 0.001
        );
    }
}
