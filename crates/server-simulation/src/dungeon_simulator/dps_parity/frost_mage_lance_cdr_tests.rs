use super::*;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use server::simc_apl::Facts;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_paid_lance_reduces_ray_and_earns_force_of_will_splinters() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        205021,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Consumed,
        |aura| matches!(aura.spell_id, 44544 | 1247730),
    );
    for _ in 0..4 {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.targets[0],
                spell_id: 1221389,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    let bindings = Bindings {
        actions: [("ray".into(), 205021)].into(),
        ..default()
    };
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let cooldown = |run: &Run| {
        NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1)
            .value("cooldown.ray.full_recharge_time")
            .unwrap()
    };
    let before = cooldown(&run);
    assert!(
        before > 0.0,
        "the real Ray cooldown must run before testing its reducer"
    );
    let start = crate::spell_cast::now(run.app.world());
    let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(30455, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let native_cap =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1246769, 3);
    assert_eq!(
        native_cap, 6.0,
        "Heart of Ice and Polished Focus each add one to native four-stack Lance"
    );
    let expected = f64::from(
        run.app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(1247798)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == 1)
            .unwrap()
            .base_points,
    ) * f64::from(native_cap)
        / 1000.0;
    let elapsed = crate::spell_cast::now(run.app.world()) - start;
    let reduction = before - cooldown(&run) - elapsed;
    assert!(
        (reduction - expected).abs() < 0.005,
        "the selected native Shatter cap must pay native Glaciate; expected{expected}, got{reduction}"
    );
    let splinters = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .skip(cursor)
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(443722)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        splinters >= 3,
        "six primary shattered stacks must earn native Force of Will3Splinters, got{splinters}"
    );
}
