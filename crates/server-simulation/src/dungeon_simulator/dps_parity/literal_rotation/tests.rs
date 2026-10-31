//! Real native casts: selected targets, request-time knowledge and AA independence.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use execution::NativeAction;
use shared::components::{Health, Player, Position};
use shared::protocol::CombatLogKind;
use std::time::Duration;

type Run = super::super::Run;

fn native_actor(
    identity: crate::followers::simulator::ParityActor,
    count: usize,
) -> (Run, Equipment) {
    let (profile, traits) = super::super::read_reference(269).unwrap();
    let mut run = super::super::prepare_reference(profile, &traits, count, identity).unwrap();
    super::super::engage(&mut run);
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    capture_fixture(run.app.world_mut(), run.actor, encounter).unwrap();
    let equipment = Equipment::load(
        run.app.world(),
        run.actor,
        &mut crate::simc_apl_native::Bindings::default(),
    )
    .unwrap();
    (run, equipment)
}

fn human(count: usize) -> (Run, Equipment) {
    // The gear/trait profile is Troll; this dispatch fixture explicitly creates
    // an actual Human. Profile metadata is not the native actor's identity.
    let (run, equipment) = native_actor(
        crate::followers::simulator::ParityActor::Player { race: 1 },
        count,
    );
    assert_eq!(run.app.world().get::<Player>(run.actor).unwrap().race, 1);
    assert!(
        run.app
            .world()
            .get::<crate::followers::Follower>(run.actor)
            .is_none()
    );
    assert!(
        run.app
            .world()
            .get::<shared::components::Npc>(run.actor)
            .is_none()
    );
    (run, equipment)
}

struct TargetFacts<'a> {
    world: &'a World,
    targets: &'a [Entity],
}
impl Facts for TargetFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name == "time" {
            return Ok(crate::spell_cast::now(self.world));
        }
        Err(format!("unknown test fact {name}"))
    }
}
impl ActionFacts for TargetFacts<'_> {
    fn target_ids(&self, _: &Action) -> Result<Vec<u64>, String> {
        Ok(self.targets.iter().map(|target| target.to_bits()).collect())
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name != "target.health.pct" {
            return self.value(name);
        }
        let target = match action.target {
            Some(bits) => Entity::try_from_bits(bits).ok_or("invalid native target")?,
            None => self.targets[0],
        };
        let health = self
            .world
            .get::<Health>(target)
            .ok_or("native target lacks Health")?;
        Ok(f64::from(health.current) * 100.0 / f64::from(health.max))
    }
}

fn prepare_palm(run: &mut Run, equipment: &Equipment) -> Prepared {
    Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        equipment,
        [NativeAction {
            name: "tiger_palm",
            spell: 100780,
            talent: None,
            self_target: false,
        }],
        false,
    )
    .unwrap()
}

fn select_palm(run: &mut Run, driver: &mut Driver, equipment: &Equipment) -> (Action, Execution) {
    for _ in 0..200 {
        let prepared = prepare_palm(run, equipment);
        let facts = TargetFacts {
            world: run.app.world(),
            targets: &run.targets,
        };
        if let Some(selected) = driver
            .select("default", &facts, &prepared, equipment)
            .unwrap()
        {
            return selected;
        }
        run.app.update();
    }
    panic!("native Tiger Palm never reached actual readiness");
}

fn palm_driver(targeted: bool) -> Driver {
    Driver::new(
        Program::parse(&[(
            "default",
            if targeted {
                "tiger_palm,target_if=min:target.health.pct"
            } else {
                "tiger_palm"
            },
        )])
        .unwrap(),
    )
}

