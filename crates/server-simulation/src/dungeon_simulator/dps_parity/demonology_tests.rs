//! Real profile/controller boundary, not a second synthetic rotation.
use super::*;

fn prepare_antoran_tyrant() -> (Run, Entity) {
    let mut run = prepare_literal(266, 1).unwrap();
    engage(&mut run);
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        265187,
        Some(run.actor),
        false,
    )
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
        .find(|(_, summon, npc)| summon.owner == run.actor && npc.template_id == 250289)
        .map(|(entity, _, _)| entity)
        .expect("selected Antoran Armaments must summon actual250289");
    (run, actor)
}

fn tyrant_admission_note(run: &mut Run, tyrant: Entity) -> String {
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1264093)
        .unwrap()
        .clone();
    let mut notes = Vec::new();
    for target in [run.targets[0], tyrant] {
        let result =
            crate::spell_cast::read_native_cast_start(run.app.world_mut(), tyrant, &spell, target);
        let result = match result {
            Ok(_) => "ready".to_owned(),
            Err(error) => format!("{error:?}"),
        };
        notes.push(format!("target={} admission={result}", target.to_bits()));
    }
    let world = run.app.world();
    format!(
        "{} summon={:?} position={:?} owner_target={:?}",
        notes.join("; "),
        world.get::<crate::spell_cast::summon::SummonedCreature>(tyrant),
        world.get::<shared::components::Position>(tyrant),
        world
            .get::<crate::networking::CombatTarget>(run.actor)
            .and_then(|target| target.0)
    )
}

#[test]
#[ignore = "requires private266 inputs; actual native alternative Tyrant root"]
fn demonology_antoran_tyrant_repeats_burning_cleave() {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, tyrant) = prepare_antoran_tyrant();
    let admission = tyrant_admission_note(&mut run, tyrant);
    for _ in 0..200 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let hits = events
        .iter()
        .filter(|event| {
            event.source == Some(tyrant.to_bits())
                && event.spell_id == Some(1264093)
                && event.kind == shared::protocol::CombatLogKind::Damage
        })
        .count();
    assert!(
        hits >= 3,
        "real Antoran Tyrant must bind and repeat native Burning Cleave, got{hits}; {admission}"
    );
}

#[test]
#[ignore = "requires private266 inputs; actual native alternative Tyrant SP inheritance"]
fn demonology_antoran_tyrant_inherits_same_owner_sp_as_base_tyrant() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70448);
    let (mut run, tyrant) = prepare_antoran_tyrant();
    let expected = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .spell_power;
    run.app.world_mut().insert_resource(FactorCapture::new(
        tyrant,
        run.targets[0],
        1264093,
        "burning_cleave",
        1,
    ));
    for _ in 0..160 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let formula = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 1264093)
        .expect("actual native Burning Cleave must calculate damage");
    let inherited = formula["factors"]["s_power"].as_f64().unwrap();
    assert!(
        (inherited - f64::from(expected)).abs() < 1.0,
        "alternative Tyrant keeps default owner SP1.0: actual={inherited}, owner={expected}"
    );
}

fn summon_native_dreadstalker(run: &mut Run) -> Entity {
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        193331,
        Some(run.actor),
        true,
    )
    .expect("native triggered Dreadstalker summon must be admitted");
    for _ in 0..100 {
        let dog = run
            .app
            .world_mut()
            .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
            .iter(run.app.world())
            .find(|(_, summon)| summon.owner == run.actor && summon.spell_id == 193331)
            .map(|(unit, _)| unit);
        if let Some(dog) = dog {
            return dog;
        }
        run.app.update();
    }
    panic!("native Dreadstalker must spawn within five seconds");
}

#[test]
#[ignore = "requires private266 inputs; caster-relative native summon geometry"]
fn demonology_native_summon_uses_open_fixture_geometry() {
    let mut run = prepare_literal(266, 1).unwrap();
    let dog = summon_native_dreadstalker(&mut run);
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Npc>(dog)
            .unwrap()
            .template_id,
        98035
    );
}

#[test]
#[ignore = "requires private266 inputs; actual Dreadstalker expiry funds Demonic Core"]
fn demonology_dreadstalker_demise_grants_native_core() {
    let mut run = prepare_literal(266, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(426115)
    );
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 264173));
    let dog = summon_native_dreadstalker(&mut run);
    for _ in 0..1000 {
        if run.app.world().get_entity(dog).is_err() {
            break;
        }
        run.app.update();
    }
    assert!(
        run.app.world().get_entity(dog).is_err(),
        "native summon must actually expire"
    );
    let stacks = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 264173)
        .map_or(0, |aura| aura.stacks);
    assert_eq!(
        stacks, 1,
        "native267102 effect1 is a100% Core grant on actual Dreadstalker demise"
    );
}

#[test]
#[ignore = "requires private266 inputs; native Aura Does Not Refresh bit"]
fn demonology_dominion_stacks_without_extending_native_expiry() {
    let mut run = prepare_literal(266, 1).unwrap();
    let read = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1276166)
            .map(|aura| (aura.expires_at, aura.stacks))
    };
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 1276166, Some(run.actor), 0);
    let (expiry, _) = read(&run).expect("native counter first grant");
    for _ in 0..20 {
        run.app.update();
    }
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 1276166, Some(run.actor), 0);
    let (refreshed, stacks) = read(&run).expect("native counter second grant");
    assert_eq!(stacks, 2, "native cumulative stacks must still advance");
    assert_eq!(
        refreshed, expiry,
        "native Attributes15 bit9 forbids extending expiry"
    );
}

#[test]
#[ignore = "requires private266 profile and native world DB"]
fn demonology_literal_fixture_executes_profile_instead_of_native_kit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(266, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_some(),
        "266 must hand control to the literal APL before native warmup"
    );
    assert!(crate::hunter_pet::summoned_pet(run.app.world(), run.actor).is_some());
    engage(&mut run);
    for _ in 0..400 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.targets, 20.0);
    let actions = combat["actions"].as_array().unwrap();
    assert!(
        actions
            .iter()
            .any(|row| row["spell_id"] == 686 && row["damage"].as_u64().unwrap() > 0),
        "literal precast/filler must actually damage the target"
    );
    assert!(
        actions
            .iter()
            .any(|row| row["spell_id"] == 1276452 && row["count"].as_u64().unwrap() > 0),
        "literal priority must cast selected Grimoire: Imp Lord"
    );
    assert!(
        !actions.iter().any(|row| row["spell_id"] == 702),
        "native follower Curse of Weakness is absent from the pinned APL"
    );
}
