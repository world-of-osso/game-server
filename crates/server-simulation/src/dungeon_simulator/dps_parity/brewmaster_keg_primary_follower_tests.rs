use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;

#[test]
#[ignore = "requires selected MID2 Brewmaster profile and world data"]
fn dps_parity_brewmaster_keg_smash_applies_both_bonuses_only_to_primary() {
    for (index, expected) in [(0, 2.25), (1, 1.0)] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_registered_player(268, 2).unwrap();
        let actor = run.actor;
        let primary = run.targets[0];
        let observed = run.targets[index];
        assert!(crate::auras::has_aura(run.app.world(), actor, 383707));
        engage(&mut run);
        run.app
            .insert_resource(FactorCapture::new(actor, observed, 121253, "keg_smash", 1));
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(primary), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let trace = run.app.world().resource::<FactorCapture>().export();
        let result = trace["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| {
                event["event"] == "direct_amount"
                    && event["spell_id"] == 121253
                    && event["target"] == observed.to_bits().to_string()
            })
            .expect("Keg Smash hit target");
        let resolved = trace["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| {
                event["event"] == "result"
                    && event["spell_id"] == 121253
                    && event["target"] == observed.to_bits().to_string()
            })
            .unwrap();
        let initial = result["factors"]["native_scaled_amount"].as_f64().unwrap();
        let input = resolved["factors"]["native_input_amount"].as_f64().unwrap();
        let crit = 1.0 + resolved["factors"]["crit_bonus"].as_f64().unwrap();
        let multiplier = input / (initial * crit);
        assert!(
            (multiplier / expected - 1.0).abs() < 0.026,
            "target {index}: multiplier={multiplier}, input={input}, scaled={initial}, crit={crit}"
        );
    }
}
