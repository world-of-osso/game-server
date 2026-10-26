//! Literal Feral lifecycle must retain the selected actor and native precombat.
use super::*;

#[test]
#[ignore = "requires spec103 profile/traits and native private DB"]
fn feral_literal_player_precombat_owns_cat_prowl_and_controller() {
    use shared::components::Player;
    for count in [1, 3] {
        let run = prepare_measurement(103, count).expect("exact selected Feral fixture");
        assert!(run.app.world().get::<Player>(run.actor).is_some());
        let driver = run.spec_driver.as_ref().expect("literal Feral controller");
        assert_eq!(driver.label(), "LITERAL_SIMC_FERAL_76f8118");
        assert!(
            run.player_rotation.is_none(),
            "no AssistedCombat replacement"
        );
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 768));
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 5215));
        assert!(driver.report()["native_precombat_snapshot"].is_object());
    }
}
