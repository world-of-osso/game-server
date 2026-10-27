//! Actual native Incinerate recipients follow selected Fire and Brimstone/Havoc.
use super::*;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_incinerate_without_fnb_or_havoc_hits_only_actual_primary() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 3).unwrap();
    let actor = run.actor;
    let targets = run.targets.clone();
    let traits = run.app.world().get::<TraitLoadout>(actor).unwrap();
    assert!(
        !traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == 196408),
        "frozen profile does not select Fire and Brimstone"
    );
    for &target in &targets {
        assert!(
            !crate::auras::has_aura(run.app.world(), target, 80240),
            "ordinary unmarked enemies have no Havoc"
        );
    }
    engage(&mut run);
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(29722, targets[0], Some(targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..60 {
        run.app.update();
    }
    // SimC76f8118 actions3918–4002: absent FnB and Havoc, the ordinary
    // paid cast has one actual primary and cannot deal secondary damage.
    for (index, target) in targets.iter().enumerate() {
        let hits = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(29722)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        assert_eq!(
            hits,
            usize::from(index == 0),
            "unselected FnB/no Havoc must not hit secondary {index}"
        );
    }
}
