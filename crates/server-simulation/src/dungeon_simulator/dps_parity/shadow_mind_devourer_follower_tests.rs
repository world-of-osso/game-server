use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn fixture() -> (
    bevy::prelude::App,
    bevy::prelude::Entity,
    bevy::prelude::Entity,
) {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 258, 70);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    (app, actor, target)
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_mind_blast_can_earn_mind_devourer_only_with_the_talent() {
    for selected in [true, false] {
        let _seed = crate::random::seed_scope(70448);
        let (mut app, actor, target) = fixture();
        assert!(crate::auras::has_aura(app.world(), actor, 373202));
        if !selected {
            crate::auras::remove_auras(
                app.world_mut(),
                actor,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == 373202,
            );
        }
        for _ in 0..200 {
            crate::spell_cast::cast_triggered(app.world_mut(), actor, 8092, Some(target), 0);
            if crate::auras::has_aura(app.world(), actor, 373204) {
                break;
            }
        }
        assert_eq!(crate::auras::has_aura(app.world(), actor, 373204), selected);
    }
}

fn cast_madness(buffed: bool) -> (i32, f32, bool) {
    let _seed = crate::random::seed_scope(70448);
    let (mut app, actor, target) = fixture();
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Insanity, 10_000.0);
    if buffed {
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 373204, Some(actor), 0);
        assert!(crate::auras::has_aura(app.world(), actor, 373204));
    }
    let before = crate::spell_cast::powers::power_value(app.world(), actor, PowerType::Insanity)
        .unwrap()
        .0;
    crate::spell_cast::creature_cast(app.world_mut(), actor, 335467, Some(target), false).unwrap();
    for _ in 0..20 {
        app.update();
    }
    let after = crate::spell_cast::powers::power_value(app.world(), actor, PowerType::Insanity)
        .unwrap()
        .0;
    let damage = app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.spell_id == Some(335467) && event.kind == CombatLogKind::Damage && !event.periodic
        })
        .expect("foreground Madness impact")
        .amount;
    (
        damage,
        before - after,
        crate::auras::has_aura(app.world(), actor, 373204),
    )
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_mind_devourer_makes_madness_free_stronger_and_consumes_one_use() {
    let (normal, normal_cost, _) = cast_madness(false);
    let (enhanced, enhanced_cost, retained) = cast_madness(true);
    assert!(normal_cost > 0.0);
    assert_eq!(enhanced_cost, 0.0);
    assert!(!retained);
    assert!(
        (enhanced as f32 / normal as f32 - 1.2).abs() < 0.02,
        "normal={normal} enhanced={enhanced}"
    );
}
