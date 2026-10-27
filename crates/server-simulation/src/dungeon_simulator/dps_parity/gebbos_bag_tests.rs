//! Equipped Gebbo's Bottomless Bag earns one native artifact through actual combat.
use super::*;
use crate::auras::{ActiveAuras, AuraSource};

const DRIVER: u32 = 1292291;
const RATING: u32 = 189;
const CRIT: i32 = 1792;
const HASTE: i32 = 917504;
const MASTERY: i32 = 33554432;
const VERSATILITY: i32 = 1879048192;

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

/// Native driver effect `index` at the equipped bag's item level.
fn read_driver_value(run: &Run, index: u8) -> f32 {
    let world = run.app.world();
    let level = world
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == DRIVER && matches!(aura.source, AuraSource::Equipment(_)))
        .expect("profile equips Gebbo's Bottomless Bag")
        .cast_item_level;
    let mut spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(DRIVER)
        .unwrap()
        .clone();
    spell.cast_item_level = level;
    let effect = spell.effects.iter().find(|e| e.index == index).unwrap();
    crate::spell_cast::effects::modded_base_points(world, run.actor, &spell, effect)
}

/// SimC76f8118 unique_gear_midnight.cpp3666–3735: each artifact's total rating
/// per secondary at application (stacks times the per-stack amount).
fn expected_ratings(run: &Run, artifact: u32) -> Vec<(i32, f32)> {
    let all = |amount: f32| {
        vec![
            (CRIT, amount),
            (HASTE, amount),
            (MASTERY, amount),
            (VERSATILITY, amount),
        ]
    };
    match artifact {
        1308013 => all(read_driver_value(run, 5)),
        1308014 => all(-read_driver_value(run, 6)),
        1308012 => vec![(HASTE, read_driver_value(run, 3))],
        1306870 => vec![(MASTERY, read_driver_value(run, 1))],
        1292299 => vec![(CRIT, read_driver_value(run, 0))],
        1292300 => {
            let decrement = read_driver_value(run, 7);
            let stacks = (read_driver_value(run, 2) / decrement).round().max(1.0);
            vec![(VERSATILITY, decrement * stacks)]
        }
        other => panic!("unexpected artifact {other}"),
    }
}

fn applied_artifact(run: &Run) -> Option<(u32, Vec<(i32, f32)>)> {
    let auras = run.app.world().get::<ActiveAuras>(run.actor)?;
    auras
        .iter()
        .find(|aura| {
            matches!(
                aura.spell_id,
                1308013 | 1308014 | 1308012 | 1306870 | 1292299 | 1292300
            )
        })
        .map(|aura| {
            let ratings = aura
                .effects
                .iter()
                .filter(|effect| effect.aura_type == RATING)
                .map(|effect| {
                    (
                        effect.misc_values[0],
                        effect.amount * f32::from(aura.stacks.max(1)),
                    )
                })
                .collect();
            (aura.spell_id, ratings)
        })
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn gebbos_bag_actual_proc_grants_a_native_artifact_rating() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    engage(&mut run);
    for _ in 0..200 {
        if let Some((artifact, mut ratings)) = applied_artifact(&run) {
            let mut expected = expected_ratings(&run, artifact);
            ratings.sort_by_key(|(mask, _)| *mask);
            expected.sort_by_key(|(mask, _)| *mask);
            assert_eq!(
                ratings.len(),
                expected.len(),
                "artifact {artifact} rating effects"
            );
            for ((mask, amount), (_, want)) in ratings.iter().zip(&expected) {
                assert!(
                    (amount - want).abs() < 1.0,
                    "artifact {artifact} rating {mask}: {amount} vs native {want}"
                );
            }
            return;
        }
        paid_incinerate(&mut run);
    }
    panic!("actual combat casts must earn a Gebbo's Bottomless Bag artifact");
}
