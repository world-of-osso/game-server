//! Selected Enlightened uses the native sampled-Mana Arcane damage fraction.
use super::*;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use shared::components::Mana;

fn missiles_damage(selected: bool, fraction: f32) -> f64 {
    let _seed = crate::random::seed_scope(70448);
    let (profile, mut traits) = read_reference(62).unwrap();
    if !selected {
        traits.retain(|entry| entry.entry_id != 126540);
    }
    let identity = inputs::read_actor(&profile).unwrap();
    let mut run = prepare_literal_reference(profile, &traits, 1, identity).unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 321387),
        selected
    );
    let actor = run.actor;
    let target = run.targets[0];
    let mut mana = run.app.world_mut().get_mut::<Mana>(actor).unwrap();
    mana.current = mana.max * fraction;
    let end = crate::spell_cast::now(run.app.world()) + 1.1;
    while crate::spell_cast::now(run.app.world()) < end {
        run.app.update();
    }
    run.app.insert_resource(FactorCapture::new(
        actor,
        target,
        7268,
        "enlightened_native",
        1,
    ));
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 7268, Some(target), 1);
    let end = crate::spell_cast::now(run.app.world()) + 1.0;
    while crate::spell_cast::now(run.app.world()) < end {
        run.app.update();
    }
    let trace = run.app.world().resource::<FactorCapture>().export();
    trace["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["spell_id"] == 7268 && event["event"] == "direct_amount")
        .expect("real native Arcane Missiles damage")["factors"]["amount"]
        .as_f64()
        .unwrap()
}

#[test]
#[ignore = "requires private native70448 Arcane player profile"]
fn arcane_enlightened_native_sampled_mana_increases_arcane_missiles_damage() {
    let without = missiles_damage(false, 0.5);
    let with = missiles_damage(true, 0.5);
    let ratio = with / without;
    assert!(
        ratio > 1.029 && ratio < 1.035,
        "selected native Enlightened should increase actual Arcane damage by 6% of the sampled ~half Mana, without={without} with={with} ratio={ratio}"
    );
}

fn sampled_mana_damage_pair(selected: bool) -> Vec<f64> {
    let _seed = crate::random::seed_scope(70448);
    let (profile, mut traits) = read_reference(62).unwrap();
    if !selected {
        traits.retain(|entry| entry.entry_id != 126540);
    }
    let identity = inputs::read_actor(&profile).unwrap();
    let mut run = prepare_literal_reference(profile, &traits, 1, identity).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    let mut mana = run.app.world_mut().get_mut::<Mana>(actor).unwrap();
    mana.current = mana.max * 0.5;
    let end = crate::spell_cast::now(run.app.world()) + 1.1;
    while crate::spell_cast::now(run.app.world()) < end {
        run.app.update();
    }
    run.app.world_mut().get_mut::<Mana>(actor).unwrap().current = 0.0;
    run.app
        .insert_resource(FactorCapture::new(actor, target, 7268, "sampled_mana", 2));
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 7268, Some(target), 1);
    let end = crate::spell_cast::now(run.app.world()) + 1.2;
    while crate::spell_cast::now(run.app.world()) < end {
        run.app.update();
    }
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 7268, Some(target), 1);
    let end = crate::spell_cast::now(run.app.world()) + 0.5;
    while crate::spell_cast::now(run.app.world()) < end {
        run.app.update();
    }
    let trace = run.app.world().resource::<FactorCapture>().export();
    trace["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["spell_id"] == 7268 && event["event"] == "direct_amount")
        .map(|event| event["factors"]["amount"].as_f64().unwrap())
        .collect()
}

#[test]
#[ignore = "requires private native70448 Arcane player profile"]
fn arcane_enlightened_native_mana_change_waits_for_next_periodic_tick() {
    let without = sampled_mana_damage_pair(false);
    let with = sampled_mana_damage_pair(true);
    assert_eq!(
        (without.len(), with.len()),
        (2, 2),
        "two real Missiles impacts"
    );
    let before_tick = with[0] / without[0];
    let after_tick = with[1] / without[1];
    assert!(
        before_tick > 1.029 && before_tick < 1.035,
        "half-Mana sample must survive a later Mana drop until the next tick: {before_tick}"
    );
    assert!(
        after_tick >= 1.0 && after_tick < 1.005,
        "next tick must sample the near-empty Mana pool: {after_tick}"
    );
}
