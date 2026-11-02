//! Apex Bite and Ravage retain resources but act as maximum-point finishers.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_apex_bite_and_ravage_are_free_max_point_spenders() {
    for ravage in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2813).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
        if ravage {
            crate::spell_cast::cast_triggered(app.world_mut(), actor, 441585, Some(actor), 1);
        }
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 391882, Some(actor), 1);
        assert!(crate::auras::has_aura(app.world(), actor, 391882));
        crate::spell_cast::powers::change_power(
            app.world_mut(),
            actor,
            PowerType::ComboPoints,
            -100.0,
        );
        crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
        let energy = crate::spell_cast::powers::power_value(app.world(), actor, PowerType::Energy)
            .unwrap()
            .0;
        let spell =
            crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 22568).unwrap();
        crate::spell_cast::creature_cast(app.world_mut(), actor, spell, Some(target), false)
            .unwrap();
        let remaining =
            crate::spell_cast::powers::power_value(app.world(), actor, PowerType::Energy)
                .unwrap()
                .0;
        assert_eq!(
            remaining, energy,
            "Apex consumes no Energy, including extra Energy; ravage={ravage}"
        );
        assert!(
            !crate::auras::has_aura(app.world(), actor, 391882),
            "Apex use is consumed; ravage={ravage}"
        );
        let sabertooth = app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 391722 && a.caster == actor)
            .map(|a| a.stacks);
        assert_eq!(
            sabertooth,
            Some(5),
            "Apex acts at maximum combo points without spending them; ravage={ravage}"
        );
    }
}
