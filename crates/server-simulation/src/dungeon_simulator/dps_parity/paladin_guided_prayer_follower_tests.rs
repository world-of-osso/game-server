//! Guided Prayer follows incoming damage and its native internal cooldown.
use super::*;
use crate::auras::{AuraApplication, AuraSource};
use server::test_support::healer_intake::{Hit, HitKind};
use shared::components::Health;
use shared::protocol::CombatLogKind;
use std::time::Duration;

#[test]
#[ignore = "requires canonical Protection profile and world database"]
fn guided_prayer_heals_after_damage_once_per_native_cooldown() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(66, 1).unwrap();
    let actor = run.actor;
    let attacker = run.targets[0];
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 404357,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    let options = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(404357)
        .unwrap()
        .aura_options
        .as_ref()
        .unwrap();
    assert_eq!(options.proc_category_recovery_ms, 60_000);
    let max = run.app.world().get::<Health>(actor).unwrap().max;
    let hit = Hit {
        tick: 0,
        source_guid: 1,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: max * 0.01,
        level_scaling_factor: 1.0,
        kind: HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    let heal_count = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(85673)
                    && event.source == Some(actor.to_bits())
                    && event.target == Some(actor.to_bits())
            })
            .count()
    };
    let take_hit = |run: &mut Run, fraction: f32| {
        run.app
            .world_mut()
            .get_mut::<Health>(actor)
            .unwrap()
            .current = max * fraction;
        crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit).unwrap();
        for _ in 0..4 {
            run.app.update();
        }
    };
    take_hit(&mut run, 0.50);
    assert_eq!(
        heal_count(&run),
        0,
        "damage above the threshold must not heal"
    );
    take_hit(&mut run, 0.20);
    assert_eq!(heal_count(&run), 1, "damage below the threshold must heal");
    take_hit(&mut run, 0.20);
    assert_eq!(heal_count(&run), 1, "the ICD must prevent a second heal");
    run.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(250),
        ));
    for _ in 0..244 {
        run.app.update();
    }
    take_hit(&mut run, 0.20);
    assert_eq!(heal_count(&run), 2, "the heal must return after the ICD");
}
