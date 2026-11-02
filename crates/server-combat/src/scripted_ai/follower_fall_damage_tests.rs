//! Follower MoveFall uses the same physical-immunity policy as player falls.
use super::*;
use crate::auras::{AuraApplication, AuraSource};
use crate::followers::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use shared::components::Health;
use std::time::Duration;

const BLESSING_OF_PROTECTION: u32 = 1_309_794;

#[test]
fn follower_fall_landing_honors_native_liadrin_protection() {
    for protected in [false, true] {
        let (mut h, follower, _) = spawn_profile_fixture_with_plugins(209057, |_| {});
        let caster = h.caster;
        let world = h.app.world_mut();
        let start = Position {
            x: 1608.33,
            y: 15.03,
            z: -2108.33,
        };
        let floor = -67.17;
        world.entity_mut(follower).insert((
            start,
            Health {
                current: 1000.0,
                max: 1000.0,
            },
        ));
        if protected {
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(BLESSING_OF_PROTECTION)
                .unwrap();
            let effect_mask = crate::auras::aura_effect_mask(spell);
            crate::auras::apply_aura(
                world,
                &AuraApplication {
                    caster,
                    target: follower,
                    spell_id: BLESSING_OF_PROTECTION,
                    effect_mask,
                    source: AuraSource::Cast,
                },
            );
            assert!(crate::auras::is_immune(world, follower, 1));
        }
        let duration_ms = (fall_seconds(start.y - floor) * 1000.0) as u32;
        let started = world.resource::<Time>().elapsed();
        world.entity_mut(follower).insert(FallMovement {
            id: 0,
            from: start,
            floor,
            horizontal_target: None,
            horizontal_duration: 0.0,
            elapsed_ms: 0,
            duration_ms,
            started,
        });
        for _ in 0..100 {
            world
                .resource_mut::<Time>()
                .advance_by(Duration::from_millis(50));
            advance_falls(world);
            if world
                .get::<FallMovement>(follower)
                .is_none_or(|fall| fall.elapsed_ms == fall.duration_ms)
            {
                break;
            }
        }
        let remaining = world.get::<Health>(follower).unwrap().current;
        if protected {
            assert_eq!(remaining, 1000.0, "the real aura must protect the landing");
        } else {
            assert!(
                remaining < 1000.0,
                "the same 82-yard fall must deal damage without the aura"
            );
        }
    }
}
