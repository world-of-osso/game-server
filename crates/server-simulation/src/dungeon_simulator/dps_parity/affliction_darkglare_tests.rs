//! Actual Darkglare beams use the source-defined full live owner spell power.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_darkglare_native_beam_inherits_full_owner_spell_power() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    run.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::networking::CombatTarget(Some(target)));
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, 205180, Some(caster), false)
        .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let actor = run
        .app
        .world_mut()
        .query::<(
            Entity,
            &crate::spell_cast::summon::SummonedCreature,
            &shared::components::Npc,
        )>()
        .iter(run.app.world())
        .find(|(_, summon, npc)| summon.owner == caster && npc.template_id == 103673)
        .map(|(entity, _, _)| entity)
        .expect("actual native205180 must summon Darkglare103673");
    let expected = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(caster)
        .unwrap()
        .spell_power;
    run.app
        .world_mut()
        .insert_resource(FactorCapture::new(actor, target, 205231, "eye_beam", 1));
    for _ in 0..100 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(205231)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0),
        "the real Darkglare must land a native Eye Beam"
    );
    let capture = run.app.world().resource::<FactorCapture>().export();
    let formula = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 205231)
        .expect("actual landed Eye Beam must have a native damage calculation");
    let inherited = formula["factors"]["s_power"].as_f64().unwrap();
    // SimC76f8118 sc_warlock_pets.cpp19/2424–2430: Darkglare retains
    // warlock_pet_t's full owner-SP share, not the generic summon share0.15.
    assert!(
        (inherited - f64::from(expected)).abs() < 1.0,
        "native Darkglare must inherit full owner SP: actual={inherited}, owner={expected}"
    );
}
