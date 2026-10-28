//! Native equipped Tendril procs from real Frost combat.
use super::*;
use crate::auras::ActiveAuras;

#[test]
#[ignore = "requires native Frost64 profile and item tables"]
fn midnight_tendril_native_damage_earns_stacking_critical_rating() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(64, 3).unwrap();
    let driver = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1310404 && aura.caster == run.actor)
        .expect("native equipped Tendril driver");
    let amount = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    let item_level = driver.cast_item_level;
    assert!(amount > 0.0);
    assert!(item_level.is_some());
    engage(&mut run);
    let mut highest = 0;
    let mut first_stacks = None;
    advance_combat_observed(&mut run, |run| {
        let auras = run.app.world().get::<ActiveAuras>(run.actor).unwrap();
        if let Some(buff) = auras
            .iter()
            .find(|aura| aura.spell_id == 1310414 && aura.caster == run.actor)
        {
            first_stacks.get_or_insert(buff.stacks);
            highest = highest.max(buff.stacks);
            let actual = buff
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .unwrap()
                .amount;
            assert!(
                (actual - amount).abs() < 0.001,
                "native equipped driver owns stat budget"
            );
            assert_eq!(buff.cast_item_level, item_level);
        }
    })
    .unwrap();
    assert!(
        first_stacks.is_some_and(|stacks| stacks >= 3),
        "first real damage must grant three new-target stacks: {first_stacks:?}"
    );
    assert!(
        highest > first_stacks.unwrap(),
        "later damage earns additional stacks"
    );
}
