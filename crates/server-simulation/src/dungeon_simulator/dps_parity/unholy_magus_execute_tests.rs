//! Real Magus and Lord damage at the native four-piece health boundary.
use super::*;
use crate::spell_cast::{effects::factor_capture::FactorCapture, summon::SummonedCreature};
use shared::components::{Health, Npc};

fn summon_magus(run: &mut Run, lord: bool) -> Entity {
    let owner = run.actor;
    if lord {
        for _ in 0..2 {
            literal_rotation::execution::execute(
                run.app.world_mut(),
                owner,
                literal_rotation::execution::Execution::Spell(
                    1247378,
                    run.targets[0],
                    Some(run.targets[0]),
                ),
            )
            .unwrap();
            for _ in 0..50 {
                run.app.update();
            }
        }
    } else {
        literal_rotation::execution::execute(
            run.app.world_mut(),
            owner,
            literal_rotation::execution::Execution::Spell(42650, owner, None),
        )
        .unwrap();
        preparation::finish_pending_setup(run).unwrap();
    }
    let expected = if lord { 264321 } else { 163366 };
    let world = run.app.world_mut();
    world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .find_map(|(entity, npc, summon)| {
            (npc.template_id == expected && summon.owner == owner).then_some(entity)
        })
        .expect("real selected Magus or Lord")
}

fn execute_factor(
    run: &mut Run,
    pet: Entity,
    spell: u32,
    periodic: bool,
    health_percent: f32,
    cast: bool,
) -> f64 {
    let target = run.targets[0];
    let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
    health.current = health.max * health_percent / 100.0;
    run.app
        .insert_resource(FactorCapture::new(pet, target, spell, "magus_execute", 1));
    if cast {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            pet,
            spell,
            Some(target),
            1
        ));
    }
    for _ in 0..160 {
        run.app.update();
        let capture = run.app.world().resource::<FactorCapture>().export();
        let kind = if periodic {
            "periodic_amount"
        } else {
            "direct_amount"
        };
        if let Some(hit) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == kind && event["spell_id"] == spell)
        {
            return hit["applied_multipliers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|factor| factor["factor"] == "ret_damage")
                .unwrap()["value"]
                .as_f64()
                .unwrap();
        }
    }
    panic!("real Magus action {spell} did not hit");
}

#[test]
#[ignore = "requires native252 profile and real Magus/Lord summons"]
fn unholy_magus_four_piece_uses_live_target_health_boundary() {
    for (spell, periodic, lord, effect) in [
        (1297055, false, false, 0),
        (1297086, false, true, 0),
        (1297091, true, false, 1),
    ] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(252, 1).unwrap();
        engage(&mut run);
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 1296655));
        let pet = summon_magus(&mut run, lord);
        let threshold =
            crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1296655, 2);
        let bonus = crate::spell_cast::scripts::script_effect_value(
            run.app.world(),
            run.actor,
            1296655,
            effect,
        );
        let above = execute_factor(&mut run, pet, spell, periodic, threshold + 1.0, true);
        // Withering Grasp must react to health crossing the boundary without a recast.
        let at = execute_factor(&mut run, pet, spell, periodic, threshold, !periodic);
        assert!(
            (at / above - (1.0 + f64::from(bonus) / 100.0)).abs() < 0.001,
            "native action {spell} must gain the four-piece bonus at the threshold: {above} -> {at}"
        );
    }
}
