use super::follower_dps_diagnostic::*;
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::powers::PowerPools;
use shared::components::{Health, PowerType};

fn recharge_deadline(app: &App, actor: Entity) -> f64 {
    let history = app.world().get::<crate::spell_cast::SpellHistory>(actor).unwrap();
    let (used, deadline) = crate::spell_cast::test_harness::charge_recovery(history, 2270);
    assert_eq!(used, 1, "Bladestorm consumes its charge");
    deadline.unwrap()
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Arms follower profile"]
fn follower_arms_sudden_death_execute_reduces_bladestorm() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 71, 90);
    for _ in 0..5 {
        app.update();
    }
    assert!(crate::auras::has_aura(app.world(), actor, 444780));
    let spin =
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 227847).unwrap();
    crate::spell_cast::creature_cast(app.world_mut(), actor, spin, Some(actor), false).unwrap();
    for _ in 0..500 {
        app.update();
        if !crate::auras::has_aura(app.world(), actor, spin) {
            break;
        }
    }
    for _ in 0..3 {
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 445579, Some(target), 1);
    }
    // Slayer's native effect64 queues its Executioner child for the real update.
    app.update();
    app.update();
    assert!(
        crate::auras::has_aura(app.world(), actor, 52437),
        "three actual Slayer executions earn SD"
    );
    let executioner = app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 445584 && aura.caster == actor)
        .unwrap()
        .stacks;
    {
        let mut health = app.world_mut().get_mut::<Health>(target).unwrap();
        health.current = health.max * 0.1;
    }
    app.world_mut()
        .get_mut::<PowerPools>(actor)
        .unwrap()
        .get_mut(PowerType::Rage)
        .unwrap()
        .current = 1000.0;
    app.world_mut()
        .get_mut::<crate::spell_cast::SpellHistory>(actor)
        .unwrap()
        .clear_fixture_precombat_gcd();
    let before = recharge_deadline(&app, actor);
    let execute =
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 163201).unwrap();
    crate::spell_cast::creature_cast(app.world_mut(), actor, execute, Some(target), false)
        .unwrap();
    let seconds = crate::spell_cast::scripts::script_effect_value(app.world(), actor, 444780, 0);
    let reduced = before - recharge_deadline(&app, actor);
    assert!((reduced - f64::from(seconds) * f64::from(executioner)).abs() < 0.001);
    // The follower controller may cast as soon as its charge returns.
    let casts_before = app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(spin)
                && event.kind == shared::protocol::CombatLogKind::CastSuccess
        })
        .count();
    let mut recast = false;
    for _ in 0..1300 {
        app.update();
        let casts = app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(spin)
                    && event.kind == shared::protocol::CombatLogKind::CastSuccess
            })
            .count();
        if casts > casts_before {
            recast = true;
            break;
        }
    }
    assert!(recast, "the follower must recast Bladestorm after its reduced recharge");
    assert!(
        crate::spell_cast::history_now(app.world()) < before,
        "the follower must recast before its original recharge deadline"
    );
}