#[test]
#[ignore = "actual native Player fixture; requires frozen parity inputs"]
fn literal_queues_nearly_ready_cooldown_before_lower_priority_cast() {
    let (mut run, equipment) = human(1);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        shared::components::PowerType::Chi,
        10.0,
    )
    .unwrap();
    execution::execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(113656, target, Some(target)),
    )
    .unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        shared::components::PowerType::Chi,
        10.0,
    )
    .unwrap();
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(113656)
        .unwrap();
    let remaining = world
        .get::<crate::spell_cast::SpellHistory>(actor)
        .unwrap()
        .remaining_cooldown_ms(spell, crate::spell_cast::history_now(world));
    assert!(remaining > 250);
    crate::spell_cast::scripts::reduce_cooldown(
        run.app.world_mut(),
        actor,
        113656,
        (remaining as f64 - 200.0) / 1000.0,
    );
    let actions = [
        NativeAction {
            name: "fists_of_fury",
            spell: 113656,
            talent: None,
            self_target: false,
        },
        NativeAction {
            name: "tiger_palm",
            spell: 100780,
            talent: None,
            self_target: false,
        },
    ];
    let mut driver = Driver::new(
        Program::parse(&[
            ("default", "fists_of_fury,if=target.health.pct>50"),
            ("default", "tiger_palm"),
        ])
        .unwrap(),
    );
    let initial = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let prepared = Prepared::read(
        run.app.world_mut(),
        actor,
        &run.targets,
        &equipment,
        actions,
        false,
    )
    .unwrap();
    let facts = TargetFacts {
        world: run.app.world(),
        targets: &run.targets,
    };
    let (action, execution) = driver
        .select("default", &facts, &prepared, &equipment)
        .unwrap()
        .unwrap();
    driver
        .execute(run.app.world_mut(), actor, action, execution)
        .unwrap();
    run.app
        .world_mut()
        .get_mut::<Health>(target)
        .unwrap()
        .current *= 0.25;
    for _ in 0..10 {
        run.app.update();
        let prepared = Prepared::read(
            run.app.world_mut(),
            actor,
            &run.targets,
            &equipment,
            actions,
            false,
        )
        .unwrap();
        let facts = TargetFacts {
            world: run.app.world(),
            targets: &run.targets,
        };
        if let Some((action, execution)) = driver
            .select("default", &facts, &prepared, &equipment)
            .unwrap()
        {
            driver
                .execute(run.app.world_mut(), actor, action, execution)
                .unwrap();
            break;
        }
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[initial..];
    let spells: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
        })
        .filter_map(|event| event.spell_id)
        .collect();
    assert_eq!(
        spells,
        vec![113656],
        "the queued Fists of Fury must execute after its cooldown without casting the lower-priority Tiger Palm"
    );
}

#[test]
#[ignore = "actual native Player fixture; requires frozen parity inputs"]
fn literal_queues_nearly_recharged_action_before_lower_priority_cast() {
    let (mut run, equipment) = human(1);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        shared::components::PowerType::Chi,
        10.0,
    )
    .unwrap();
    execution::execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(107428, target, Some(target)),
    )
    .unwrap();
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        shared::components::PowerType::Chi,
        10.0,
    )
    .unwrap();
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(107428)
        .unwrap()
        .clone();
    let mut queueable = false;
    for _ in 0..500 {
        run.app.update();
        if crate::spell_cast::read_native_cast_for_cooldown_queue(
            run.app.world_mut(),
            actor,
            &spell,
            target,
            false,
        )
        .is_ok()
        {
            queueable = true;
            break;
        }
    }
    assert!(
        queueable,
        "the spent Rising Sun Kick charge must enter the queue window"
    );
    let actions = [
        NativeAction {
            name: "rising_sun_kick",
            spell: 107428,
            talent: None,
            self_target: false,
        },
        NativeAction {
            name: "tiger_palm",
            spell: 100780,
            talent: None,
            self_target: false,
        },
    ];
    let mut driver = Driver::new(
        Program::parse(&[("default", "rising_sun_kick"), ("default", "tiger_palm")]).unwrap(),
    );
    let initial = run.app.world().resource::<ObservedCombatEvents>().0.len();
    for _ in 0..10 {
        let prepared = Prepared::read(
            run.app.world_mut(),
            actor,
            &run.targets,
            &equipment,
            actions,
            false,
        )
        .unwrap();
        let facts = TargetFacts {
            world: run.app.world(),
            targets: &run.targets,
        };
        if let Some((action, execution)) = driver
            .select("default", &facts, &prepared, &equipment)
            .unwrap()
        {
            driver
                .execute(run.app.world_mut(), actor, action, execution)
                .unwrap();
        }
        run.app.update();
        let events = &run.app.world().resource::<ObservedCombatEvents>().0[initial..];
        if events.iter().any(|event| {
            event.source == Some(actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
        }) {
            break;
        }
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[initial..];
    let spells: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
        })
        .filter_map(|event| event.spell_id)
        .collect();
    assert_eq!(
        spells,
        vec![107428],
        "the queued charge action must execute before Tiger Palm"
    );
}

