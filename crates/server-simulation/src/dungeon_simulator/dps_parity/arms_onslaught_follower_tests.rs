use super::follower_dps_diagnostic::*;
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::powers::PowerPools;
use shared::components::{Health, PowerType};

fn cooldown_ms(app: &App, actor: Entity, action: u32) -> u64 {
    let world = app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(action)
        .unwrap();
    world
        .get::<crate::spell_cast::SpellHistory>(actor)
        .unwrap()
        .remaining_cooldown_ms(spell, crate::spell_cast::history_now(world))
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
    crate::spell_cast::SpellHistory::clear_fixture_precombat_gcd(app.world_mut(), actor);
    let before = cooldown_ms(&app, actor, spin);
    assert!(before > 0, "the follower's actual Bladestorm started its cooldown");
    let execute =
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 163201).unwrap();
    crate::spell_cast::creature_cast(app.world_mut(), actor, execute, Some(target), false)
        .unwrap();
    let seconds = crate::spell_cast::scripts::script_effect_value(app.world(), actor, 444780, 0);
    assert_eq!(
        before - cooldown_ms(&app, actor, spin),
        (seconds * f32::from(executioner) * 1000.0) as u64,
        "the follower's SD Execute reduces its live Bladestorm per Executioner stack"
    );
}
