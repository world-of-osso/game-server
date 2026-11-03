//! Production follower APL and Command Demon against actual enemy casts.
use super::*;
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

fn production_warlock(spec: u32, selected_pet: Option<&str>) -> Run {
    production_warlock_targets(spec, selected_pet, 1)
}

fn production_warlock_targets(spec: u32, selected_pet: Option<&str>, targets: usize) -> Run {
    let (mut profile, traits) = read_reference(spec).unwrap();
    if let Some(pet) = selected_pet {
        profile
            .options
            .insert("warlock.default_pet".into(), serde_json::json!(pet));
    }
    let app = super::super::runtime::build_literal_follower_app();
    let mut run = spawn_reference_in(
        app,
        profile,
        &traits,
        targets,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    for _ in 0..600 {
        run.app.update();
        if crate::hunter_pet::summoned_pet(run.app.world(), run.actor).is_some() {
            return run;
        }
    }
    panic!("production precombat must summon its configured demon");
}

fn production_demonology() -> Run {
    production_warlock(266, None)
}

fn engage_follower(run: &mut Run) {
    engage(run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
}

fn prepare_enemy(run: &mut Run) {
    let enemy = run.targets[0];
    prepare_target(run, enemy);
}

fn prepare_target(run: &mut Run, enemy: Entity) {
    let world = run.app.world_mut();
    let actor = run.actor;
    let stats = world
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .clone();
    let pools = world
        .get::<crate::spell_cast::powers::PowerPools>(actor)
        .unwrap()
        .clone();
    let mana = *world.get::<shared::components::Mana>(actor).unwrap();
    world.entity_mut(enemy).insert((stats, pools, mana));
}

fn start_enemy_fireball(run: &mut Run) {
    prepare_enemy(run);
    let world = run.app.world_mut();
    let actor = run.actor;
    let enemy = run.targets[0];
    crate::spell_cast::creature_cast(world, enemy, 133, Some(actor), false)
        .expect("enemy begins a real Fireball cast");
    assert!(world.get::<CastState>(enemy).is_some());
}

#[test]
#[ignore = "requires canonical 266 inputs and production follower APL"]
fn production_felguard_interrupt_preserves_owner_cast() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = production_demonology();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Npc>(pet)
            .unwrap()
            .template_id,
        17252
    );
    engage_follower(&mut run);
    start_enemy_fireball(&mut run);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        686,
        Some(run.targets[0]),
        false,
    )
    .expect("owner begins Shadow Bolt");
    for _ in 0..25 {
        run.app.update();
        if run.app.world().get::<CastState>(run.targets[0]).is_none() {
            break;
        }
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(119914)
                && event.kind == CombatLogKind::CastSuccess
        }),
        "production APL must issue the native Axe Toss command"
    );
    assert!(
        events.iter().any(|event| {
            event.source == Some(pet.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(347008)
                && event.kind == CombatLogKind::Interrupt
        }),
        "learned Axe Toss must interrupt the real enemy cast: {events:?}"
    );
    assert_eq!(
        run.app
            .world()
            .get::<CastState>(run.actor)
            .map(|cast| cast.spell_id),
        Some(686),
        "pet command must leave the owner's Shadow Bolt active"
    );
}

#[test]
#[ignore = "requires canonical 266 inputs and production follower APL"]
fn production_felguard_preparation_precedes_owner_damage() {
    let run = production_demonology();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Npc>(pet)
            .unwrap()
            .template_id,
        17252
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        !events.iter().any(|event| {
            event.source == Some(run.actor.to_bits()) && event.kind == CombatLogKind::Damage
        }),
        "the configured Felguard must be prepared before combat damage"
    );
}

#[test]
#[ignore = "requires canonical 266 inputs and production follower APL"]
fn production_felguard_fights_its_companions_engaged_enemy() {
    let mut run = production_demonology();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage_follower(&mut run);
    for _ in 0..160 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(pet.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "the real Felguard must damage the companion's engaged enemy"
    );
}

