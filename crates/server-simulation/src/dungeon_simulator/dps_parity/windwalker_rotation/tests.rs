use super::*;
use crate::simc_apl_native::FightStyle;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn fixture(world: &World) -> Encounter {
    Encounter::Fixture {
        started_at: crate::spell_cast::now(world),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    }
}

fn action(name: &str) -> Action {
    Action {
        source_row: None,
        name: name.into(),
        target: None,
        options: Default::default(),
    }
}

pub(super) fn cast(run: &mut super::super::Run, spell: u32) {
    use crate::spell_cast::CastFailure;
    for _ in 0..1200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.targets[0]),
            false,
        ) {
            Ok(()) => {
                run.app.update();
                return;
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::InProgress
                | CastFailure::NotEnoughPower(_),
            ) => run.app.update(),
            Err(reason) => panic!("native spell{spell} failed: {reason:?}"),
        }
    }
    panic!("native spell{spell} never became ready");
}

#[test]
#[ignore = "native shared parity fixture: Mage and Monk targets must fit the frontal pack"]
fn literal_native_frontal_pack_keeps_shared_fixture_targets_in_front() {
    let minimum_cosine = 30_f32.to_radians().cos();
    for spec in [63, 269] {
        let run = super::super::prepare_measurement(spec, 3).unwrap();
        let world = run.app.world();
        let actor = world
            .get::<shared::components::Position>(run.actor)
            .unwrap();
        let primary = world
            .get::<shared::components::Position>(run.targets[0])
            .unwrap();
        let forward = Vec2::new(primary.x - actor.x, primary.z - actor.z).normalize();
        for &target in &run.targets {
            let position = world.get::<shared::components::Position>(target).unwrap();
            let offset = Vec2::new(position.x - actor.x, position.z - actor.z);
            let cosine = offset.normalize().dot(forward);
            assert!(
                cosine >= minimum_cosine,
                "spec{spec} target{target:?} must fit the shared60-degree frontal pack: actor={actor:?} target={position:?} cosine={cosine}"
            );
        }
    }
}

#[test]
#[ignore = "native shared parity fixture: real FoF must reach all three configured opponents"]
fn literal_native_frontal_pack_fof_hits_all_three_fixture_opponents() {
    let _scope = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(269, 3).unwrap();
    let started = run.app.world().resource::<Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, started);
    super::super::engage(&mut run);
    for _ in 0..3 {
        cast(&mut run, 100780);
    }
    let first_event = run.app.world().resource::<ObservedCombatEvents>().0.len();
    cast(&mut run, 113656);
    for _ in 0..200 {
        run.app.update();
        if !crate::auras::has_aura(run.app.world(), run.actor, 113656) {
            break;
        }
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 113656),
        "the actual native FoF channel must finish"
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[first_event..];
    let recipients: std::collections::BTreeSet<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(117418)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .filter_map(|event| event.target)
        .collect();
    let expected: std::collections::BTreeSet<_> =
        run.targets.iter().map(|target| target.to_bits()).collect();
    eprintln!("FRONTAL_PACK_FOF recipients={recipients:?} expected={expected:?}");
    assert_eq!(
        recipients, expected,
        "native FoF must reach all three opponents without widening its authored cone"
    );
}

#[test]
#[ignore = "literal WW native RED: action-specific combo facts after actual TP and utility"]
fn literal_ww_native_combo_facts_follow_the_proposed_action_and_ignore_utility() {
    let mut run = super::super::prepare(269, 1).unwrap();
    let clock = fixture(run.app.world());
    let rotation = Rotation::load(run.app.world_mut(), run.actor, clock).unwrap();
    for (name, expected) in [
        ("cooldown.whirling_dragon.remains", 0.0),
        (
            "cooldown.invoke_power_infusion_0.duration",
            i64::MIN as f64 / 1000.0,
        ),
    ] {
        assert_eq!(
            rotation
                .value_for(
                    run.app.world_mut(),
                    run.actor,
                    &run.targets,
                    &action("tiger_palm"),
                    name
                )
                .unwrap(),
            expected
        );
    }
    assert!(
        rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("tiger_palm"),
                "cooldown.unknown_whirling_dragon.remains"
            )
            .is_err()
    );
    assert_eq!(
        rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("tiger_palm"),
                "combo_strike"
            )
            .unwrap(),
        1.0
    );
    cast(&mut run, 100780);
    assert_eq!(
        rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("tiger_palm"),
                "combo_strike"
            )
            .unwrap(),
        0.0
    );
    assert_eq!(
        rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("blackout_kick"),
                "combo_strike"
            )
            .unwrap(),
        1.0
    );
    cast(&mut run, 115546);
    assert_eq!(
        rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("tiger_palm"),
                "combo_strike"
            )
            .unwrap(),
        0.0
    );
}

