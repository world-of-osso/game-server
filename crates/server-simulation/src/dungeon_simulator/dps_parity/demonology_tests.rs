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

#[test]
#[ignore = "requires private266 inputs; actual Tyrant cooldown expected-remains facts"]
fn demonology_tyrant_expected_cooldown_uses_native_elapsed_time() {
    use server::simc_apl::Facts;
    let (mut run, _) = prepare_antoran_tyrant();
    for _ in 0..40 {
        run.app.update();
    }
    let world = run.app.world();
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("summon_demonic_tyrant".into(), 265187)].into(),
        ..Default::default()
    };
    let facts = crate::simc_apl_native::NativeFacts::new(
        world,
        run.actor,
        &bindings,
        crate::simc_apl_native::Encounter::Fixture {
            started_at: crate::spell_cast::now(world),
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        },
        1,
    );
    let read = |property: &str| {
        facts
            .value(&format!("cooldown.summon_demonic_tyrant.{property}"))
            .unwrap()
    };
    let duration = read("duration");
    let remains = read("remains");
    let started = world
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap()
        .last_successful_cast_at(265187)
        .unwrap();
    let elapsed = crate::spell_cast::now(world) - started;
    assert!(remains > 0.0 && remains < duration);
    let expected = remains * elapsed / (duration - remains);
    assert!((read("remains_expected") - expected).abs() < 1e-8);
    assert_eq!(read("remains_guess"), read("remains_expected"));
    assert_eq!(
        read("remains"),
        remains,
        "fact reads cannot advance native timers"
    );
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
#[ignore = "requires private266 inputs; inherited follower minion attack permission"]
fn demonology_follower_minion_can_attack_native_neutral_dummy() {
    let (run, tyrant) = prepare_antoran_tyrant();
    let target = run.targets[0];
    let world = run.app.world();
    assert!(world.get::<shared::components::Player>(run.actor).is_none());
    assert!(crate::spell_cast::targets::relation_matches(
        world,
        run.actor,
        target,
        crate::spell_cast::targets::Relation::Enemy,
    ));
    assert!(
        crate::spell_cast::targets::relation_matches(
            world,
            tyrant,
            target,
            crate::spell_cast::targets::Relation::Enemy,
        ),
        "a real follower minion must inherit its controller's neutral-target attack permission"
    );
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

#[test]
#[ignore = "requires private266 inputs; native guardian critical chance"]
fn demonology_guardian_inherits_owner_critical_chance() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let (mut run, tyrant) = prepare_antoran_tyrant();
    let expected = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct;
    assert!(expected > 0.0);
    run.app.world_mut().insert_resource(FactorCapture::new(
        tyrant,
        run.targets[0],
        1264093,
        "burning_cleave",
        1,
    ));
    for _ in 0..100 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let result = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "result" && event["spell_id"] == 1264093)
        .expect("actual Burning Cleave must resolve damage");
    let chance = result["factors"]["crit"].as_f64().unwrap() * 100.0;
    // SimC76f8118 pet.cpp217–220: guardians inherit owner critical chance.
    assert!(
        (chance - f64::from(expected)).abs() < 0.01,
        "native guardian crit{chance} must inherit owner crit{expected}"
    );
}

#[test]
#[ignore = "requires private266 inputs; native guardian cast haste"]
fn demonology_guardian_fel_firebolt_inherits_owner_cast_haste() {
    let mut run = prepare_literal(266, 1).unwrap();
    let imp = summon_native_guardian(&mut run, 104317);
    let world = run.app.world();
    let haste = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .haste_pct;
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(104318)
        .unwrap();
    let base_ms = spell.misc.as_ref().unwrap().cast_time.unwrap().base_ms;
    assert!(haste > 0.0 && base_ms > 0);
    let expected_ms = (base_ms as f32 / (1.0 + haste / 100.0)).round();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        imp,
        104318,
        Some(run.targets[0]),
        false,
    )
    .expect("native Fel Firebolt must start an ordinary cast");
    let cast = run
        .app
        .world()
        .get::<shared::casting::CastState>(imp)
        .unwrap();
    // SimC76f8118 pet.cpp222–226: guardians inherit owner spell cast speed.
    assert!(
        (cast.duration * 1000.0 - expected_ms).abs() < 1.0,
        "actual guardian cast{}ms must inherit owner haste{haste}: expected{expected_ms}ms",
        cast.duration * 1000.0
    );
}

