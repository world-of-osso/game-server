//! Follower kit uses a physical max-reach setup before the native Retreat.
use super::*;
use crate::followers::kit::{Conditions, Target};
use shared::components::{ActiveSpec, Position, Rotation};

#[test]
fn havoc_follower_walks_to_melee_edge_before_retreat() {
    let (mut h, unit) = crate::followers::kit_tests::at_level_unengaged(209053, 90, |_| {});
    let enemy = h.npc;
    let mut follower = h.app.world().get::<Follower>(unit).unwrap().clone();
    follower.profile.class_id = 12;
    follower.profile.spec_id = 577;
    h.app.world_mut().entity_mut(unit).insert((
        follower.clone(),
        ActiveSpec(577),
        crate::class_progression::KnownSpells::fixture(vec![198793]),
        Position {
            x: -1.0,
            y: 0.0,
            z: 0.0,
        },
        Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    ));
    let target = *h.app.world().get::<Position>(enemy).unwrap();
    let reach = crate::scripted_ai::model_reach::melee_range(h.app.world(), unit, enemy).unwrap();
    let entry = Entry {
        spec_id: 577,
        spell_id: 198793,
        min_level: 1,
        max_level: 90,
        slot: Slot::Attack,
        conditions: Conditions {
            target: Target::SelfUnit,
            ..Default::default()
        },
    };
    let mut steps = 0;
    for _ in 0..40 {
        let before = *h.app.world().get::<Position>(unit).unwrap();
        let admitted = try_entry(
            h.app.world_mut(),
            unit,
            &follower,
            &[unit],
            Some(enemy),
            &[(enemy, None)],
            &entry,
        );
        if !admitted {
            let world = h.app.world_mut();
            let known = crate::spell_cast::knowledge::resolve_actor_action(world, unit, 198793);
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(198793)
                .unwrap()
                .clone();
            let readiness = crate::spell_cast::read_native_cast_start(world, unit, &spell, unit);
            let conditions = conditions_met(world, unit, unit, &[unit], &[(enemy, None)], &entry);
            panic!(
                "Retreat unavailable: known={known:?}, readiness={readiness:?}, conditions={conditions}"
            );
        }
        if h.app
            .world()
            .get::<crate::knockback::KnockbackFlight>(unit)
            .is_some()
        {
            let distance = (before.x - target.x).hypot(before.z - target.z);
            assert!(
                (distance - reach).abs() < 0.05,
                "Retreat started at {distance}, reach {reach}"
            );
            let rotation = h.app.world().get::<Rotation>(unit).unwrap();
            let away = Vec2::new(before.x - target.x, before.z - target.z).normalize();
            assert!(Vec2::new(rotation.y.sin(), rotation.y.cos()).dot(away) > 0.99);
            assert!(steps > 1, "follower must walk rather than teleport");
            return;
        }
        let after = *h.app.world().get::<Position>(unit).unwrap();
        assert!(
            (after.x - before.x).hypot(after.z - before.z) < 1.0,
            "one frame cannot teleport to max reach"
        );
        steps += 1;
        h.advance(1);
    }
    panic!("follower did not launch its native Retreat");
}