fn make_secondary_low(run: &mut Run) -> Entity {
    let target = run.targets[1];
    let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
    health.current = health.max * 0.25;
    target
}

fn casts(world: &World, actor: Entity) -> Vec<Option<u64>> {
    world
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id == Some(100780)
        })
        .map(|event| event.target)
        .collect()
}

fn attack_state(run: &Run) -> (Entity, Duration, Duration) {
    let attack = run
        .app
        .world()
        .get::<crate::combat::AutoAttack>(run.actor)
        .unwrap();
    (attack.victim, attack.timer, attack.off_hand_timer)
}

#[test]
#[ignore = "actual native Player fixture; requires frozen parity inputs"]
fn literal_wait_defers_cast_until_duration_then_reevaluates_condition() {
    let (mut run, equipment) = human(1);
    let mut driver = Driver::new(
        Program::parse(&[
            ("default", "wait,sec=0.2,if=target.health.pct>50"),
            ("default", "tiger_palm"),
        ])
        .unwrap(),
    );
    let (action, execution) = select_palm(&mut run, &mut driver, &equipment);
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    assert!(
        casts(run.app.world(), run.actor).is_empty(),
        "wait must not cast Tiger Palm"
    );
    let target = run.targets[0];
    let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
    health.current = health.max * 0.25;
    let started = crate::spell_cast::now(run.app.world());
    while crate::spell_cast::now(run.app.world()) - started < 0.15 {
        run.app.update();
        let prepared = prepare_palm(&mut run, &equipment);
        let facts = TargetFacts {
            world: run.app.world(),
            targets: &run.targets,
        };
        assert!(
            driver
                .select("default", &facts, &prepared, &equipment)
                .unwrap()
                .is_none(),
            "a selected wait must finish even when its condition changes"
        );
    }
    for _ in 0..10 {
        run.app.update();
    }
    let (action, execution) = select_palm(&mut run, &mut driver, &equipment);
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    assert_eq!(
        casts(run.app.world(), run.actor).len(),
        1,
        "after wait, reevaluate the false condition and cast"
    );
}

#[test]
#[ignore = "actual native Player fixture; requires frozen parity inputs"]
fn literal_player_native_cast_and_request_time_spellbook_rejection() {
    let _scope = crate::random::seed_scope(70448);
    let (mut run, equipment) = human(1);
    let mut driver = palm_driver(false);
    let (action, execution) = select_palm(&mut run, &mut driver, &equipment);
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    for _ in 0..20 {
        run.app.update();
    }
    assert_eq!(casts(run.app.world(), run.actor).len(), 1);
    let (action, execution) = select_palm(&mut run, &mut driver, &equipment);
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::class_progression::KnownSpells::default());
    let error = driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .expect_err("Player request must revalidate knowledge after preparation");
    assert!(error.contains("NotKnown"), "{error}");
    assert_eq!(casts(run.app.world(), run.actor).len(), 1);
}

