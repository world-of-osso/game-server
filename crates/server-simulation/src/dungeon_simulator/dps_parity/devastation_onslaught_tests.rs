//! Onslaught grants Burnout only on a selected actor's real combat transition.
use super::*;

#[test]
#[ignore = "requires native Devastation follower profile and world DB"]
fn devastation_onslaught_grants_burnout_on_actual_combat_entry() {
    for selected in [false, true] {
        let mut run = prepare_literal(1467, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        assert!(crate::auras::has_aura(run.app.world(), actor, 441245));
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 441245,
            );
        }
        assert!(!crate::auras::has_aura(run.app.world(), actor, 375802));
        crate::combat_refs::set_in_combat_with(run.app.world_mut(), actor, target);
        for _ in 0..5 {
            run.app.update();
        }
        assert_eq!(
            crate::auras::has_aura(run.app.world(), actor, 375802),
            selected,
            "combat entry only grants the real Burnout aura when Onslaught is selected"
        );
    }
}
