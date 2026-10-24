use super::super::literal_rotation::{self, Driver, equipment::Equipment, execution::Prepared};
use super::{bindings, facts::EleFacts, source};
use crate::simc_apl_native::{Bindings, Encounter, FightStyle};
use bevy::prelude::*;
use server::simc_apl::{ActionFacts, Facts, Program};

fn fixture(count: usize) -> (super::super::Run, Bindings, Equipment, Encounter) {
    let mut run = super::super::prepare(262, count).unwrap();
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter).unwrap();
    super::super::engage_fixture_opponents(&mut run);
    let mut bindings = bindings::build();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    wait_for_native_lightning_bolt(&mut run);
    (run, bindings, equipment, encounter)
}

fn apply(world: &mut World, caster: Entity, target: Entity, spell: u32) {
    crate::auras::apply_aura(
        world,
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn wait_for_native_lightning_bolt(run: &mut super::super::Run) {
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(188196)
        .unwrap()
        .clone();
    for _ in 0..120 {
        match crate::spell_cast::read_native_cast_start(
            run.app.world_mut(),
            run.actor,
            &spell,
            run.targets[0],
        ) {
            Ok(_) => return,
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("native LB prerequisite rejected: {reason:?}"),
        }
    }
    panic!("native LB prerequisite did not become ready");
}

fn advance_with_native_hits(run: &mut super::super::Run, seconds: f64) {
    let until = crate::spell_cast::now(run.app.world()) + seconds;
    // npc_training_dummy evades and strips foreign auras after its last
    // attacker leaves combat. Real native hits retain combat, not fake timers.
    for _ in 0..(seconds * 20.0).ceil() as usize + 2 {
        if crate::spell_cast::now(run.app.world()) >= until {
            return;
        }
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            188196,
            Some(run.targets[0]),
            false,
        ) {
            Ok(())
            | Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::InProgress,
            ) => {}
            Err(reason) => panic!("native maintenance hit rejected: {reason:?}"),
        }
        run.app.update();
    }
    panic!("native fixture clock did not reach {until}");
}

fn lightning_bolt_eligibility(run: &mut super::super::Run, target: Entity) -> String {
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    let book = [403, 188196, 452201].map(|spell| (spell, known.contains(spell)));
    let resolved = crate::auras::overrides::resolve_spell(run.app.world(), run.actor, 188196);
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(188196)
        .unwrap()
        .clone();
    let refusal =
        crate::spell_cast::read_native_cast_start(run.app.world_mut(), run.actor, &spell, target)
            .err();
    format!(
        "book={book:?}, resolved={resolved}, refusal={refusal:?}, now={}",
        crate::spell_cast::now(run.app.world())
    )
}

#[test]
#[ignore = "opt-in native Elemental profile: requires private DPS_PARITY_WORLD_DB"]
fn literal_ele_native_owned_target_facts_select_and_damage_the_foreign_dot_target() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let (mut run, bindings, equipment, encounter) = fixture(3);
    let other = run
        .app
        .world()
        .get::<crate::followers::Follower>(run.actor)
        .unwrap()
        .owner;
    apply(run.app.world_mut(), run.actor, run.targets[0], 188389);
    apply(run.app.world_mut(), other, run.targets[1], 188389);
    let mut driver = Driver::new(
        Program::parse(&[(
            "default",
            "lightning_bolt,target_if=min:dot.flame_shock.remains",
        )])
        .unwrap(),
    );
    let target = run.targets[1];
    let eligibility = lightning_bolt_eligibility(&mut run, target);
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        bindings::execution_actions().filter(|action| action.name == "lightning_bolt"),
        false,
    )
    .unwrap();
    let facts = EleFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        &equipment,
        encounter,
        &run.targets,
    );
    assert_eq!(facts.value("active_dot.flame_shock").unwrap(), 1.0);
    let selected = driver
        .select("default", &facts, &prepared, &equipment)
        .unwrap();
    assert!(
        selected.is_some(),
        "actual native LB eligibility: {eligibility}"
    );
    let (action, execution) = selected.unwrap();
    assert_eq!(action.target, Some(run.targets[1].to_bits()));
    assert_eq!(
        facts.value_for(&action, "dot.flame_shock.remains").unwrap(),
        0.0
    );
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    for _ in 0..80 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let root_hits = |target: Entity| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(188196)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count()
    };
    assert!(root_hits(run.targets[1]) > 0);
    assert_eq!(root_hits(run.targets[0]), 0);

    apply(run.app.world_mut(), run.actor, run.targets[0], 197209);
    apply(run.app.world_mut(), other, run.targets[1], 197209);
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        bindings::execution_actions().filter(|action| action.name == "lightning_bolt"),
        false,
    )
    .unwrap();
    let facts = EleFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        &equipment,
        encounter,
        &run.targets,
    );
    assert_eq!(facts.value("lightning_rod").unwrap(), 1.0);
    let mut driver = Driver::new(
        Program::parse(&[(
            "default",
            "lightning_bolt,target_if=min:debuff.lightning_rod.remains",
        )])
        .unwrap(),
    );
    let (action, _) = driver
        .select("default", &facts, &prepared, &equipment)
        .unwrap()
        .unwrap();
    assert_eq!(action.target, Some(run.targets[1].to_bits()));
    assert_eq!(
        facts
            .value_for(&action, "debuff.lightning_rod.remains")
            .unwrap(),
        0.0
    );
    assert!(facts.value("dot.flame_shock.typo").is_err());
    assert!(facts.value("debuff.lightning_rod.typo").is_err());
    let invalid = server::simc_apl::Action {
        name: "lightning_bolt".into(),
        target: Some(Entity::PLACEHOLDER.to_bits()),
        options: Default::default(),
    };
    assert!(
        facts
            .value_for(&invalid, "dot.flame_shock.remains")
            .is_err()
    );
    let unknown = Program::parse(&[(
        "default",
        "lightning_bolt,if=0&talent.unknown_elemental_talent",
    )])
    .unwrap();
    assert!(
        unknown
            .select("default", &facts, &mut unknown.state(), |_| Ok(true))
            .is_err()
    );
    source::load().expect("exact82-row Elemental program must load without literal rewrites");
}