#[test]
#[ignore = "actual native Player fixture; requires frozen parity inputs"]
fn literal_player_selected_secondary_receives_cast_without_retargeting() {
    let _scope = crate::random::seed_scope(70448);
    let (mut run, equipment) = human(3);
    select_palm(&mut run, &mut palm_driver(false), &equipment);
    let target = make_secondary_low(&mut run);
    let primary = run
        .app
        .world()
        .get::<crate::networking::CombatTarget>(run.actor)
        .unwrap()
        .0;
    let attack = attack_state(&run);
    let mut driver = palm_driver(true);
    let (action, execution) = select_palm(&mut run, &mut driver, &equipment);
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    assert_eq!(
        casts(run.app.world(), run.actor),
        vec![Some(target.to_bits())]
    );
    assert_eq!(
        run.app
            .world()
            .get::<crate::networking::CombatTarget>(run.actor)
            .unwrap()
            .0,
        primary
    );
    assert_eq!(
        attack_state(&run),
        attack,
        "spell dispatch must not change AA state"
    );
}

#[test]
#[ignore = "actual native Player fixture; requires frozen parity inputs"]
fn literal_player_readiness_cannot_reuse_primary_admission_for_far_secondary() {
    let _scope = crate::random::seed_scope(70448);
    let (mut run, equipment) = human(3);
    select_palm(&mut run, &mut palm_driver(false), &equipment);
    let far = make_secondary_low(&mut run);
    run.app.world_mut().get_mut::<Position>(far).unwrap().x = 100.0;
    let prepared = prepare_palm(&mut run, &equipment);
    let facts = TargetFacts {
        world: run.app.world(),
        targets: &run.targets,
    };
    let result = palm_driver(true)
        .select("default", &facts, &prepared, &equipment)
        .unwrap();
    assert!(
        result.is_none(),
        "chosen out-of-range target must not borrow primary readiness"
    );
    assert!(casts(run.app.world(), run.actor).is_empty());
}

#[test]
#[ignore = "actual native harmful precast; requires frozen parity inputs"]
fn literal_player_precombat_resolves_the_native_selected_enemy() {
    let _scope = crate::random::seed_scope(70448);
    let (mut run, equipment) = human(1);
    select_palm(&mut run, &mut palm_driver(false), &equipment);
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &[],
        &equipment,
        [NativeAction {
            name: "tiger_palm",
            spell: 100780,
            talent: None,
            self_target: false,
        }],
        false,
    )
    .unwrap();
    let facts = TargetFacts {
        world: run.app.world(),
        targets: &run.targets,
    };
    let mut driver = Driver::new(Program::parse(&[("precombat", "tiger_palm")]).unwrap());
    let (action, execution) = driver
        .select("precombat", &facts, &prepared, &equipment)
        .unwrap()
        .expect("harmful precast must see the native selected enemy");
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    assert_eq!(
        casts(run.app.world(), run.actor),
        vec![Some(run.targets[0].to_bits())]
    );
}

#[test]
#[ignore = "actual native Player fixture; requires frozen parity inputs"]
fn literal_player_expired_selected_target_never_redirects_to_primary() {
    let _scope = crate::random::seed_scope(70448);
    let (mut run, equipment) = human(3);
    select_palm(&mut run, &mut palm_driver(false), &equipment);
    let target = make_secondary_low(&mut run);
    let mut driver = palm_driver(true);
    let (action, execution) = select_palm(&mut run, &mut driver, &equipment);
    crate::zones::unsummon(run.app.world_mut(), target);
    assert!(
        driver
            .execute(run.app.world_mut(), run.actor, action, execution)
            .is_err()
    );
    assert!(
        casts(run.app.world(), run.actor).is_empty(),
        "expired target cannot become primary cast"
    );
}

