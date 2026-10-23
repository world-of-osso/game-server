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
    run.app.world_mut().get_mut::<Position>(far).unwrap().0[0] = 100.0;
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
