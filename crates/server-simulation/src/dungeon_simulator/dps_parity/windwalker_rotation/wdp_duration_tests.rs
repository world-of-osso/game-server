use super::*;

#[test]
#[ignore = "native earned enabler duration: actual overlapping cooldowns plus source hasted grace"]
fn literal_ww_native_wdp_duration_matches_overlap_plus_hasted_grace() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(269, 1).unwrap();
    let start = run.app.world().resource::<Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, start);
    super::super::engage(&mut run);
    let mut rotation = (run.spec_driver.take().unwrap() as Box<dyn std::any::Any>).downcast::<Rotation>().unwrap();
    for _ in 0..600 {
        rotation
            .drive(run.app.world_mut(), run.actor, &run.targets)
            .unwrap();
        let duration = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 196742 && aura.caster == run.actor)
            .map(|aura| aura.duration_ms);
        if let Some(actual) = duration {
            let rsk = read_fact(&mut run, &rotation, "cooldown.rising_sun_kick.remains");
            let fof = read_fact(&mut run, &rotation, "cooldown.fists_of_fury.remains");
            let haste = read_fact(&mut run, &rotation, "spell_haste");
            assert!(
                rsk > 0.0 && fof > 0.0,
                "actual native cooldowns must overlap"
            );
            let expected = (rsk.min(fof) * 1000.0).round() as u32 + (1500.0 * haste) as u32;
            assert_eq!(
                actual, expected,
                "SIMC_WW_WDP_ENABLER_DURATION: native overlap plus truncated hasted1500ms grace"
            );
            return;
        }
        run.app.update();
    }
    panic!("actual RSK/FoF source casts must earn a native enabler window");
}

fn read_fact(run: &mut super::super::Run, rotation: &Rotation, name: &str) -> f64 {
    let action = Action {
        source_row: None,
        name: "whirling_dragon_punch".into(),
        target: None,
        options: Default::default(),
    };
    rotation
        .value_for(run.app.world_mut(), run.actor, &run.targets, &action, name)
        .unwrap()
}
