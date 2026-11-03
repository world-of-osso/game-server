//! The equipped random-stat neck must update real rating auras on proc and refresh.
use super::*;
use crate::auras::{ActiveAuras, RemoveReason};
use crate::stats::StatModifiers;

#[test]
#[ignore = "native263 reference neck and private world database"]
fn enhancement_neck_random_stat_proc_switches_live_rating_on_refresh() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(263, 1).unwrap();
    let actor = run.actor;
    let driver = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1317582 && aura.caster == actor)
        .expect("actual equipped Aqirbane Reliquary proc")
        .clone();
    assert_eq!(driver.cast_item_level, Some(344));
    assert_eq!(driver.cast_item_inventory_type, Some(2));
    let positive = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    let negative = driver
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    // RandPropPoints344 SuperiorF_0 393.575195 * neck multiplier 1.10491686
    // * native coefficients .997961879/-.167881429; SimC reports 433/-73.
    assert!(
        (positive - 433.0).abs() <= 1.0 && negative == -73.0,
        "native item-scaled driver: {positive}/{negative}"
    );
    let child = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .trigger_spell;
    assert_eq!(child, 1317581);
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    let mut previous = None;
    for _ in 0..TICKS {
        run.app.update();
        drive_player(&mut run).unwrap();
        let Some(buff) = run
            .app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == child && aura.caster == actor)
            .cloned()
        else {
            previous = None;
            continue;
        };
        let mut effects = buff.effects.clone();
        effects.sort_by_key(|effect| effect.index);
        assert_eq!(effects.len(), 4);
        let selected: Vec<_> = effects
            .iter()
            .enumerate()
            .filter(|(_, effect)| effect.amount == positive)
            .collect();
        assert_eq!(
            selected.len(),
            1,
            "exactly one secondary gains the scaled amount"
        );
        assert!(
            effects
                .iter()
                .all(|effect| effect.amount == positive || effect.amount == negative)
        );
        let chosen = selected[0].0;
        if previous.is_some_and(|(instance, index)| instance == buff.instance_id && index != chosen)
        {
            let before = run
                .app
                .world()
                .get::<StatModifiers>(actor)
                .unwrap()
                .ratings_flat;
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                RemoveReason::Cancelled,
                |aura| aura.spell_id == child && aura.caster == actor,
            );
            let after = run
                .app
                .world()
                .get::<StatModifiers>(actor)
                .unwrap()
                .ratings_flat;
            let delta = [
                before.crit - after.crit,
                before.haste - after.haste,
                before.mastery - after.mastery,
                before.versatility - after.versatility,
            ];
            for (index, actual) in delta.into_iter().enumerate() {
                let expected = if index == chosen { positive } else { negative };
                assert!(
                    (actual - expected).abs() < 0.01,
                    "rating {index}: {actual} != {expected}"
                );
            }
            return;
        }
        previous = Some((buff.instance_id, chosen));
    }
    panic!("real neck procs did not refresh with a different secondary stat");
}
