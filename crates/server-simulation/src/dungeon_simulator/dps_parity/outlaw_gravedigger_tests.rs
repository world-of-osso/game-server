use super::*;
use server::simc_apl::Facts;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn fund(run: &mut Run, points: f32, energy: bool) {
    let mut powers = run
        .app
        .world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
        .unwrap();
    let pool = powers.get_mut(PowerType::Energy).unwrap();
    pool.current = if energy { pool.max } else { 0.0 };
    powers.get_mut(PowerType::ComboPoints).unwrap().current = points;
}

fn cast(run: &mut Run, spell: u32) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
}

fn read_bte_remaining(run: &Run) -> f64 {
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("between_the_eyes".into(), 315341)].into(),
        ..default()
    };
    crate::simc_apl_native::NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        crate::simc_apl_native::Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        },
        1,
    )
    .value("cooldown.between_the_eyes.remains")
    .unwrap()
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_gravedigger_one_earns_additional_native_bte_stacks() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    let mut extra = false;
    for _ in 0..8 {
        fund(&mut run, 5.0, true);
        cast(&mut run, 315341);
        run.app
            .world_mut()
            .entity_mut(run.actor)
            .remove::<crate::combat::AutoAttack>();
        extra = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 315341 && aura.caster == run.actor && aura.stacks >= 2);
        if extra {
            break;
        }
        // Let the real cooldown and older asynchronous stack expire; do not
        // clear cooldowns or inject the extra buff to satisfy the assertion.
        let ticks = (read_bte_remaining(&run) * 20.0).ceil() as usize + 1;
        for _ in 0..ticks {
            run.app.update();
        }
    }
    // sc_rogue.cpp3847–3850: selected native Gravedigger1 rolls an
    // additional buff trigger, never another damage hit or a fabricated stack.
    assert!(
        extra,
        "actual native Bte must earn its sourced extra aura stack"
    );
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_native_dispatch_earns_one_delayed_scoundrel_strike() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    fund(&mut run, 4.0, true);
    cast(&mut run, 2098);
    for _ in 0..8 {
        run.app.update();
    }
    assert!(
        !run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1265918)),
        "below-threshold Dispatch must not earn Scoundrel Strike"
    );
    fund(&mut run, 5.0, true);
    cast(&mut run, 2098);
    for _ in 0..8 {
        run.app.update();
    }
    let hits = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1265918)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    // SimC76f8118 sc_rogue.cpp3794/8879–8893: qualifying actual
    // Dispatch pays one native Scoundrel Strike after200ms at the same enemy.
    assert_eq!(
        hits, 1,
        "actual native Dispatch must earn one Scoundrel Strike"
    );
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_earned_gravedigger_pays_native_points_and_resets_between_the_eyes() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    for _ in 0..32 {
        fund(&mut run, 5.0, true);
        cast(&mut run, 2098);
        if crate::auras::has_aura(run.app.world(), run.actor, 1265935) {
            break;
        }
    }
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1265935),
        "actual native finishers must earn Palmed Bullets and their Gravedigger threshold"
    );
    fund(&mut run, 6.0, false);
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    cast(&mut run, 315341);
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let amount =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1279356, 0)
            as i32;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1279356)
                && event.kind == CombatLogKind::Energize
                && event.amount == amount),
        "earned Gravedigger must pay its native combo-point payload after real Bte; events: {events:?}; auras: {:?}",
        run.app.world().get::<crate::auras::ActiveAuras>(run.actor)
    );
    // sc_rogue.cpp3862–3872: landed earned Bte restores its running
    // cooldown, spends native Palmed uses and consumes Gravedigger.
    assert_eq!(read_bte_remaining(&run), 0.0);
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 1265935));
}