#[test]
#[ignore = "actual native Player fixture; requires frozen parity inputs"]
fn literal_player_retarget_selects_enemy_and_observes_line_cooldown() {
    let _scope = crate::random::seed_scope(70448);
    let (mut run, equipment) = human(3);
    let target = make_secondary_low(&mut run);
    let mut driver = Driver::new(
        Program::parse(&[(
            "default",
            "retarget,target_if=min:target.health.pct,line_cd=0.25",
        )])
        .unwrap(),
    );
    let select = |run: &mut Run, driver: &mut Driver| {
        let prepared = Prepared::read(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            &equipment,
            std::iter::empty::<NativeAction<'_>>(),
            false,
        )
        .unwrap();
        let facts = TargetFacts {
            world: run.app.world(),
            targets: &run.targets,
        };
        driver
            .select("default", &facts, &prepared, &equipment)
            .unwrap()
    };
    let (action, execution) =
        select(&mut run, &mut driver).expect("retarget selects the lowest health enemy");
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<crate::networking::CombatTarget>(run.actor)
            .unwrap()
            .0,
        Some(target)
    );
    assert!(
        select(&mut run, &mut driver).is_none(),
        "line cooldown prevents immediate reselection"
    );
    for _ in 0..10 {
        run.app.update();
    }
    assert!(
        select(&mut run, &mut driver).is_some(),
        "line cooldown expires"
    );
    let mut palm = palm_driver(false);
    let (action, execution) = select_palm(&mut run, &mut palm, &equipment);
    palm.execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    assert_eq!(
        casts(run.app.world(), run.actor),
        vec![Some(target.to_bits())]
    );
}

#[test]
#[ignore = "actual native Player fixture; requires frozen parity inputs"]
fn literal_player_retarget_auto_attack_changes_only_victim() {
    let _scope = crate::random::seed_scope(70448);
    let (mut run, equipment) = human(3);
    let target = make_secondary_low(&mut run);
    let primary = run
        .app
        .world()
        .get::<crate::networking::CombatTarget>(run.actor)
        .unwrap()
        .0;
    {
        let mut attack = run
            .app
            .world_mut()
            .get_mut::<crate::combat::AutoAttack>(run.actor)
            .unwrap();
        attack.timer = Duration::from_millis(1234);
        attack.off_hand_timer = Duration::from_millis(2345);
    }
    let before = attack_state(&run);
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        std::iter::empty::<NativeAction<'_>>(),
        false,
    )
    .unwrap();
    let facts = TargetFacts {
        world: run.app.world(),
        targets: &run.targets,
    };
    // SimC76f8118 player.cpp10824–10860 changes only MH/OH attack targets.
    let mut driver = Driver::new(
        Program::parse(&[(
            "default",
            "retarget_auto_attack,target_if=min:target.health.pct",
        )])
        .unwrap(),
    );
    let (action, execution) = driver
        .select("default", &facts, &prepared, &equipment)
        .unwrap()
        .expect("active AA can retarget to a different selected victim");
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    assert_eq!(attack_state(&run), (target, before.1, before.2));
    assert_eq!(
        run.app
            .world()
            .get::<crate::networking::CombatTarget>(run.actor)
            .unwrap()
            .0,
        primary
    );
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        std::iter::empty::<NativeAction<'_>>(),
        false,
    )
    .unwrap();
    let facts = TargetFacts {
        world: run.app.world(),
        targets: &run.targets,
    };
    let mut start = Driver::new(Program::parse(&[("default", "auto_attack")]).unwrap());
    assert!(
        start
            .select("default", &facts, &prepared, &equipment)
            .unwrap()
            .is_none(),
        "already-running AA must not be restarted back onto primary"
    );
}

