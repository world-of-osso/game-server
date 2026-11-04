//! Death Strike heals from damage the Blood DK's shields absorbed in its window.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

const DEATH_STRIKE: u32 = 49998;
const DEATH_STRIKE_HEAL: u32 = 45470;
const UMBILICUS_SHIELD: u32 = 391527;

#[test]
#[ignore = "requires native Blood profile and world database"]
fn death_strike_heals_from_damage_absorbed_by_shields() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(250, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let enemy = run.targets[0];
    let max = run.app.world().get::<Health>(actor).unwrap().max;
    assert!(crate::spell_cast::cast_triggered_with_overrides(
        run.app.world_mut(),
        actor,
        UMBILICUS_SHIELD,
        Some(actor),
        1,
        crate::spell_cast::TriggeredSpellOverrides {
            effect0_amount: Some(1_000_000_000.0),
            ..Default::default()
        },
    ));
    let health_before = run.app.world().get::<Health>(actor).unwrap().current;
    let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let hit = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(DEATH_STRIKE)
        .unwrap()
        .clone();
    let ctx = crate::spell_cast::effects::EffectCtx {
        caster: enemy,
        spell: &hit,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::deal_damage(
        run.app.world_mut(),
        &ctx,
        actor,
        10.0 * max,
        false,
        false,
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[cursor..];
    let absorbed: i64 = events
        .iter()
        .filter(|e| e.kind == CombatLogKind::Damage && e.target == Some(actor.to_bits()))
        .map(|e| i64::from(e.absorbed))
        .sum();
    assert!(
        absorbed as f32 >= 3.0 * max,
        "the shield absorbs the hit: {absorbed}"
    );
    assert_eq!(
        run.app.world().get::<Health>(actor).unwrap().current,
        health_before,
        "the absorbed hit costs no health"
    );
    let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        PowerType::RunicPower,
        1000.0,
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, DEATH_STRIKE, Some(enemy), false)
        .unwrap();
    for _ in 0..5 {
        run.app.update();
    }
    let heal = run.app.world().resource::<ObservedCombatEvents>().0[cursor..]
        .iter()
        .find(|e| {
            e.source == Some(actor.to_bits())
                && e.spell_id == Some(DEATH_STRIKE_HEAL)
                && e.kind == CombatLogKind::Heal
        })
        .expect("Death Strike heal")
        .amount as f32;
    // DB2 49998: at least 20% of the 5 s damage before talent and healing multipliers.
    // Excluding absorbs would leave only the max-health floor, far below this bound.
    assert!(
        heal >= 0.2 * absorbed as f32,
        "heal {heal} must use absorbed damage {absorbed}; max health {max}"
    );
}