fn log_zenith_priority_probe(run: &mut super::super::Run, rotation: &Rotation, frame: usize) {
    const ZENITH: u32 = 1249625;
    let values: serde_json::Map<String, Value> = [
        "fight_style.patchwerk",
        "cooldown.zenith.full_recharge_time",
        "cooldown.zenith.remains",
        "cooldown.rising_sun_kick.remains",
        "cooldown.fists_of_fury.remains",
        "talent.flurry_strikes",
        "talent.spiritual_focus",
        "trinket.1.has_use_buff",
        "trinket.2.has_use_buff",
        "trinket.1.cooldown.ready",
        "trinket.2.cooldown.ready",
        "trinket.1.cooldown.remains",
        "trinket.2.cooldown.remains",
        "chi",
        "energy",
        "buff.zenith.up",
    ]
    .into_iter()
    .map(|name| {
        let value = rotation.value_for(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            &action("zenith"),
            name,
        );
        (name.into(), json!(value))
    })
    .collect();
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(ZENITH)
        .cloned()
        .unwrap();
    let admission = match crate::spell_cast::read_native_cast_start(
        run.app.world_mut(),
        run.actor,
        &spell,
        run.actor,
    ) {
        Ok(_) => "ready".to_owned(),
        Err(reason) => format!("{reason:?}"),
    };
    let world = run.app.world();
    let known = world
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    let traits = world
        .get::<crate::class_progression::TraitLoadout>(run.actor)
        .unwrap();
    eprintln!(
        "WW_NATIVE_ZENITH_PROBE {}",
        json!({
            "frame": frame, "known": known.contains(ZENITH),
            "selected": traits.active_talent_spells().any(|spell| spell.spell_id == ZENITH),
            "resolved": crate::auras::overrides::resolve_spell(world, run.actor, ZENITH),
            "native_admission": admission, "facts": values,
        })
    );
}

#[test]
#[ignore = "native self-cast readiness under the pinned Zenith target_if source"]
fn literal_ww_native_zenith_target_if_preserves_enemy_context_and_self_aura() {
    const ZENITH: u32 = 1249625;
    let mut run = super::super::prepare_measurement(269, 1).unwrap();
    // target_if reads explicit fixture TTD/boss role; native setup does not install it.
    let started_at = run.app.world().resource::<Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, started_at);
    let mut rotation = (run.spec_driver.take().unwrap() as Box<dyn std::any::Any>).downcast::<Rotation>().unwrap();
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &rotation.equipment,
        bindings::execution_actions(),
        false,
    )
    .unwrap();
    assert!(
        prepared
            .execution(&action("zenith"), &rotation.equipment)
            .unwrap()
            .is_some(),
        "native default self cast must already be ready"
    );
    let xuen = facts::read_xuen(run.app.world_mut(), run.actor);
    let facts = WwFacts::new(
        run.app.world(),
        run.actor,
        &rotation.bindings,
        &rotation.equipment,
        rotation.encounter,
        &run.targets,
        xuen,
    );
    assert_eq!(
        facts.target_ids(&action("zenith")).unwrap(),
        vec![run.targets[0].to_bits()]
    );
    let (selected, execution) = rotation
        .select(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            "zenith",
            &prepared,
        )
        .unwrap()
        .expect("pinned Zenith target_if must select its actual enemy context");
    assert_eq!(selected.target, Some(run.targets[0].to_bits()));
    rotation
        .driver
        .execute(run.app.world_mut(), run.actor, selected, execution)
        .unwrap();
    for _ in 0..20 {
        run.app.update();
        if run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .is_some_and(|auras| auras.iter().any(|aura| aura.spell_id == ZENITH))
        {
            break;
        }
    }
    assert!(
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == ZENITH && aura.caster == run.actor),
        "enemy scoring context must still cast the actual Zenith aura on self"
    );
    assert!(
        !run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.targets[0])
            .is_some_and(|auras| auras
                .iter()
                .any(|aura| aura.spell_id == ZENITH && aura.caster == run.actor))
    );
}

