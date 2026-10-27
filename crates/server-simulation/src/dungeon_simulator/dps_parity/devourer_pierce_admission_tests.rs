//! Source-selected Pierce the Veil native action admission.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_selected_demonsurge_admits_pierce_in_meta() {
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    assert!(run.app.world().get::<crate::class_progression::TraitLoadout>(actor).is_some_and(|traits| traits.active_talent_spells().any(|spell| spell.spell_id == 452402)), "profile must select native Demonsurge");
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    for _ in 0..30 {
        if crate::auras::has_aura(run.app.world(), actor, 1217607) && !crate::auras::has_aura(run.app.world(), actor, 201453) { break; }
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), actor, 1217607), "native Meta must be active");
    assert!(!crate::auras::has_aura(run.app.world(), actor, 201453), "native Meta pacify must expire");
    assert!(!crate::auras::has_aura(run.app.world(), actor, 1239525), "Hungering Slash must not replace Pierce");
    // SimC76f8118 sc_demon_hunter.cpp5874–5889,11383–11384: selected
    // Demonsurge exposes Pierce the Veil in Devourer Meta.
    let spell = literal_rotation::execution::resolve_action(run.app.world(), actor, 1245483).expect("native Pierce resolution").expect("selected Demonsurge must admit native Pierce");
    assert_eq!(spell, 1245483, "native Pierce without its replacement");
    literal_rotation::execution::execute(run.app.world_mut(), actor, literal_rotation::execution::Execution::Spell(spell, target, Some(target))).expect("real native Pierce admission");
    for _ in 0..40 {
        if run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.iter().any(|event| event.source == Some(actor.to_bits()) && event.spell_id == Some(1245483) && event.kind == CombatLogKind::CastSuccess) { return; }
        run.app.update();
    }
    panic!("admitted native Pierce must execute");
}
