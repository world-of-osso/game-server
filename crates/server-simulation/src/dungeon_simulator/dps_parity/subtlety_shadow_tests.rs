//! Native Shadow Dance expiry and per-recipient Lingering Shadow damage.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

fn stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 385960 && aura.caster == run.actor)
        })
        .map_or(0, |aura| aura.stacks)
}

fn cast(run: &mut Run, spell: u32, target: Entity) {
    let mut powers = run
        .app
        .world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
        .unwrap();
    let energy = powers.get_mut(PowerType::Energy).unwrap();
    energy.current = energy.max;
    drop(powers);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, target, Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
}

fn expire_dance(run: &mut Run) {
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 382524),
        "selected native Lingering Shadow talent"
    );
    let actor = run.actor;
    cast(run, 185313, actor);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 185313));
    for _ in 0..600 {
        if !crate::auras::has_aura(run.app.world(), run.actor, 185313) {
            break;
        }
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 185313),
        "ordinary Dance must expire"
    );
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 385960 && aura.caster == run.actor)
        .expect("native Dance expiry must earn Lingering Shadow");
    // SimC76f8118 sc_rogue.cpp11184–11193: tick-zero removes one stack.
    assert_eq!(aura.stacks, aura.max_stacks - 1);
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_shadow_native_dance_expiry_decays_earned_stacks() {
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    expire_dance(&mut run);
    let initial = stacks(&run);
    for _ in 0..21 {
        run.app.update();
    }
    assert!(
        stacks(&run) > 0 && stacks(&run) < initial,
        "native periodic clock must decay the earned stack count"
    );
    for _ in 0..400 {
        run.app.update();
    }
    assert_eq!(
        stacks(&run),
        0,
        "Lingering Shadow must expire, not remain at one stack"
    );
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_shadow_native_storm_copies_each_earned_hit_once() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(261, 3).unwrap();
    engage(&mut run);
    let primary = run.targets[0];
    cast(&mut run, 197835, primary);
    assert!(
        !run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.spell_id == Some(386081)),
        "no Lingering Shadow without earned stacks"
    );
    expire_dance(&mut run);
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    cast(&mut run, 197835, primary);
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        let hits: Vec<_> = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(197835)
                    && event.amount > 0
            })
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "real Shuriken Storm must land on each recipient"
        );
        let copies: Vec<_> = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(386081)
                    && event.amount > 0
            })
            .collect();
        assert_eq!(
            copies.len(),
            1,
            "each eligible landed hit must produce exactly one Lingering Shadow, without recursion"
        );
        assert!(
            !copies[0].crit,
            "source crit is already baked into the residual"
        );
    }
}