fn run_literal(count: usize) {
    let _scope = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(269, count).unwrap();
    super::super::engage(&mut run);
    let mut rotation = (run.spec_driver.take().unwrap() as Box<dyn std::any::Any>).downcast::<Rotation>().unwrap();
    for frame in 0..600 {
        if frame % 20 == 0 {
            log_zenith_priority_probe(&mut run, &rotation, frame);
        }
        rotation
            .drive(run.app.world_mut(), run.actor, &run.targets)
            .unwrap();
        run.app.update();
    }
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
    assert!(
        casts(100780) > 0,
        "literal opener must naturally build Chi with native Tiger Palm"
    );
    assert!(
        casts(113656) > 0,
        "literal priority must reach actual FoF, not generic BOK starvation"
    );
    assert!(
        casts(107428) > 0,
        "actual native Rising Sun Kick must execute"
    );
    assert_eq!(
        casts(392983),
        0,
        "unselected Strike of the Windlord must not leak from the old generic kit"
    );
    if count == 3 {
        assert!(casts(101546) > 0, "literal multitarget SCK must execute");
        for target in &run.targets {
            assert!(
                events.iter().any(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(107270)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                }),
                "each real target must receive actual native SCK107270 damage"
            );
        }
    }
    assert!(
        run.app
            .world()
            .get::<crate::followers::Follower>(run.actor)
            .is_some(),
        "native follower identity stays intact"
    );
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(run.actor)
            .is_none(),
        "no synthetic measured player"
    );
}

#[test]
#[ignore = "literal WW native RED: 30s real ST casts under pinned source ordering"]
fn literal_ww_native_st_executes_its_real_selected_kit_without_generic_starvation() {
    run_literal(1);
}

#[test]
#[ignore = "literal WW native RED: 30s real three-target casts and recipients"]
fn literal_ww_native_three_target_executes_sck_and_damages_all_native_targets() {
    run_literal(3);
}

#[test]
#[ignore = "native FoF damage RED; exact literal fixture ordering and frozen inputs"]
fn literal_ww_native_fof_channel_releases_damage_on_the_measured_target() {
    let _scope = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(269, 1).unwrap();
    super::super::engage(&mut run);
    let mut rotation = (run.spec_driver.take().unwrap() as Box<dyn std::any::Any>).downcast::<Rotation>().unwrap();
    let mut observed_channel = false;
    for _ in 0..600 {
        rotation
            .drive(run.app.world_mut(), run.actor, &run.targets)
            .unwrap();
        let world = run.app.world();
        if !observed_channel
            && world
                .get::<shared::casting::CastState>(run.actor)
                .is_some_and(|cast| cast.spell_id == 113656)
        {
            observed_channel = true;
            let aura = world
                .get::<crate::auras::ActiveAuras>(run.actor)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 113656);
            eprintln!(
                "FOF_NATIVE_PROBE {}",
                json!({
                    "cast": format!("{:?}", world.get::<shared::casting::CastState>(run.actor)),
                    "position": format!("{:?}", world.get::<shared::components::Position>(run.actor)),
                    "rotation": format!("{:?}", world.get::<shared::components::Rotation>(run.actor)),
                    "target_position": format!("{:?}", world.get::<shared::components::Position>(run.targets[0])),
                    "aura_effects": aura.map(|aura| aura.effects.iter().map(|effect| json!({"index": effect.index,"aura": effect.aura_type,"period_ms": effect.period_ms,"trigger": effect.trigger_spell})).collect::<Vec<_>>())
                })
            );
        }
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(113656)
                && event.kind == CombatLogKind::CastSuccess),
        "literal fixture must actually cast FoF"
    );
    assert!(observed_channel, "native FoF must enter its actual channel");
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(117418)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "actual FoF113656 must deliver its native117418 periodic damage, not merely log a root cast"
    );
}

