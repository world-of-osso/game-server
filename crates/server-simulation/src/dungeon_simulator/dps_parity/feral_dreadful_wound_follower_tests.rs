//! Landed Ravage gives the selected follower its native Dreadful Wound.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_ravage_applies_dreadful_wound() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
    assert!(
        crate::auras::has_aura(app.world(), actor, 441809),
        "Dreadful Wound selected"
    );
    assert!(
        crate::auras::has_aura(app.world(), actor, 1271839),
        "Exacerbating Wounds selected"
    );
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    let rip = app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1079)
        .unwrap()
        .clone();
    let before = crate::auras::damage_taken_multiplier(app.world(), target, actor, &rip);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 441591, Some(target), 1);
    let aura = app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 441812 && a.caster == actor)
        .expect("Ravage earns owned wound");
    assert_eq!(aura.duration_ms, 6000);
    let after = crate::auras::damage_taken_multiplier(app.world(), target, actor, &rip);
    assert!(
        (after / before - 1.08).abs() < 0.001,
        "wound vulnerability {before} -> {after}"
    );
    for _ in 0..50 {
        app.update();
    }
    let damage = app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(441812)
                && event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.amount > 0
        });
    assert!(damage, "the wound deals actual periodic damage");
}
