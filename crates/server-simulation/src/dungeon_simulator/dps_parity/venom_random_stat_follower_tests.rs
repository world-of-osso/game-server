//! Vile Vial applies one random rating per phase, with native item scaling.
use super::*;
use crate::simc_apl_native::Bindings;
use literal_rotation::{
    equipment::Equipment,
    execution::{self, Execution},
};

fn ratings(run: &Run) -> Vec<f64> {
    let stats = report::stats(run.app.world(), run.actor);
    ["crit", "haste", "mastery", "versatility"]
        .into_iter()
        .map(|name| stats["ratings"][name].as_f64().unwrap())
        .collect()
}

#[test]
#[ignore = "requires native262 profile and frozen world data"]
fn vile_vial_refresh_replaces_the_previous_random_rating() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(262, 1).unwrap();
    let mut bindings = Bindings::default();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let item = equipment
        .uses()
        .find(|(_, item)| item.item_id == 273796)
        .unwrap()
        .1;
    let before = ratings(&run);
    let actor = run.actor;
    execution::execute(run.app.world_mut(), actor, Execution::Item(item, actor)).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let current = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1293316)
        .unwrap();
    let mut spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1293316)
        .unwrap()
        .clone();
    spell.cast_item_level = current.cast_item_level;
    spell.cast_item_inventory_type = current.cast_item_inventory_type;
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..16 {
        crate::auras::apply_aura_with_spell(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 1293316,
                effect_mask: crate::auras::aura_effect_mask(&spell),
                source: crate::auras::AuraSource::Cast,
            },
            &spell,
        );
        run.app.update();
        let delta: Vec<_> = ratings(&run)
            .iter()
            .zip(&before)
            .map(|(after, before)| after - before)
            .collect();
        assert_eq!(
            delta.iter().filter(|&&value| value > 0.0).count(),
            1,
            "refresh retains one rating: {delta:?}"
        );
        assert!(delta.iter().all(|&value| value >= 0.0));
        seen.insert(delta.iter().position(|&value| value > 0.0).unwrap());
    }
    assert!(
        seen.len() > 1,
        "seeded refreshes must reroll the rating: {seen:?}"
    );
}

#[test]
#[ignore = "requires native262 profile and frozen world data"]
fn vile_vial_item_use_and_expiry_change_one_rating_each() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(262, 1).unwrap();
    let mut bindings = Bindings::default();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let item = equipment
        .uses()
        .find(|(_, item)| item.item_id == 273796)
        .expect("native Vile Vial is equipped")
        .1;
    let before = ratings(&run);
    let actor = run.actor;
    execution::execute(run.app.world_mut(), actor, Execution::Item(item, actor)).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let boosted = ratings(&run);
    let delta: Vec<_> = boosted
        .iter()
        .zip(&before)
        .map(|(after, before)| after - before)
        .collect();
    assert_eq!(
        delta.iter().filter(|&&value| value > 0.0).count(),
        1,
        "one rating gained: {delta:?}"
    );
    assert!(
        delta.iter().all(|&value| value >= 0.0),
        "no rating lost during buff: {delta:?}"
    );
    for _ in 0..400 {
        run.app.update();
    }
    assert!(!crate::auras::has_aura(run.app.world(), actor, 1293316));
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1295123),
        "expiry applies the native debuff"
    );
    let debuffed = ratings(&run);
    let delta: Vec<_> = debuffed
        .iter()
        .zip(&before)
        .map(|(after, before)| after - before)
        .collect();
    assert_eq!(
        delta.iter().filter(|&&value| value < 0.0).count(),
        1,
        "one rating lost: {delta:?}"
    );
    assert!(
        delta.iter().all(|&value| value <= 0.0),
        "no rating gained during debuff: {delta:?}"
    );
}