#[test]
#[ignore = "opt-in native Elemental profile: requires private DPS_PARITY_WORLD_DB"]
fn literal_ele_native_refreshability_and_counts_follow_real_aura_time() {
    let (mut run, bindings, equipment, encounter) = fixture(1);
    apply(run.app.world_mut(), run.actor, run.targets[0], 188389);
    let applied = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 188389 && aura.caster == run.actor)
        .unwrap();
    let initial = (
        crate::spell_cast::now(run.app.world()),
        applied.duration_ms,
        applied.expires_at,
    );
    let maximum =
        crate::auras::maximum_duration_ms(run.app.world(), run.actor, run.targets[0], 188389)
            .unwrap();
    let facts = EleFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        &equipment,
        encounter,
        &run.targets,
    );
    assert_eq!(facts.value("active_dot.flame_shock").unwrap(), 1.0);
    assert_eq!(facts.value("dot.flame_shock.refreshable").unwrap(), 0.0);
    let ticks = (f64::from(maximum) * 0.7 / 50.0).ceil() as usize + 2;
    advance_with_native_hits(&mut run, ticks as f64 / 20.0);
    assert!(
        run.app
            .world()
            .get::<shared::components::CombatStatus>(run.targets[0])
            .is_some_and(|status| status.0),
        "real native maintenance hits must retain dummy combat, not bypass its evade script"
    );
    let facts = EleFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        &equipment,
        encounter,
        &run.targets,
    );
    assert!(
        facts.value("dot.flame_shock.remains").unwrap() > 0.0,
        "native FS initial(now,duration,expiry)={initial:?}, maximum={maximum}, ticks={ticks}, now={}, caster_present={}",
        crate::spell_cast::now(run.app.world()),
        run.app.world().get_entity(run.actor).is_ok()
    );
    assert_eq!(facts.value("dot.flame_shock.refreshable").unwrap(), 1.0);
    advance_with_native_hits(&mut run, f64::from(maximum) / 1000.0 + 0.1);
    let facts = EleFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        &equipment,
        encounter,
        &run.targets,
    );
    assert_eq!(facts.value("active_dot.flame_shock").unwrap(), 0.0);
    assert_eq!(facts.value("dot.flame_shock.remains").unwrap(), 0.0);
    assert_eq!(facts.value("dot.flame_shock.refreshable").unwrap(), 1.0);
}

#[test]
#[ignore = "opt-in native Elemental precombat qualification pending shared lifecycle"]
fn literal_ele_native_precombat_casts_once_and_finishes_before_the_fight_clock() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let mut run = super::prepare_measurement(1).unwrap();
    let now = crate::spell_cast::now(run.app.world());
    assert!(
        now > 0.2,
        "real precombat must advance native cast/GCD time"
    );
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none()
    );
    assert!(
        !run.app
            .world()
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .unwrap()
            .has_active_gcd(crate::spell_cast::history_now(run.app.world()))
    );
    let stormkeeper = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 191634 && aura.caster == run.actor)
        .unwrap();
    assert_eq!(stormkeeper.charges, 2);
    let report = run.elemental_rotation.as_ref().unwrap().report();
    assert_eq!(report["started_at"].as_f64(), Some(now));
    assert!(report["native_precombat_snapshot"].is_object());
    assert!(
        !run.elemental_rotation
            .as_mut()
            .unwrap()
            .precombat_step(run.app.world_mut(), run.actor, &run.targets)
            .unwrap()
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let casts = |spell| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::CastSuccess
            })
            .count()
    };
    assert_eq!(casts(318038), 1);
    assert_eq!(casts(192106), 1);
    assert_eq!(casts(191634), 1);
    assert_eq!(
        casts(58875),
        0,
        "native legacy utility must not precede literal precombat"
    );
}

#[test]
#[ignore = "opt-in native Elemental non-Fixture qualification pending shared lifecycle"]
fn literal_ele_native_rejects_dungeon_and_preserves_real_kit_combat() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let mut run = super::super::prepare(262, 1).unwrap();
    let encounter = Encounter::Dungeon {
        started_at: crate::spell_cast::now(run.app.world()),
    };
    let error = super::Rotation::load(run.app.world_mut(), run.actor, encounter)
        .err()
        .unwrap();
    assert!(error.contains("DUNGEON_TTD_UNAVAILABLE"));
    super::super::engage(&mut run);
    for _ in 0..400 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
        "rejected dungeon load must leave the ordinary native kit dealing damage"
    );
}
