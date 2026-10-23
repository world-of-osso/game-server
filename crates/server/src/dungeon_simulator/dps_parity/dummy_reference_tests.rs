//! TC a352b1fa npc_training_dummy: timed combat exit is not an evade.
use super::{Run, prepare};
use crate::auras::{ActiveAuras, aura_now};
use serde_json::{Value, json};
use shared::components::CombatStatus;

const FIRE: u32 = 63;
const FIREBALL: u32 = 133;
const IGNITE: u32 = 12654;
const BERSERK: u32 = 26662;
const CURSE_OF_WEAKNESS: u32 = 702;
const COUNTERSPELL: u32 = 2139;
const AFTER_DUMMY_COMBAT_TIMEOUT: f64 = 5.25;

fn prepare_live_dot() -> Run {
    let mut run = prepare(FIRE, 1).expect("native Fire BiS and active traits must load");
    // Isolate Ignite from Intensifying Flame's separate direct-hit bridge.
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 416714,
    );
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    crate::spell_cast::cast_for_player(run.app.world_mut(), run.actor, FIREBALL)
        .expect("ordinary native Fireball must start");
    for _ in 0..200 {
        run.app.update();
        if crate::auras::has_aura(run.app.world(), run.targets[0], IGNITE) {
            break;
        }
    }
    assert!(crate::auras::has_aura(
        run.app.world(),
        run.targets[0],
        IGNITE
    ));
    run
}

fn read_dot_state(run: &Run) -> Value {
    let world = run.app.world();
    let target = run.targets[0];
    let dot = world
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == IGNITE);
    json!({"time": aura_now(world), "actor": run.actor.to_bits(), "target": target.to_bits(),
        "target_in_combat": world.get::<CombatStatus>(target).map(|status| status.0),
        "dot": dot.map(|aura| json!({"caster": aura.caster.to_bits(),
            "duration_ms": aura.duration_ms, "expires_at": aura.expires_at,
            "interrupt_flags": aura.interrupt_flags,
            "effects": aura.effects.iter().map(|effect| json!({"index": effect.index,
                "amount": effect.amount, "period_ms": effect.period_ms})).collect::<Vec<_>>()}))})
}

fn advance_past_combat_timeout(run: &mut Run) {
    let until = aura_now(run.app.world()) + AFTER_DUMMY_COMBAT_TIMEOUT;
    while aura_now(run.app.world()) < until {
        run.app.update();
    }
}

#[test]
#[ignore = "opt-in real31146 and native Fireball-to-Ignite; requires private parity inputs"]
fn native_dummy_tc_combat_exit_retains_a_live_foreign_dot_past_five_seconds() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_live_dot();
    let before = read_dot_state(&run);
    let expires = before["dot"]["expires_at"].as_f64().unwrap();
    assert!(expires - aura_now(run.app.world()) > AFTER_DUMMY_COMBAT_TIMEOUT);
    advance_past_combat_timeout(&mut run);
    let after = read_dot_state(&run);
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let trace = json!({"before": before, "after": after, "events": events});
    let path = super::inputs::directory()
        .unwrap()
        .join("dummy-tc-dot-retention.json");
    std::fs::write(path, serde_json::to_vec_pretty(&trace).unwrap()).unwrap();
    assert_eq!(
        after["target_in_combat"], false,
        "TC ends the attacker's timed reference"
    );
    assert!(
        !after["dot"].is_null(),
        "TC NullCreatureAI has no evade: unexpired non-LeavingCombat DoT must survive; {trace}"
    );
    let late_start = events.len();
    while aura_now(run.app.world()) <= expires + 0.1 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events[late_start..].iter().any(|event| {
            event.spell_id == Some(IGNITE)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.periodic
                && event.amount > 0
        }),
        "the real native DoT must land its post-five-second tick, not merely retain an icon"
    );
}

#[test]
#[ignore = "opt-in real31146 native LeavingCombat interrupt control; requires private parity inputs"]
fn native_dummy_tc_combat_exit_still_removes_leaving_combat_auras() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_live_dot();
    let target = run.targets[0];
    crate::spell_cast::cast_triggered(run.app.world_mut(), target, BERSERK, Some(target), 1);
    assert!(crate::auras::has_aura(run.app.world(), target, BERSERK));
    advance_past_combat_timeout(&mut run);
    assert!(!run.app.world().get::<CombatStatus>(target).unwrap().0);
    assert!(
        !crate::auras::has_aura(run.app.world(), target, BERSERK),
        "native26662 LeavingCombat interrupt remains authoritative"
    );
}

#[test]
#[ignore = "opt-in ordinary native creature evade control; requires private parity inputs"]
fn native_dummy_tc_fix_keeps_ordinary_creature_evade_aura_removal() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(FIRE, 1).expect("native Fire BiS and active traits must load");
    let creature = crate::zones::summon_creature(
        run.app.world_mut(),
        run.actor,
        40,
        [1.5, 0.0, 0.0, std::f32::consts::PI],
    )
    .expect("native Defias40 must spawn");
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        CURSE_OF_WEAKNESS,
        Some(creature),
        1,
    );
    assert!(crate::auras::has_aura(
        run.app.world(),
        creature,
        CURSE_OF_WEAKNESS
    ));
    crate::creature_aggro::enter_evade_mode(run.app.world_mut(), creature);
    assert!(
        !crate::auras::has_aura(run.app.world(), creature, CURSE_OF_WEAKNESS),
        "ordinary CreatureAI EnterEvadeMode still removes its foreign auras"
    );
}

#[test]
#[ignore = "opt-in real31146 TC non-damage refresh boundary; requires private parity inputs"]
fn native_dummy_tc_nondamaging_spell_does_not_restart_the_damage_timer() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_live_dot();
    let until = aura_now(run.app.world()) + AFTER_DUMMY_COMBAT_TIMEOUT;
    for _ in 0..40 {
        run.app.update();
    }
    let target = run.targets[0];
    assert_eq!(
        run.app
            .world()
            .get::<crate::networking::CombatTarget>(run.actor)
            .unwrap()
            .0,
        Some(target)
    );
    crate::spell_cast::cast_for_player(run.app.world_mut(), run.actor, COUNTERSPELL)
        .expect("ordinary native Counterspell must start without dealing damage");
    while aura_now(run.app.world()) < until {
        run.app.update();
    }
    assert!(
        !run.app.world().get::<CombatStatus>(target).unwrap().0,
        "TC refreshes an existing dummy timer only on non-DOT damage, not a nondamaging spell"
    );
}