#[test]
#[ignore = "native RWK producer RED; real300s literal fixture with selected traits"]
fn literal_ww_native_combo_breaker_blackout_earns_rwk_and_delivers_its_child() {
    let report = super::super::measure(269, 1).unwrap();
    let actions = report["combat"]["actions"].as_array().unwrap();
    assert!(
        actions.iter().any(|action| action["spell_id"] == 100784
            && action["hits"].as_u64().is_some_and(|hits| hits > 0)),
        "actual Blackout Kick damage is required before testing its producer"
    );
    assert!(
        report["buff_uptime_pct"]["116768"]
            .as_f64()
            .is_some_and(|uptime| uptime > 0.0),
        "actual Tiger Palm must naturally earn Combo Breaker"
    );
    assert!(
        report["buff_uptime_pct"]["1250554"]
            .as_f64()
            .is_some_and(|uptime| uptime > 0.0),
        "native selected RWK must naturally earn its1250554 override from landed Combo-Breaker Blackout Kicks"
    );
    assert!(
        actions.iter().any(|action| action["spell_id"] == 467307
            && action["count"].as_u64().is_some_and(|count| count > 0)),
        "literal APL must admit the actual earned RWK override"
    );
    assert!(
        actions.iter().any(|action| action["spell_id"] == 468179
            && action["damage"].as_u64().is_some_and(|damage| damage > 0)),
        "actual RWK must deliver native468179 damage, not just a root cast"
    );
}

#[test]
#[ignore = "native selected WDP earns its DB2 aura from real overlapping RSK and FoF cooldowns"]
fn literal_ww_native_selected_wdp_earns_its_native_cooldown_window() {
    let _scope = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(269, 1).unwrap();
    let started_at = run.app.world().resource::<Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, started_at);
    super::super::engage(&mut run);
    let mut rotation = (run.spec_driver.take().unwrap() as Box<dyn std::any::Any>).downcast::<Rotation>().unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|spell| spell.spell_id == 152175)
    );
    let mut overlapping_native_cooldowns = false;
    let mut earned_duration = None;
    for _ in 0..600 {
        rotation
            .drive(run.app.world_mut(), run.actor, &run.targets)
            .unwrap();
        run.app.update();
        let rsk = rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("rising_sun_kick"),
                "cooldown.rising_sun_kick.remains",
            )
            .unwrap();
        let fof = rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("fists_of_fury"),
                "cooldown.fists_of_fury.remains",
            )
            .unwrap();
        overlapping_native_cooldowns |= rsk > 0.0 && fof > 0.0;
        if let Some(aura) = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 196742 && aura.caster == run.actor)
        {
            earned_duration.get_or_insert(aura.duration_ms);
        }
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    for spell in [107428, 113656] {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::CastSuccess),
            "actual native RSK/FoF source cast {spell} must precede the producer test"
        );
    }
    assert!(
        overlapping_native_cooldowns,
        "both actual native cooldowns must overlap"
    );
    assert!(
        earned_duration.is_some_and(|duration| duration > 1000),
        "selected owner must earn the approved cooldown-overlap plus hasted-grace window"
    );
    let base_duration = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(196742)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .duration
        .as_ref()
        .unwrap()
        .duration_ms;
    assert_eq!(
        base_duration, 1000,
        "the scripted override must not mutate DB2's base duration"
    );
}