#[test]
#[ignore = "requires private266 inputs; native owner passive modifies Imp Energy cost"]
fn demonology_guardian_owner_passive_reduces_fel_firebolt_cost() {
    let mut run = prepare_literal(266, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 334591));
    let imp = summon_native_guardian(&mut run, 104317);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        imp,
        104318,
        Some(run.targets[0]),
        false,
    )
    .expect("native Fel Firebolt must cast with its real Energy pool");
    for _ in 0..100 {
        run.app.update();
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(imp)
            .is_none()
        {
            break;
        }
    }
    let pool = run
        .app
        .world()
        .get::<crate::spell_cast::powers::PowerPools>(imp)
        .unwrap()
        .get(shared::components::PowerType::Energy)
        .unwrap();
    // SimC76f8118 sc_warlock_init.cpp34 + native334591 effect0:
    // native20 Energy discounted20% consumes16 from the actual100 pool.
    assert_eq!(
        pool.current, 84.0,
        "selected native owner passive must modify the Imp's cost"
    );
}

#[test]
#[ignore = "requires private266 inputs; native Imp-erator owner passive critical chance"]
fn demonology_guardian_owner_passive_increases_fel_firebolt_crit() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let mut run = prepare_literal(266, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 416230));
    let expected = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct
        + 15.0;
    let imp = summon_native_guardian(&mut run, 104317);
    run.app.world_mut().insert_resource(FactorCapture::new(
        imp,
        run.targets[0],
        104318,
        "fel_firebolt",
        1,
    ));
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        imp,
        104318,
        Some(run.targets[0]),
        false,
    )
    .expect("native Fel Firebolt must cast before sampling its critical chance");
    for _ in 0..100 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let event = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "result" && event["spell_id"] == 104318)
        .expect("actual Fel Firebolt must damage the native dummy");
    let chance = event["factors"]["crit"].as_f64().unwrap() * 100.0;
    // Pinned Fel Firebolt action-effects parse native416230 effect0's15pp;
    // this is additive to the separately qualified inherited owner crit.
    assert!(
        (chance - f64::from(expected)).abs() < 0.01,
        "native Imp-erator must add15pp: observed{chance}, expected{expected}"
    );
}

#[test]
#[ignore = "requires private266 inputs; actual Implosion retirement and native explosion"]
fn demonology_implosion_consumes_actual_imps_and_deals_native_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(266, 1).unwrap();
    for _ in 0..6 {
        summon_native_guardian(&mut run, 104317);
    }
    let imps: Vec<_> = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .filter(|(_, summon)| summon.owner == run.actor && summon.spell_id == 104317)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(
        imps.len(),
        6,
        "native summon roots must create actual Imp actors"
    );
    engage(&mut run);
    // SimC76f8118 sc_warlock_actions.cpp2980–3170 launches actual Imps,
    // emits native196278 and dismisses the participating actor after impact.
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        196277,
        Some(run.targets[0]),
        false,
    )
    .expect("selected native Implosion must be admitted with real owned Imps");
    for _ in 0..100 {
        run.app.update();
    }
    assert!(
        imps.iter()
            .any(|imp| run.app.world().get_entity(*imp).is_err()),
        "actual selected Wild Imps must retire after their Implosion impacts"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(196278)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
        }),
        "actual Implosion must deal native explosion damage to its selected enemy"
    );
}

