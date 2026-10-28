//! Local real-opener Lord damage factors.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::summon::SummonedCreature;

#[test]
#[ignore = "local native252 Magus factor diagnostic"]
fn unholy_magus_actual_opener_factors_diagnostic() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).unwrap();
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    let mut captured = false;
    for _ in 0..1200 {
        drive_player(&mut run).unwrap();
        run.app.update();
        if !captured {
            let owner = run.actor;
            let world = run.app.world_mut();
            let lord = world
                .query::<(Entity, &SummonedCreature)>()
                .iter(world)
                .find_map(|(entity, summon)| {
                    (summon.owner == owner && summon.spell_id == 1292072).then_some(entity)
                });
            if let Some(lord) = lord {
                run.app.insert_resource(FactorCapture::new(
                    lord,
                    run.targets[0],
                    1297086,
                    "lord_necrotic_bolt",
                    4,
                ));
                captured = true;
            }
        } else {
            let capture = run.app.world().resource::<FactorCapture>().export();
            if let Some(hit) = capture["events"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["event"] == "direct_amount")
            {
                println!(
                    "opener lord elapsed={} hit={hit}",
                    run.app.world().resource::<Time>().elapsed_secs_f64() - started.as_secs_f64()
                );
                return;
            }
        }
    }
    panic!("no actual Lord bolt in the literal opener");
}