#[test]
#[ignore = "native landed RSK cannot earn WDP while FoF remains unspent and ready"]
fn literal_ww_native_rsk_without_fof_cooldown_does_not_earn_wdp_enabler() {
    let _scope = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(269, 1).unwrap();
    let encounter = fixture(run.app.world());
    let rotation = Rotation::load(run.app.world_mut(), run.actor, encounter).unwrap();
    cast(&mut run, 100780);
    cast(&mut run, 100780);
    cast(&mut run, 107428);
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(185099)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "the native RSK child must actually land"
    );
    assert_eq!(
        rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("fists_of_fury"),
                "cooldown.fists_of_fury.remains"
            )
            .unwrap(),
        0.0
    );
    assert!(
        rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("rising_sun_kick"),
                "cooldown.rising_sun_kick.remains"
            )
            .unwrap()
            > 0.0
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 196742),
        "one actual cooldown is insufficient for the native WDP enabler"
    );
}

#[test]
#[ignore = "native unselected WDP owner has real overlapping cooldowns but cannot earn the enabler"]
fn literal_ww_native_unselected_wdp_owner_does_not_earn_the_enabler() {
    let _scope = crate::random::seed_scope(70448);
    let (profile, mut traits) = super::super::read_reference(269).unwrap();
    // Native node101207 is a choice:125014/392983 Windlord versus134452/152175 WDP.
    // Keep the same point spend so later native20-point gates remain valid.
    let choice = traits
        .iter_mut()
        .find(|entry| entry.entry_id == 134452)
        .unwrap();
    assert_eq!(choice.node_id, 101207);
    choice.entry_id = 125014;
    let identity = super::super::inputs::read_actor(&profile).unwrap();
    let mut run = super::super::prepare_literal_reference(profile, &traits, 1, identity).unwrap();
    run.spec_driver = Some(super::prepare_driver(&mut run).unwrap());
    let started_at = run.app.world().resource::<Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, started_at);
    super::super::engage(&mut run);
    let mut rotation = (run.spec_driver.take().unwrap() as Box<dyn std::any::Any>).downcast::<Rotation>().unwrap();
    assert!(
        !run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|spell| spell.spell_id == 152175)
    );
    let mut overlapping_native_cooldowns = false;
    for _ in 0..600 {
        rotation
            .drive(run.app.world_mut(), run.actor, &run.targets)
            .unwrap();
        run.app.update();
        let rsk = rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("rising_sun_kick"),
                "cooldown.rising_sun_kick.remains",
            )
            .unwrap();
        let fof = rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("fists_of_fury"),
                "cooldown.fists_of_fury.remains",
            )
            .unwrap();
        overlapping_native_cooldowns |= rsk > 0.0 && fof > 0.0;
        assert!(
            !crate::auras::has_aura(run.app.world(), run.actor, 196742),
            "unselected native WDP owner cannot earn the196742 enabler"
        );
    }
    assert!(
        overlapping_native_cooldowns,
        "negative control must reach the real cooldown boundary"
    );
}

#[test]
#[ignore = "native earned RWK reset and override consumption; no aura or power grants"]
fn literal_ww_native_earned_rwk_resets_rsk_and_consumes_its_override() {
    let _scope = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(269, 1).unwrap();
    let clock = fixture(run.app.world());
    let rotation = Rotation::load(run.app.world_mut(), run.actor, clock).unwrap();
    let mut earned = false;
    for _ in 0..64 {
        cast(&mut run, 100780);
        if !crate::auras::has_aura(run.app.world(), run.actor, 116768) {
            continue;
        }
        cast(&mut run, 107428);
        let before = rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action("rising_sun_kick"),
                "cooldown.rising_sun_kick.full_recharge_time",
            )
            .unwrap();
        assert!(
            before > 0.0,
            "actual RSK must spend its native shared charge first"
        );
        if !crate::auras::has_aura(run.app.world(), run.actor, 116768) {
            continue;
        }
        cast(&mut run, 100784);
        if crate::auras::has_aura(run.app.world(), run.actor, 1250554) {
            earned = true;
            assert_eq!(
                rotation
                    .value_for(
                        run.app.world_mut(),
                        run.actor,
                        &run.targets,
                        &action("rising_sun_kick"),
                        "cooldown.rising_sun_kick.full_recharge_time",
                    )
                    .unwrap(),
                0.0,
                "earned RWK must reset the actual native RSK charge"
            );
            break;
        }
    }
    assert!(
        earned,
        "selected owner must naturally earn1250554 from landed BOK"
    );
    let rwk = action("rushing_wind_kick");
    let mut dispatched = false;
    for _ in 0..1200 {
        let prepared = Prepared::read(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            &rotation.equipment,
            bindings::execution_actions(),
            false,
        )
        .unwrap();
        if let Some(execution) = prepared.execution(&rwk, &rotation.equipment).unwrap() {
            literal_rotation::execution::execute(run.app.world_mut(), run.actor, execution)
                .unwrap();
            run.app.update();
            dispatched = true;
            break;
        }
        run.app.update();
    }
    assert!(
        dispatched,
        "earned actual RWK must dispatch through native literal admission"
    );
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(468179)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
        "actual earned RWK must damage through native468179"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 1250554),
        "actual RWK use must consume its earned override rather than repeatedly replacing RSK"
    );
    assert_eq!(
        crate::auras::overrides::resolve_spell(run.app.world(), run.actor, 107428),
        107428
    );
}