#[test]
#[ignore = "requires canonical 266 inputs and production follower APL"]
fn production_felguard_interrupt_cooldown_preserves_damage_priority() {
    let mut run = production_demonology();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage_follower(&mut run);
    start_enemy_fireball(&mut run);
    for _ in 0..160 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let casts = events
        .iter()
        .filter(|event| {
            event.source == Some(pet.to_bits())
                && event.spell_id == Some(89766)
                && event.kind == CombatLogKind::CastSuccess
        })
        .count();
    assert_eq!(casts, 1, "the first enemy Fireball must consume Axe Toss");
    let start = events.len();
    start_enemy_fireball(&mut run);
    for _ in 0..20 {
        run.app.update();
    }
    assert!(
        run.app.world().get::<CastState>(run.targets[0]).is_some(),
        "Axe Toss on cooldown must not interrupt the second Fireball"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(!events[start..].iter().any(|event| {
        event.source == Some(pet.to_bits())
            && event.spell_id == Some(89766)
            && event.kind == CombatLogKind::CastSuccess
    }));
    assert!(
        run.app.world().get::<CastState>(run.actor).is_some()
            || events[start..].iter().any(|event| {
                event.source == Some(run.actor.to_bits()) && event.kind == CombatLogKind::Damage
            }),
        "an unavailable pet interrupt must not stop the owner's damage priority"
    );
}

#[test]
#[ignore = "requires canonical 266 inputs and production follower APL"]
fn production_felguard_interrupts_nonprimary_engaged_caster() {
    let mut run = production_warlock_targets(266, None, 2);
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage_follower(&mut run);
    let caster = run.targets[1];
    prepare_target(&mut run, caster);
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, 133, Some(run.actor), false)
        .expect("second enemy begins Fireball");
    assert!(run.app.world().get::<CastState>(run.targets[0]).is_none());
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        686,
        Some(run.targets[0]),
        false,
    )
    .expect("owner begins Shadow Bolt on primary");
    for _ in 0..25 {
        run.app.update();
        if run.app.world().get::<CastState>(caster).is_none() {
            break;
        }
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(pet.to_bits())
                && event.target == Some(caster.to_bits())
                && event.spell_id == Some(347008)
                && event.kind == CombatLogKind::Interrupt),
        "the secondary engaged caster must be interrupted: {events:?}"
    );
    assert_eq!(
        run.app
            .world()
            .get::<CastState>(run.actor)
            .map(|cast| cast.spell_id),
        Some(686)
    );
}

#[test]
#[ignore = "requires canonical 266 inputs and production follower APL"]
fn production_felguard_does_not_pull_an_idle_caster() {
    let mut run = production_warlock_targets(266, None, 2);
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage_follower(&mut run);
    let outsider = run.targets[1];
    crate::combat_refs::end_all_combat(run.app.world_mut(), outsider);
    run.app
        .world_mut()
        .entity_mut(outsider)
        .insert(CastState::normal(133, run.actor.to_bits(), 30.0, true));
    assert!(
        crate::combat_refs::active_opponents(run.app.world(), outsider)
            .next()
            .is_none()
    );
    for _ in 0..20 {
        run.app.update();
    }
    assert!(
        crate::combat_refs::active_opponents(run.app.world(), outsider)
            .next()
            .is_none(),
        "pet and follower must not pull the idle outsider"
    );
    assert!(
        !run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(pet.to_bits())
                    && event.target == Some(outsider.to_bits())
                    && event.spell_id == Some(89766)
            })
    );
}

#[test]
#[ignore = "requires canonical 266 inputs and production follower APL"]
fn production_felguard_does_not_interrupt_without_learned_axe_toss() {
    let mut run = production_demonology();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    let mut charm = run
        .app
        .world_mut()
        .get_mut::<crate::pet_control::PetCharm>(pet)
        .unwrap();
    let slot = charm
        .spell_slot(89766)
        .expect("the real Felguard knows Axe Toss");
    charm.action_bar[slot] = 0;
    assert!(crate::pet_control::command_demon_pet(run.app.world(), run.actor, 119914).is_none());
    engage_follower(&mut run);
    start_enemy_fireball(&mut run);
    for _ in 0..20 {
        run.app.update();
    }
    assert!(run.app.world().get::<CastState>(run.targets[0]).is_some());
    assert!(
        !run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(119914)
                    && event.kind == CombatLogKind::CastSuccess
            })
    );
}

#[test]
#[ignore = "requires canonical 266 inputs and production follower APL"]
fn production_felguard_does_not_interrupt_immune_enemy() {
    let mut run = production_demonology();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage_follower(&mut run);
    start_enemy_fireball(&mut run);
    let enemy = run.targets[0];
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(104773)
        .unwrap();
    let effect_mask = crate::auras::aura_effect_mask(spell);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: enemy,
            target: enemy,
            spell_id: 104773,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(!crate::spell_cast::can_interrupt_cast(
        run.app.world(),
        pet,
        enemy
    ));
    for _ in 0..20 {
        run.app.update();
    }
    assert!(run.app.world().get::<CastState>(enemy).is_some());
    assert!(
        !run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(pet.to_bits())
                    && event.spell_id == Some(89766)
                    && event.kind == CombatLogKind::CastSuccess
            })
    );
}

