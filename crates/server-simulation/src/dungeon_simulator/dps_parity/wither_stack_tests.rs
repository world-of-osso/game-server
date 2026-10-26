//! Shared Hellcaller Wither: counters do not multiply ticks or grow on refresh.
use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const WITHER: u32 = 445474;
const WITHER_CAST: u32 = 445468;

fn apply_native_aura(run: &mut Run, target: Entity, spell_id: u32) {
    let mask = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap()
        .effects
        .iter()
        .filter(|effect| effect.effect == 6)
        .fold(0, |mask, effect| mask | (1 << effect.index));
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: run.actor,
            target,
            spell_id,
            effect_mask: mask,
            source: AuraSource::Cast,
        },
    );
}

fn apply_counter_state(run: &mut Run, stacks: u8) {
    // Native SPELLVALUE_AURA_STACK is the explicit counter input to this
    // boundary, not a claimed producer or an edited damage coefficient.
    let (caster, target) = (run.actor, run.targets[0]);
    let mut spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(WITHER)
        .unwrap()
        .clone();
    spell.cast_aura_stacks = Some(stacks);
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == WITHER && aura.caster == caster,
    );
    assert!(crate::spell_cast::launch_triggered_spell(
        run.app.world_mut(),
        caster,
        &spell,
        Some(target),
        1
    ));
    for _ in 0..10 {
        run.app.update();
        if crate::auras::has_aura(run.app.world(), target, WITHER) {
            break;
        }
    }
    assert_eq!(
        counter(run),
        stacks,
        "native counter input must be retained"
    );
}

fn counter(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == WITHER && aura.caster == run.actor)
        .unwrap()
        .stacks
}

fn first_tick_damage(spec: u32, stacks: u8) -> (f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(spec, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    apply_native_aura(&mut run, actor, 171884); // Native Denounce removes critical RNG.
    run.app.update();
    let target = run.targets[0];
    apply_counter_state(&mut run, stacks);
    for _ in 0..100 {
        run.app.update();
        let event = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(WITHER)
                    && event.periodic
                    && event.kind == CombatLogKind::Damage
            });
        if let Some(event) = event {
            assert!(
                event.amount > 0 && !event.crit,
                "actual ordinary native Wither tick required"
            );
            let spell = run
                .app
                .world()
                .resource::<crate::spell_info::SpellStore>()
                .get(WITHER)
                .unwrap();
            let variance = spell
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .unwrap()
                .variance;
            return (f64::from(event.amount), f64::from(variance));
        }
    }
    panic!("native spec{spec} Wither must deal its first periodic tick");
}

#[test]
#[ignore = "requires private265/267 native profiles and world DB"]
fn wither_counter_stacks_do_not_multiply_native_periodic_damage() {
    // SimC76f8118 sc_warlock_actions.cpp1066: dot_ignore_stack=true.
    for spec in [265, 267] {
        let (one, variance) = first_tick_damage(spec, 1);
        let (ten, _) = first_tick_damage(spec, 10);
        let spread = (1.0 + variance / 2.0) / (1.0 - variance / 2.0);
        assert!(
            ten >= one / spread - 2.0 && ten <= one * spread + 2.0,
            "spec{spec} Wither base damage must ignore the real counter: stack1={one}, stack10={ten}, native variance={variance}"
        );
    }
}

fn cast_wither(run: &mut Run) {
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(
            WITHER_CAST,
            run.targets[0],
            Some(run.targets[0]),
        ),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    let landed = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(WITHER_CAST)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        });
    assert!(
        landed,
        "an actual completed native Wither cast is the prerequisite"
    );
}

#[test]
#[ignore = "requires private265/267 native profiles and world DB"]
fn wither_actual_recasts_preserve_existing_counter_in_both_specs() {
    // SimC76f8118 sc_warlock_actions.cpp1174–1186 cancels the automatic
    // refreshed-dot stack increment, without removing already earned stacks.
    for spec in [265, 267] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(spec, 1).unwrap();
        engage(&mut run);
        cast_wither(&mut run);
        assert_eq!(counter(&run), 1);
        cast_wither(&mut run);
        assert_eq!(
            counter(&run),
            1,
            "spec{spec} reapplication cannot earn a counter stack"
        );
        apply_counter_state(&mut run, 8);
        cast_wither(&mut run);
        assert_eq!(
            counter(&run),
            8,
            "spec{spec} refresh must keep the prior earned counter, not reset or increment it"
        );
    }
}
