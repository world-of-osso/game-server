//! Local real-combat permanent ghoul factor diagnostic.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;

#[test]
#[ignore = "local 252 ghoul damage diagnostic"]
fn unholy_ghoul_actual_claw_factor_diagnostic() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).unwrap();
    let start = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, start);
    engage(&mut run);
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).expect("native ghoul");
    run.app.insert_resource(FactorCapture::new(
        pet,
        run.targets[0],
        91778,
        "sweeping_claws",
        4,
    ));
    for _ in 0..1200 {
        drive_player(&mut run).unwrap();
        run.app.update();
        let capture = run.app.world().resource::<FactorCapture>().export();
        if let Some(hit) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["event"] == "direct_amount")
        {
            println!(
                "owner pet multipliers {:?}",
                run.app
                    .world()
                    .get::<crate::auras::ActiveAuras>(run.actor)
                    .unwrap()
                    .iter()
                    .flat_map(|aura| aura
                        .effects
                        .iter()
                        .filter(|effect| effect.aura_type == 429 || effect.aura_type == 531)
                        .map(move |effect| (
                            aura.spell_id,
                            aura.stacks,
                            effect.aura_type,
                            effect.amount
                        )))
                    .collect::<Vec<_>>()
            );
            println!(
                "target pet debuffs {:?}",
                run.app
                    .world()
                    .get::<crate::auras::ActiveAuras>(run.targets[0])
                    .unwrap()
                    .iter()
                    .flat_map(|aura| aura
                        .effects
                        .iter()
                        .filter(|effect| effect.aura_type == 380 || effect.aura_type == 381)
                        .map(move |effect| (
                            aura.spell_id,
                            aura.stacks,
                            effect.aura_type,
                            effect.amount,
                            aura.caster.to_bits()
                        )))
                    .collect::<Vec<_>>()
            );
            println!(
                "ghoul elapsed={} hit={hit}",
                run.app.world().resource::<Time>().elapsed_secs_f64() - start.as_secs_f64()
            );
            return;
        }
    }
    panic!("actual controlled ghoul must deliver Sweeping Claws");
}