#[test]
#[ignore = "requires canonical 265/266/267 inputs and production follower APL"]
fn production_felhunter_interrupts_for_each_warlock_spec() {
    for spec in [265, 266, 267] {
        let mut run = production_warlock(spec, Some("felhunter"));
        let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
        assert_eq!(
            run.app
                .world()
                .get::<shared::components::Npc>(pet)
                .unwrap()
                .template_id,
            417
        );
        engage_follower(&mut run);
        start_enemy_fireball(&mut run);
        assert_eq!(
            crate::pet_control::command_demon_pet(run.app.world(), run.actor, 119910),
            Some((pet, 19647)),
            "spec {spec}: pet spellbook"
        );
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            686,
            Some(run.targets[0]),
            false,
        )
        .expect("owner begins a real Shadow Bolt");
        for _ in 0..25 {
            run.app.update();
            if run.app.world().get::<CastState>(run.targets[0]).is_none() {
                break;
            }
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events.iter().any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(119910)
                    && event.kind == CombatLogKind::CastSuccess
            }),
            "spec {spec} must issue the native Spell Lock command; events={events:?}; pet position={:?}; actor position={:?}; enemy position={:?}; enemy cast={:?}",
            run.app.world().get::<shared::components::Position>(pet),
            run.app
                .world()
                .get::<shared::components::Position>(run.actor),
            run.app
                .world()
                .get::<shared::components::Position>(run.targets[0]),
            run.app.world().get::<CastState>(run.targets[0])
        );
        assert!(
            events.iter().any(|event| {
                event.source == Some(pet.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(19647)
                    && event.kind == CombatLogKind::Interrupt
            }),
            "spec {spec} must interrupt with its learned Felhunter spell"
        );
        assert_eq!(
            run.app
                .world()
                .get::<CastState>(run.actor)
                .map(|cast| cast.spell_id),
            Some(686)
        );
    }
}

#[test]
#[ignore = "requires canonical 266 inputs and production follower APL"]
fn production_felguard_orders_wait_for_owner_combat() {
    let mut run = production_demonology();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage(&mut run);
    prepare_enemy(&mut run);
    let player = fixture_player(&run);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.targets[0],
        133,
        Some(player),
        false,
    )
    .expect("enemy begins a real Fireball against the player, not the follower");
    assert!(
        run.app
            .world()
            .get::<crate::followers::Follower>(run.actor)
            .is_some()
    );
    assert!(
        crate::combat_refs::active_opponents(run.app.world(), run.actor)
            .next()
            .is_none()
    );
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(CastState::normal(686, run.targets[0].to_bits(), 30.0, true));
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    server_follower_apl::demonology_rotation::command_minions(
        run.app.world_mut(),
        run.actor,
        &run.targets,
    );
    server_follower_apl::drive_followers(run.app.world_mut());
    assert_eq!(
        run.app
            .world()
            .get::<crate::networking::CombatTarget>(pet)
            .and_then(|target| target.0),
        None
    );
    assert!(
        crate::combat_refs::active_opponents(run.app.world(), run.actor)
            .next()
            .is_none(),
        "actor entered combat; events={:?}",
        &run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[start..]
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        !events[start..].iter().any(|event| {
            event.source == Some(pet.to_bits())
                && event.spell_id == Some(89766)
                && event.kind == CombatLogKind::CastSuccess
        }),
        "automated pet commands must wait for the follower's own combat reference"
    );
    assert!(run.app.world().get::<CastState>(run.targets[0]).is_some());
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    server_follower_apl::demonology_rotation::command_minions(
        run.app.world_mut(),
        run.actor,
        &run.targets,
    );
    assert_eq!(
        run.app
            .world()
            .get::<crate::networking::CombatTarget>(pet)
            .and_then(|target| target.0),
        Some(run.targets[0])
    );
}

#[test]
#[ignore = "requires canonical 266 inputs and production follower APL"]
fn production_stunned_felguard_preserves_owner_damage_priority() {
    let mut run = production_demonology();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage_follower(&mut run);
    prepare_enemy(&mut run);
    let enemy_position = *run
        .app
        .world()
        .get::<shared::components::Position>(run.targets[0])
        .unwrap();
    run.app
        .world_mut()
        .entity_mut(pet)
        .insert(shared::components::Position {
            x: enemy_position.x - 2.0,
            ..enemy_position
        });
    crate::spell_cast::creature_cast(run.app.world_mut(), run.targets[0], 853, Some(pet), false)
        .expect("enemy casts real Hammer of Justice on the pet");
    assert!(
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(pet)
            .unwrap()
            .is_stunned()
    );
    start_enemy_fireball(&mut run);
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..20 {
        run.app.update();
    }
    assert!(run.app.world().get::<CastState>(run.targets[0]).is_some());
    // Allow the owner's current global cooldown and damage cast to finish while the stun lasts.
    for _ in 0..60 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(pet)
            .unwrap()
            .is_stunned()
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(!events[start..].iter().any(|event| {
        event.source == Some(pet.to_bits())
            && event.spell_id == Some(89766)
            && event.kind == CombatLogKind::CastSuccess
    }));
    assert!(
        events[start..].iter().any(|event| {
            event.source == Some(run.actor.to_bits()) && event.kind == CombatLogKind::Damage
        }),
        "the stunned pet must not prevent the owner's damage priority: {events:?}"
    );
}
