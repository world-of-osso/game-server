use super::*;

#[test]
fn bound_health_threshold_completes_alive_on_spell_and_lethal_direct_damage() {
    for (threshold, large_health) in [(59.5_f32, false), (0.0, false), (0.0, true)] {
        let (mut h, boss, map) = bound_boss_at_difficulty(213216, 2652, 3, 205, None);
        h.app
            .world_mut()
            .resource_mut::<MapStore>()
            .encounter_health_thresholds
            .insert((2652, 213216), threshold);
        if large_health {
            let mut health = h.app.world_mut().get_mut::<Health>(boss).unwrap();
            health.max = 226_379_184.0;
            health.current = health.max;
        }
        h.engage(boss);
        h.hit_pct(boss, 20.0);
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map)
                & (1 << 3),
            0,
            "damage above the boundary does not complete the encounter"
        );
        let max = h.health(boss).max;
        if threshold == 0.0 {
            let player = h.player();
            if !large_health {
                h.app.world_mut().get_mut::<Health>(boss).unwrap().current = 16.0;
                let world = h.app.world();
                let attacker = crate::level_scaling::UnitScaling::of(world, player);
                let victim = crate::level_scaling::UnitScaling::of(world, boss);
                let nonlethal_damage = 15.5 * victim.health_multiplier(&attacker);
                assert_eq!(
                    crate::level_scaling::health_loss(&attacker, &victim, nonlethal_damage),
                    15.0
                );
                crate::combat::deal_direct_damage(
                    h.app.world_mut(),
                    player,
                    boss,
                    nonlethal_damage,
                );
                assert_eq!(h.health(boss).current, 1.0);
                assert_eq!(
                    h.app
                        .world()
                        .resource::<crate::maps::InstanceManager>()
                        .completed_encounters(map)
                        & (1 << 3),
                    0,
                    "nonlethal damage to one health must not complete a zero-percent encounter"
                );
            }
            crate::combat::deal_direct_damage(h.app.world_mut(), player, boss, max * 2.0);
        } else {
            h.hit_pct(boss, 200.0);
        }
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map)
                & (1 << 3),
            1 << 3,
            "crossing the configured boundary completes the encounter"
        );
        let health = h.health(boss);
        assert!((health.current - (max * threshold / 100.0).max(1.0)).abs() < 0.01);
        assert!(
            h.app
                .world()
                .get::<crate::combat::DeathState>(boss)
                .is_none()
        );
        assert!(
            h.app
                .world()
                .get::<shared::components::ThreatTable>(boss)
                .is_none()
        );
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::UnitFactionTemplate>(boss)
                .unwrap()
                .0,
            35
        );
    }
}