#[test]
#[ignore = "requires private266 inputs; actual Imp Lord native attack dispatch"]
fn demonology_imp_lord_executes_real_greater_felbolts() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(266, 1).unwrap();
    engage(&mut run);
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        1276452,
        Some(run.targets[0]),
        false,
    )
    .expect("selected ordinary Grimoire must summon native Imp Lord");
    preparation::finish_pending_setup(&mut run).unwrap();
    let lord = run
        .app
        .world_mut()
        .query::<(
            Entity,
            &crate::spell_cast::summon::SummonedCreature,
            &shared::components::Npc,
        )>()
        .iter(run.app.world())
        .find(|(_, summon, npc)| summon.owner == run.actor && npc.template_id == 258584)
        .map(|(unit, _, _)| unit)
        .expect("ordinary Grimoire must create actual258584");
    for _ in 0..200 {
        run.app.update();
    }
    let hits = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(lord.to_bits())
                && event.spell_id == Some(1277116)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    // SimC76f8118 pets.cpp1672–1705: real Imp Lord repeats Greater Felbolt
    // from200 initial Energy, paying native40 while retaining the125 gate.
    assert!(
        hits >= 2,
        "actual Imp Lord must launch real Greater Felbolts: hits{hits}"
    );
}

fn summon_native_guardian(run: &mut Run, root: u32) -> Entity {
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, root, Some(run.actor), true)
        .expect("native triggered guardian summon must be admitted");
    for _ in 0..100 {
        let dog = run
            .app
            .world_mut()
            .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
            .iter(run.app.world())
            .find(|(_, summon)| summon.owner == run.actor && summon.spell_id == root)
            .map(|(unit, _)| unit);
        if let Some(dog) = dog {
            return dog;
        }
        run.app.update();
    }
    panic!("native guardian root{root} must spawn within five seconds");
}

fn summon_native_dreadstalker(run: &mut Run) -> Entity {
    summon_native_guardian(run, 193331)
}

#[test]
#[ignore = "requires private266 inputs; actual Wild Imp Energy regeneration"]
fn demonology_wild_imp_spent_energy_does_not_regenerate() {
    let mut run = prepare_literal(266, 1).unwrap();
    let imp = summon_native_guardian(&mut run, 104317);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        imp,
        104318,
        Some(run.targets[0]),
        false,
    )
    .expect("actual Wild Imp must cast native Fel Firebolt");
    for _ in 0..100 {
        run.app.update();
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(imp)
            .is_none()
        {
            break;
        }
    }
    let energy = |run: &Run| {
        run.app
            .world()
            .get::<crate::spell_cast::powers::PowerPools>(imp)
            .unwrap()
            .get(shared::components::PowerType::Energy)
            .unwrap()
            .current
    };
    let spent = energy(&run);
    assert!(
        spent < 100.0,
        "actual Fel Firebolt must spend native Energy"
    );
    for _ in 0..40 {
        run.app.update();
    }
    // SimC76f8118 sc_warlock_pets.cpp701/875–882: Wild Imp Energy
    // regeneration is disabled; waiting cannot fund another bolt.
    assert_eq!(
        energy(&run),
        spent,
        "Wild Imp must not regenerate spent Energy"
    );
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
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor)
        .expect("the actual configured Felguard must exist before pull");
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
        actions.iter().any(|row| {
            matches!(row["spell_id"].as_u64(), Some(104318 | 1264093 | 1277116))
                && row["actor"] != run.actor.to_bits()
                && row["damage"].as_u64().unwrap() > 0
        }),
        "the literal controller must let actual guardians attack its selected enemy"
    );
    assert!(
        actions
            .iter()
            .any(|row| row["actor"] == pet.to_bits() && row["damage"].as_u64().unwrap() > 0),
        "the literal controller must command its real Felguard, not only summon it"
    );
    assert!(
        !actions.iter().any(|row| row["spell_id"] == 702),
        "native follower Curse of Weakness is absent from the pinned APL"
    );
}