#[test]
#[ignore = "native unselected RWK owner control with naturally earned Combo Breaker"]
fn literal_ww_native_unselected_rwk_owner_does_not_earn_the_override() {
    let _scope = crate::random::seed_scope(70448);
    let (profile, mut traits) = super::super::read_reference(269).unwrap();
    // Pinned native entry134543 is selected1250566; declare the unselected input
    // before native progression, never remove a live aura/grant or reset powers.
    let original = traits.len();
    traits.retain(|entry| entry.entry_id != 134543);
    assert_eq!(traits.len() + 1, original);
    let identity = super::super::inputs::read_actor(&profile).unwrap();
    let mut run = super::super::spawn_reference(profile, &traits, 1, identity).unwrap();
    let clock = fixture(run.app.world());
    literal_rotation::capture_fixture(run.app.world_mut(), run.actor, clock).unwrap();
    super::super::preparation::warm_up_reference(&mut run).unwrap();
    assert!(
        !run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|spell| spell.spell_id == 1250566)
    );
    let bok_hits = |run: &super::super::Run| {
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(100784)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count()
    };
    let mut eligible_hit = false;
    for _ in 0..64 {
        cast(&mut run, 100780);
        if !crate::auras::has_aura(run.app.world(), run.actor, 116768) {
            continue;
        }
        let before = bok_hits(&run);
        cast(&mut run, 100784);
        if bok_hits(&run) > before {
            eligible_hit = true;
            break;
        }
    }
    assert!(
        eligible_hit,
        "actual BOK must land after naturally earned Combo Breaker"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 1250554),
        "unselected native1250566 owner must not earn the RWK override"
    );
}

#[test]
#[ignore = "explicit native non-Fixture control; requires the prepared parity inputs"]
fn literal_ww_native_non_fixture_follower_keeps_its_native_kit() {
    let mut run = super::super::prepare(269, 1).unwrap();
    super::super::engage(&mut run);
    for _ in 0..80 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<super::super::literal_rotation::LiteralParityFixture>(run.actor)
            .is_none()
    );
    assert!(
        run.app
            .world()
            .get::<crate::followers::Follower>(run.actor)
            .is_some()
    );
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
        "non-Fixture follower still runs its independent native kit"
    );
}

#[test]
#[ignore = "literal WW native RED: dungeon clock unavailable instead of hidden rotation fallback"]
fn literal_ww_native_driver_rejects_dungeon_ttd_without_touching_the_native_kit() {
    let mut run = super::super::prepare(269, 1).unwrap();
    let result = Rotation::load(
        run.app.world_mut(),
        run.actor,
        Encounter::Dungeon { started_at: 0.0 },
    );
    assert!(result.err().unwrap().contains("DUNGEON_TTD_UNAVAILABLE"));
    super::super::engage(&mut run);
    for _ in 0..80 {
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
        "separate native kit still works; rejected literal loading cannot capture its controller"
    );
}
