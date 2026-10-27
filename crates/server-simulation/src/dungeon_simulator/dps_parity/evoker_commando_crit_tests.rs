//! An actual passive Commando's Pyre inherits its owner's live critical chance.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::summon::SummonedCreature;
use shared::components::Npc;

#[test]
#[ignore = "requires native Devastation profile and canonical-source Commando templates"]
fn evoker_commando_crit_actual_flight_pyre_reads_live_owner_critical_chance() {
    let mut run = prepare_literal(1467, 1).unwrap();
    engage(&mut run);
    let primary = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1260745));
    crate::spell_cast::creature_cast_with_targets(
        run.app.world_mut(),
        run.actor,
        433874,
        crate::spell_cast::CastTargets {
            unit: run.actor,
            primary: Some(primary),
            destination: None,
        },
    )
    .unwrap();
    let owner = run.actor;
    let mut pets = Vec::new();
    for _ in 0..10 {
        run.app.update();
        pets = run
            .app
            .world_mut()
            .query::<(Entity, &Npc, &SummonedCreature)>()
            .iter(run.app.world())
            .filter(|(_, npc, summon)| npc.template_id == 219827 && summon.owner == owner)
            .map(|(entity, _, _)| entity)
            .collect();
        if !pets.is_empty() {
            break;
        }
    }
    assert_eq!(
        pets.len(),
        2,
        "actual Command Squadron must produce two real owned actors"
    );
    run.app.world_mut().insert_resource(FactorCapture::new(
        pets[0],
        primary,
        1236970,
        "native_commando_pyre",
        3,
    ));
    for _ in 0..240 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let chances: Vec<_> = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["event"] == "result" && event["spell_id"] == 1236970)
        .map(|event| event["factors"]["crit"].as_f64().unwrap() * 100.0)
        .collect();
    assert!(
        !chances.is_empty(),
        "actual pet flight must produce its native Pyre"
    );
    let live = f64::from(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(owner)
            .unwrap()
            .crit_pct,
    );
    // SimC76f8118 pet.cpp217–220: live owner spell crit applies to guardian casts.
    assert!(
        chances.iter().all(|chance| (chance - live).abs() < 0.01),
        "each actual Commando Pyre must roll its owner's live{live}% chance: {chances:?}"
    );
}