#[test]
#[ignore = "actual native resource-query boundary; requires frozen parity inputs"]
fn literal_player_resource_query_preserves_actual_pools() {
    use crate::spell_cast::powers::PowerPools;
    use shared::components::PowerType;
    let _scope = crate::random::seed_scope(70448);
    let (mut run, equipment) = human(3);
    select_palm(&mut run, &mut palm_driver(false), &equipment);
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(100780)
        .unwrap();
    assert!(
        crate::spell_cast::applicable_costs(run.app.world(), run.actor, spell)
            .unwrap()
            .iter()
            .any(
                |(pool, cost, _)| *pool == crate::spell_cast::CostPool::Power(PowerType::Energy)
                    && *cost > 0.0
            ),
        "fixture TP must really require native Energy"
    );
    let far = run.targets[1];
    run.app.world_mut().get_mut::<Position>(far).unwrap().x = 100.0;
    for (full, target, expected) in [
        (false, run.targets[0], true),
        (true, run.targets[0], false),
        (false, far, false),
    ] {
        {
            let mut pools = run
                .app
                .world_mut()
                .get_mut::<PowerPools>(run.actor)
                .unwrap();
            let energy = pools.get_mut(PowerType::Energy).unwrap();
            energy.current = if full { energy.max } else { 0.0 };
        }
        let before = run
            .app
            .world()
            .get::<PowerPools>(run.actor)
            .unwrap()
            .clone();
        let prepared = prepare_palm(&mut run, &equipment);
        let action = Action {
            source_row: None,
            name: "tiger_palm".into(),
            target: Some(target.to_bits()),
            options: Default::default(),
        };
        for _ in 0..3 {
            assert_eq!(
                prepared.resource_limited(&action, &equipment, PowerType::Energy),
                Ok(expected),
                "zero Energy limits only the otherwise-native-ready target"
            );
        }
        assert_eq!(
            run.app.world().get::<PowerPools>(run.actor).unwrap(),
            &before,
            "neither precompute nor query may grant, spend or temporarily replace pools"
        );
    }
    assert!(casts(run.app.world(), run.actor).is_empty());
}

#[test]
#[ignore = "actual native selected-pool query; requires frozen parity inputs"]
fn literal_player_resource_query_does_not_fill_other_powers() {
    use crate::spell_cast::powers::PowerPools;
    use shared::components::PowerType;
    let _scope = crate::random::seed_scope(70448);
    let (mut run, equipment) = human(1);
    select_palm(&mut run, &mut palm_driver(false), &equipment);
    {
        let mut pools = run
            .app
            .world_mut()
            .get_mut::<PowerPools>(run.actor)
            .unwrap();
        pools.get_mut(PowerType::Energy).unwrap().current = 0.0;
        pools.get_mut(PowerType::Chi).unwrap().current = 0.0;
    }
    let before = run
        .app
        .world()
        .get::<PowerPools>(run.actor)
        .unwrap()
        .clone();
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(100784)
        .unwrap();
    assert!(
        crate::spell_cast::applicable_costs(run.app.world(), run.actor, spell)
            .unwrap()
            .iter()
            .any(
                |(pool, amount, _)| *pool == crate::spell_cast::CostPool::Power(PowerType::Chi)
                    && *amount > 0.0
            )
    );
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        [NativeAction {
            name: "blackout_kick",
            spell: 100784,
            talent: None,
            self_target: false,
        }],
        false,
    )
    .unwrap();
    let action = Action {
        source_row: None,
        name: "blackout_kick".into(),
        target: Some(run.targets[0].to_bits()),
        options: Default::default(),
    };
    // SimC10644–10818 changes only the selected pool, not every resource.
    assert_eq!(
        prepared.resource_limited(&action, &equipment, PowerType::Energy),
        Ok(false)
    );
    assert_eq!(
        prepared.resource_limited(&action, &equipment, PowerType::Chi),
        Ok(true)
    );
    assert_eq!(
        run.app.world().get::<PowerPools>(run.actor).unwrap(),
        &before
    );
    assert!(casts(run.app.world(), run.actor).is_empty());
}

#[test]
#[ignore = "actual native WW follower control; requires frozen parity inputs"]
fn literal_player_change_preserves_ww_creature_spellbook_policy() {
    let _scope = crate::random::seed_scope(70448);
    let (mut run, equipment) = native_actor(crate::followers::simulator::ParityActor::Follower, 1);
    let mut driver = palm_driver(false);
    let (action, execution) = select_palm(&mut run, &mut driver, &equipment);
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::class_progression::KnownSpells::default());
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    assert_eq!(casts(run.app.world(), run.actor).len(), 1);
}
