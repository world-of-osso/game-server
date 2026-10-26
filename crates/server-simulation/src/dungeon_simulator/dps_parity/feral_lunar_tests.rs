//! Lunar Inspiration owns its native override aura, not extra learned spells.
use super::*;

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_selected_lunar_inspiration_applies_and_removes_native_override() {
    let mut run = prepare_literal(103, 1).unwrap();
    let world = run.app.world_mut();
    let actor = run.actor;
    assert!(crate::auras::has_aura(world, actor, 155580));
    assert!(
        crate::auras::has_aura(world, actor, 155627),
        "selected Lunar Inspiration must apply its native Moonfire override"
    );
    assert_eq!(
        server::spell_cast::knowledge::resolve_action(world, actor, 8921).unwrap(),
        Some(155625)
    );
    let known: Vec<_> = world
        .get::<crate::class_progression::KnownSpells>(actor)
        .unwrap()
        .iter()
        .collect();
    crate::auras::remove_auras(
        world,
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 155580,
    );
    assert!(!crate::auras::has_aura(world, actor, 155627));
    assert_eq!(
        world
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        known,
        "removing the talent changes its override, not the learned book"
    );
    assert_eq!(
        server::spell_cast::knowledge::resolve_action(world, actor, 8921).unwrap(),
        Some(8921)
    );
    crate::auras::apply_aura(
        world,
        &crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 155580,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(world, actor, 155627));
    assert_eq!(
        server::spell_cast::knowledge::resolve_action(world, actor, 8921).unwrap(),
        Some(155625)
    );
    crate::spell_cast::creature_cast(world, actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let moonfire = server::spell_cast::knowledge::resolve_actor_action(
        run.app.world(), actor, 8921,
    ).unwrap();
    crate::spell_cast::creature_cast(
        run.app.world_mut(), actor, moonfire, Some(run.targets[0]), false,
    ).expect("ordinary selected Moonfire is usable in native Cat Form");
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.targets[0], 155625));
}
