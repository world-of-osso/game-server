//! Shared APL facts against actual native Marksmanship metadata.
use super::*;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use server::simc_apl::{Action, ActionFacts, Facts};

fn player() -> Run {
    let (profile, traits) = read_reference(254).unwrap();
    prepare_literal_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 1 },
    )
    .unwrap()
}

fn clock(world: &World) -> Encounter {
    Encounter::Fixture {
        started_at: crate::spell_cast::now(world),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    }
}

#[test]
#[ignore = "requires private spec254 profile and native data"]
fn native_apl_literal_player_engagement_does_not_start_unsourced_melee() {
    let mut run = player();
    engage(&mut run);
    run.app.update();
    let combat = report::combat(run.app.world(), &run.targets, 1.0);
    assert_eq!(
        combat["total_damage"], 0,
        "literal source owns the first attack, not fixture engagement"
    );
}

#[test]
#[ignore = "requires private spec254 profile and native data"]
fn native_apl_literal_retarget_enumerates_real_native_candidates() {
    let run = player();
    let bindings = Bindings::default();
    let facts = NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        clock(run.app.world()),
        1,
    )
    .with_targets(Some(run.targets[0]), &run.targets);
    let action = Action {
        name: "retarget".into(),
        target: None,
        options: Default::default(),
    };
    assert_eq!(
        facts.target_ids(&action).unwrap(),
        vec![run.targets[0].to_bits()]
    );
}

#[test]
#[ignore = "requires private spec254 profile and native data"]
fn native_apl_target_health_reads_selected_candidate_current_health() {
    let run = player();
    let bindings = Bindings::default();
    let facts = NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        clock(run.app.world()),
        1,
    )
    .with_targets(Some(run.targets[0]), &run.targets);
    let action = Action {
        name: "retarget".into(),
        target: Some(run.targets[0].to_bits()),
        options: Default::default(),
    };
    let health = run
        .app
        .world()
        .get::<shared::components::Health>(run.targets[0])
        .unwrap();
    assert_eq!(
        facts.value_for(&action, "target.health").unwrap(),
        f64::from(health.current)
    );
}

#[test]
#[ignore = "requires private spec254 profile and native data"]
fn native_apl_patchwerk_unconfigured_pull_event_has_no_remaining_time() {
    let run = player();
    let bindings = Bindings::default();
    let facts = NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        clock(run.app.world()),
        1,
    );
    assert_eq!(facts.value("raid_event.pull.exists").unwrap(), 0.0);
    assert_eq!(facts.value("raid_event.pull.remains").unwrap(), 0.0);
    assert_eq!(
        facts.value("raid_event.pull.in").unwrap(),
        i64::MAX as f64 / 1000.0
    );
}

#[test]
#[ignore = "requires private spec254 profile and native data"]
fn native_apl_target_debuff_react_tracks_actual_owned_aura_acquisition() {
    use crate::simc_apl_native::reaction::{ReactionSettings, configure};
    let mut run = player();
    let target = run.targets[0];
    let bindings = Bindings {
        buffs: [("spotters_mark".into(), 466872)].into(),
        ..Default::default()
    };
    configure(
        run.app.world_mut(),
        target,
        ReactionSettings {
            mean: 0.25,
            stddev: 0.0,
            offset: 0.0,
            nu: 0.0,
            maximum: 0.25,
        },
        &[466872],
    )
    .unwrap();
    let action = Action {
        name: "aimed_shot".into(),
        target: Some(target.to_bits()),
        options: Default::default(),
    };
    let read = |run: &Run| {
        NativeFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            clock(run.app.world()),
            1,
        )
        .value_for(&action, "debuff.spotters_mark.react")
        .unwrap()
    };
    assert_eq!(read(&run), 0.0);
    let effect_mask = crate::auras::aura_effect_mask(
        run.app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(466872)
            .unwrap(),
    );
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.actor,
            target,
            spell_id: 466872,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert_eq!(
        read(&run),
        0.0,
        "new target proc is not immediately reacted"
    );
    for _ in 0..6 {
        run.app.update();
    }
    assert_eq!(
        read(&run),
        1.0,
        "actual acquired target stack becomes reacted"
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 466872,
    );
    assert_eq!(read(&run), 0.0);
}

