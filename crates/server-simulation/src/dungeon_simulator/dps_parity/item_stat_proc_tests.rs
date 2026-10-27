//! Equipped Arcanoweave Lining earns Arcanoweave Insight through actual combat.
use super::*;
use crate::auras::{ActiveAuras, AuraSource};

fn paid_incinerate(run: &mut Run) {
    let actor = run.actor;
    let target = run.targets[0];
    let actual = literal_rotation::execution::resolve_action(run.app.world(), actor, 29722)
        .unwrap()
        .expect("selected native Incinerate");
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(actual, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
    for _ in 0..60 {
        run.app.update();
    }
}

fn insight_amount(run: &Run) -> Option<f32> {
    let auras = run.app.world().get::<ActiveAuras>(run.actor)?;
    auras
        .iter()
        .find(|aura| aura.spell_id == 1229746 && aura.caster == run.actor)
        .map(|aura| aura.effects[0].amount)
}

fn intellect(run: &Run) -> f32 {
    run.app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .primary
        .intellect
}

/// Native effect0 value times effect1 player share, at each equipped item level.
fn read_expected_insight(run: &Run) -> f32 {
    let world = run.app.world();
    let levels: Vec<Option<u16>> = world
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .filter(|aura| aura.spell_id == 1283697 && matches!(aura.source, AuraSource::Equipment(_)))
        .map(|aura| aura.cast_item_level)
        .collect();
    assert!(!levels.is_empty(), "profile equips Arcanoweave Lining");
    levels
        .into_iter()
        .map(|level| {
            let mut spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(1283697)
                .unwrap()
                .clone();
            spell.cast_item_level = level;
            let value = |index: u8| {
                let effect = spell.effects.iter().find(|e| e.index == index).unwrap();
                crate::spell_cast::effects::modded_base_points(world, run.actor, &spell, effect)
            };
            value(0) * value(1) / 100.0
        })
        .sum()
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn item_stat_proc_arcanoweave_lining_earns_insight_intellect() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    engage(&mut run);
    let expected = read_expected_insight(&run);
    assert!(expected > 0.0);
    let mut before = intellect(&run);
    for _ in 0..200 {
        if let Some(amount) = insight_amount(&run) {
            // SimC76f8118 unique_gear_midnight.cpp682–685,712–714.
            assert!(
                (amount - expected).abs() < 1.0,
                "Insight uses the equipped driver's native share: {amount} vs {expected}"
            );
            let after = intellect(&run);
            assert!(
                after - before >= amount - 1.0,
                "Insight raises intellect: {before} -> {after}"
            );
            return;
        }
        before = intellect(&run);
        paid_incinerate(&mut run);
    }
    panic!("actual combat casts must earn Arcanoweave Insight");
}
