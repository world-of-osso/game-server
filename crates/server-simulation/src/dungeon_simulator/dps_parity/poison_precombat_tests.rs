//! Boundary control for13120's native coatings; composite selection remains14775-owned.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared259/current native view and DPS_PARITY_ACTOR=player; four native1500ms coatings"]
fn native_poison_precombat_completes_every_selected_coating_before_pull() {
    let _seed = crate::random::seed_scope(70448);
    assert_eq!(std::env::var("DPS_PARITY_ACTOR").unwrap(), "player");
    let mut run = prepare_literal(259, 1).unwrap();
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    assert!(
        known.contains(381801),
        "actual selected Dragon-Tempered Blades required"
    );
    assert!(
        known.contains(381637),
        "actual selected Atrophic Poison required"
    );
    let coatings = [2823, 3408, 381664, 381637];
    for spell in coatings {
        let before = crate::spell_cast::now(run.app.world());
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(spell, run.actor, None),
        )
        .unwrap_or_else(|error| panic!("native coating{spell} admission: {error}"));
        let duration = run
            .app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .unwrap_or_else(|| panic!("native coating{spell} must not be treated as instant"))
            .duration;
        assert!(duration > 0.0, "native cast metadata must remain effective");
        preparation::finish_pending_setup(&mut run).unwrap();
        let elapsed = crate::spell_cast::now(run.app.world()) - before;
        assert!(
            elapsed + 0.001 >= f64::from(duration),
            "coating{spell} must finish its real cast"
        );
        assert!(
            run.app
                .world()
                .get::<shared::casting::CastState>(run.actor)
                .is_none()
        );
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let casts: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
        })
        .filter_map(|event| event.spell_id.filter(|id| coatings.contains(id)))
        .collect();
    assert_eq!(
        casts, coatings,
        "all selected native coatings must complete in constructor order"
    );
    for spell in coatings {
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, spell),
            "selected native coating{spell} must not be dropped by a later coating"
        );
    }
    for &target in &run.targets {
        assert!(!crate::combat_refs::has_active_reference_to(
            run.app.world(),
            run.actor,
            target
        ));
    }
}
