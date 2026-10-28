//! Real Dark Transformation and native Magus damage.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::summon::SummonedCreature;

fn next_bolt(run: &mut Run, magus: Entity) -> f64 {
    run.app.insert_resource(FactorCapture::new(
        magus,
        run.targets[0],
        1297055,
        "magus_bolt",
        1,
    ));
    for _ in 0..180 {
        run.app.update();
        let capture = run.app.world().resource::<FactorCapture>().export();
        if let Some(hit) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "direct_amount")
        {
            return hit["applied_multipliers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|factor| factor["factor"] == "school_damage_done")
                .unwrap()["value"]
                .as_f64()
                .unwrap();
        }
    }
    panic!("actual Magus must complete a real Necrotic Bolt");
}

#[test]
#[ignore = "requires native252 profile and Magus templates"]
fn unholy_commander_tracks_real_dark_transformation_and_magus_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 390259));
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 390260));
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let owner = run.actor;
    let magus = {
        let world = run.app.world_mut();
        world
            .query::<(Entity, &SummonedCreature)>()
            .iter(world)
            .find_map(|(entity, summon)| {
                (summon.owner == owner && summon.spell_id == 317776).then_some(entity)
            })
            .expect("real Army must summon its Magus")
    };
    let before = next_bolt(&mut run, magus);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(1233448, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 390260));
    let percent =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 390264, 0);
    assert!(
        percent > 0.0,
        "selected native Commander must provide a pet damage bonus"
    );
    let after = next_bolt(&mut run, magus);
    assert!(
        (after / before - (1.0 + f64::from(percent) / 100.0)).abs() < 0.001,
        "actual Magus hits must gain the sourced Commander effect: {before} -> {after}"
    );
}