#[test]
#[ignore = "requires private spec254 profile and native data"]
fn native_apl_hero_tree_uses_selected_subtree_not_allocated_inactive_nodes() {
    let run = player();
    let b = Bindings::default();
    let f = NativeFacts::new(run.app.world(), run.actor, &b, clock(run.app.world()), 1);
    assert_eq!(f.value("hero_tree.sentinel").unwrap(), 1.0);
    assert_eq!(f.value("hero_tree.dark_ranger").unwrap(), 0.0);
    assert!(f.value("hero_tree.misspelled_tree").is_err());
}

#[test]
#[ignore = "requires private spec254 profile and native data"]
fn native_apl_buff_max_stack_exists_without_an_active_buff() {
    let run = player();
    let b = Bindings {
        buffs: [("bullseye".into(), 204090)].into(),
        ..Default::default()
    };
    let f = NativeFacts::new(run.app.world(), run.actor, &b, clock(run.app.world()), 1);
    assert_eq!(f.value("buff.bullseye.stack").unwrap(), 0.0);
    assert_eq!(f.value("buff.bullseye.max_stack").unwrap(), 30.0);
}

#[test]
#[ignore = "requires private spec254 profile and native data"]
fn native_apl_cast_time_uses_current_native_action_modifiers() {
    let run = player();
    let b = Bindings {
        actions: [("aimed_shot".into(), 19434), ("arcane_shot".into(), 185358)].into(),
        ..Default::default()
    };
    let world = run.app.world();
    let f = NativeFacts::new(world, run.actor, &b, clock(world), 1);
    let aimed = Action {
        name: "aimed_shot".into(),
        target: None,
        options: Default::default(),
    };
    let store = world.resource::<crate::spell_info::SpellStore>();
    let spell = store.get(19434).unwrap();
    let native_cast = f64::from(crate::spell_cast::cast_time_ms(world, run.actor, spell)) / 1000.0;
    assert!(native_cast > 0.0);
    assert_eq!(f.value_for(&aimed, "cast_time").unwrap(), native_cast);
    assert_eq!(f.value("action.aimed_shot.cast_time").unwrap(), native_cast);
}

#[test]
#[ignore = "requires private spec254 profile and native data"]
fn native_apl_action_cost_uses_current_native_power_modifiers() {
    let run = player();
    let b = Bindings {
        actions: [("aimed_shot".into(), 19434)].into(),
        ..Default::default()
    };
    let world = run.app.world();
    let f = NativeFacts::new(world, run.actor, &b, clock(world), 1);
    let aimed = Action {
        name: "aimed_shot".into(),
        target: None,
        options: Default::default(),
    };
    let store = world.resource::<crate::spell_info::SpellStore>();
    let spell = store.get(19434).unwrap();
    let native_cost: f32 = crate::spell_cast::applicable_costs(world, run.actor, spell)
        .unwrap()
        .iter()
        .map(|(_, amount, _)| *amount)
        .sum();
    assert!(native_cost > 0.0);
    assert_eq!(f.value_for(&aimed, "cost").unwrap(), f64::from(native_cost));
    assert_eq!(
        f.value("action.aimed_shot.cost").unwrap(),
        f64::from(native_cost)
    );
}

#[test]
#[ignore = "requires private spec254 profile and native data"]
fn native_apl_auto_shot_starts_and_does_not_restart_weapon_attack() {
    use super::literal_rotation::{
        equipment::Equipment,
        execution::{Prepared, execute},
    };
    let mut run = player();
    let mut b = Bindings::default();
    let equipment = Equipment::load(run.app.world_mut(), run.actor, &mut b).unwrap();
    let action = Action {
        name: "auto_shot".into(),
        target: None,
        options: Default::default(),
    };
    let ready = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        [],
        false,
    )
    .unwrap();
    let selected = ready
        .execution(&action, &equipment)
        .unwrap()
        .expect("native ranged start");
    execute(run.app.world_mut(), run.actor, selected).unwrap();
    run.app.update();
    let combat = report::combat(run.app.world(), &run.targets, 1.0);
    let actions = combat["actions"].as_array().unwrap();
    assert!(
        actions
            .iter()
            .any(|action| action["spell_id"] == 75 && action["count"].as_u64().unwrap() > 0),
        "native Auto Shot must actually cast"
    );
    assert!(
        !actions
            .iter()
            .any(|action| action["spell_id"].is_null() && action["damage"].as_u64().unwrap() > 0),
        "ranged start must not start melee swings"
    );
    let ready = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        [],
        false,
    )
    .unwrap();
    assert!(ready.execution(&action, &equipment).unwrap().is_none());
}
