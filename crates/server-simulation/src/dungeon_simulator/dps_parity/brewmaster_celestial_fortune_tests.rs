//! Celestial Fortune follows effective healing, including external dummy healing.
use super::*;
use shared::{components::Health, protocol::CombatLogKind};

fn fortune_heals(run: &Run) -> Vec<i32> {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(216521))
        .map(|event| event.amount)
        .collect()
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn celestial_fortune_uses_effective_nonleech_healing_and_external_heals() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let versatility = {
        let mut stats = run
            .app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(actor)
            .unwrap();
        stats.crit_pct = 100.0;
        stats.versatility_pct
    };
    {
        let mut health = run.app.world_mut().get_mut::<Health>(actor).unwrap();
        health.current = health.max * 0.2;
    }
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(383701)
        .unwrap()
        .clone();
    let ctx = crate::spell_cast::effects::EffectCtx {
        caster: actor,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::apply_fixed_heal(run.app.world_mut(), &ctx, actor, 100_000.0);
    let heals = fortune_heals(&run);
    assert_eq!(
        heals.len(),
        1,
        "one actual heal produces one nonrecursive fortune heal"
    );
    let expected = 100_000.0 * 0.7 * (1.0 + versatility / 100.0);
    assert!(
        (heals[0] as f32 - expected).abs() <= 2.0,
        "fortune must use effective heal and native percentage: {heals:?} vs {expected}"
    );

    crate::spell_cast::script_registry::heal_received(
        run.app.world_mut(),
        actor,
        0,
        75_000.0,
        20_000.0,
        false,
    );
    let heals = fortune_heals(&run);
    assert_eq!(heals.len(), 2);
    let expected_external = 20_000.0 * 0.7 * (1.0 + versatility / 100.0);
    assert!(
        (heals[1] as f32 - expected_external).abs() <= 2.0,
        "external overheal is excluded"
    );
    crate::spell_cast::script_registry::heal_received(
        run.app.world_mut(),
        actor,
        143924,
        10_000.0,
        10_000.0,
        false,
    );
    crate::spell_cast::script_registry::heal_received(
        run.app.world_mut(),
        actor,
        1,
        10_000.0,
        10_000.0,
        true,
    );
    assert_eq!(
        fortune_heals(&run).len(),
        2,
        "leech and percentage heals cannot proc"
    );
}
