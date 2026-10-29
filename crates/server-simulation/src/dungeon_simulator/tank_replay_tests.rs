use super::healer_intake::Intake;
use std::path::Path;

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn training_of_niuzao_tracks_live_stagger_levels() {
    use crate::auras::{self, ActiveAuras, RemoveReason};
    use crate::stats::CharacterStats;
    use shared::components::Health;

    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(268, 1).unwrap();
    let tank = run.actor;
    let source = run.targets[0];
    for _ in 0..4 {
        run.app.update();
    }
    let baseline = run
        .app
        .world()
        .get::<CharacterStats>(tank)
        .unwrap()
        .mastery_points;
    let per_level =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), tank, 383714, 0);
    crate::spell_cast::effects::replay_simc_dummy_strike(
        run.app.world_mut(),
        source,
        tank,
        100_000.0,
        false,
        false,
    );
    for _ in 0..4 {
        run.app.update();
    }
    let world = run.app.world();
    let pool = auras::stagger::pool_size(world, tank);
    let max = world.get::<Health>(tank).unwrap().max;
    assert!(
        pool > 0.0 && pool < max * 0.2,
        "real hit creates light Stagger: {pool}/{max}"
    );
    assert!(
        (world.get::<CharacterStats>(tank).unwrap().mastery_points - baseline - per_level).abs()
            < 0.01
    );
    let light = *world.get::<CharacterStats>(tank).unwrap();
    let stacks = world
        .get::<ActiveAuras>(tank)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 195630)
        .expect("the real incoming hit grants Elusive Brawler")
        .stacks;
    let light_dodges = (0..4000)
        .filter(|_| {
            matches!(
                crate::combat::roll_simc_dummy_outcome(
                    run.app.world(),
                    source,
                    tank,
                    false,
                    0.0,
                    0.0
                )
                .unwrap()
                .0,
                crate::melee::MeleeOutcome::Dodge
            )
        })
        .count();
    for (fraction, level) in [(0.4_f32, 2.0_f32), (0.8, 3.0)] {
        let world = run.app.world_mut();
        let pool = auras::stagger::pool_size(world, tank);
        let aura = world
            .get::<ActiveAuras>(tank)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 124255)
            .unwrap();
        let instance = aura.instance_id;
        let current = aura.effects[0].amount;
        assert!(auras::set_effect_amount(
            world,
            tank,
            instance,
            0,
            current * (max * fraction) / pool
        ));
        for _ in 0..4 {
            run.app.update();
        }
        let mastery = run
            .app
            .world()
            .get::<CharacterStats>(tank)
            .unwrap()
            .mastery_points;
        assert!(
            (mastery - baseline - level * per_level).abs() < 0.01,
            "fraction {fraction}, mastery {mastery}, baseline {baseline}"
        );
    }
    let heavy = *run.app.world().get::<CharacterStats>(tank).unwrap();
    let gained_dodge = heavy.defense.dodge - light.defense.dodge;
    let expected_dodge = (heavy.mastery_pct - light.mastery_pct) * f32::from(stacks);
    assert!(
        (gained_dodge - expected_dodge).abs() < 0.01,
        "held Elusive stacks must use live mastery: gained {gained_dodge}, expected {expected_dodge}"
    );
    let heavy_dodges = (0..4000)
        .filter(|_| {
            matches!(
                crate::combat::roll_simc_dummy_outcome(
                    run.app.world(),
                    source,
                    tank,
                    false,
                    0.0,
                    0.0
                )
                .unwrap()
                .0,
                crate::melee::MeleeOutcome::Dodge
            )
        })
        .count();
    assert!(
        heavy_dodges > light_dodges + 150,
        "actual swing rolls must reflect increased mastery: light={light_dodges}, heavy={heavy_dodges}"
    );
    auras::remove_auras(run.app.world_mut(), tank, RemoveReason::Cancelled, |a| {
        a.spell_id == 124255
    });
    for _ in 0..4 {
        run.app.update();
    }
    assert!(
        (run.app
            .world()
            .get::<CharacterStats>(tank)
            .unwrap()
            .mastery_points
            - baseline)
            .abs()
            < 0.01
    );
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn simc_raid_buffs_update_native_tank_stats() {
    let mut run = super::dps_parity::prepare_registered_player(268, 1).unwrap();
    let actor = run.actor;
    let before = *run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap();
    let buffs = [
        ("mark_of_the_wild", 1126),
        ("power_word_fortitude", 21562),
        ("battle_shout", 6673),
        ("skyfury", 462854),
    ]
    .into_iter()
    .map(|(name, spell_id)| crate::simc_tank_timeline::RaidBuff {
        name: name.into(),
        spell_id,
    })
    .collect::<Vec<_>>();
    super::tank_replay::apply_simc_raid_buffs(&mut run, &buffs).unwrap();
    let after = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap();
    assert_eq!(after.primary.agility, before.primary.agility);
    assert!((after.primary.stamina - before.primary.stamina * 1.05).abs() < 2.0);
    assert!((after.versatility_pct - before.versatility_pct - 3.0).abs() < 0.01);
    assert!((after.mastery_points - before.mastery_points - 2.0).abs() < 0.01);
    assert!(
        (after.attack_power - 3783.0).abs() < 3.0,
        "Brewmaster AP {}",
        after.attack_power
    );
    for buff in buffs {
        assert!(crate::auras::has_aura(
            run.app.world(),
            actor,
            buff.spell_id
        ));
    }
}

#[test]
#[ignore = "reads the pinned MID2 Protection profile"]
fn tank_replay_rejects_profiles_without_level_90_mid2_inputs() {
    let (mut profile, _) = super::dps_parity::read_reference(66).unwrap();
    super::dps_parity::require_mid2_profile(&profile).unwrap();
    profile.level = "89".into();
    assert!(super::dps_parity::require_mid2_profile(&profile).is_err());
    profile.level = "90".into();
    profile
        .options
        .insert("reference_tier".into(), serde_json::json!("MID1"));
    assert!(super::dps_parity::require_mid2_profile(&profile).is_err());
}

#[test]
#[ignore = "requires canonical Blood profile and world database"]
fn simc_dummy_strike_triggers_taken_melee_ability_proc() {
    use crate::auras::RemoveReason;
    use shared::components::Health;
    let mut run = super::dps_parity::prepare_registered_player(250, 1).unwrap();
    let target = run.actor;
    let source = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), target, 326801));
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        RemoveReason::Cancelled,
        |aura| matches!(aura.spell_id, 77535 | 207203 | 326808 | 326809),
    );
    let max = run.app.world().get::<Health>(target).unwrap().max;
    crate::spell_cast::creature_cast(run.app.world_mut(), target, 433895, Some(source), false)
        .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), target, 326808),
        "self-targeted Vampiric Strike effects must not trigger Sanguination"
    );
    run.app
        .world_mut()
        .get_mut::<Health>(target)
        .unwrap()
        .current = max * 0.30;
    crate::spell_cast::effects::replay_simc_dummy_strike(
        run.app.world_mut(),
        source,
        target,
        max * 0.01,
        false,
        false,
    );
    run.app.update();
    assert!(
        !crate::auras::has_aura(run.app.world(), target, 326808),
        "SimC Sanguination does not proc below its threshold"
    );
    run.app
        .world_mut()
        .get_mut::<Health>(target)
        .unwrap()
        .current = max * 0.80;
    crate::spell_cast::effects::replay_simc_dummy_strike(
        run.app.world_mut(),
        source,
        target,
        max * 0.01,
        false,
        false,
    );
    run.app.update();
    assert!(
        crate::auras::has_aura(run.app.world(), target, 326808),
        "landed melee ability above the threshold must proc Rune of Sanguination"
    );
}

#[test]
#[ignore = "uses the native recipient, auto-only proc and SimC special damage"]
fn simc_dummy_strike_does_not_trigger_auto_attack_procs() {
    use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
    let mut run = super::dps_parity::prepare_registered_player(250, 1).unwrap();
    let target = run.actor;
    let source = run.targets[0];
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        RemoveReason::Cancelled,
        |aura| matches!(aura.spell_id, 195181 | 207203 | 77535),
    );
    for _ in 0..3 {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: target,
                target,
                spell_id: 195181,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    let stacks = |run: &super::dps_parity::Run| {
        run.app
            .world()
            .get::<ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 195181)
            .map_or(0, |aura| aura.stacks)
    };
    assert_eq!(stacks(&run), 3);
    crate::spell_cast::effects::replay_simc_dummy_strike(
        run.app.world_mut(),
        source,
        target,
        10_000.0,
        false,
        false,
    );
    run.app.update();
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(source.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0)
    );
    assert_eq!(
        stacks(&run),
        3,
        "special attack must not consume Bone Shield"
    );
    let hit = super::healer_intake::Hit {
        tick: 1,
        source_guid: 0,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 10_000.0,
        level_scaling_factor: 1.0,
        kind: super::healer_intake::HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    crate::combat::replay_healer_hit(run.app.world_mut(), source, target, &hit).unwrap();
    run.app.update();
    assert_eq!(
        stacks(&run),
        2,
        "landed auto-attack must consume one Bone Shield stack"
    );
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn brewmaster_elusive_brawler_is_consumed_by_dodge_not_miss() {
    use crate::auras::{self, AuraApplication, AuraSource};
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(268, 1).unwrap();
    let tank = run.actor;
    let source = run.targets[0];
    assert!(
        !run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(tank)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 454484)
    );
    for _ in 0..3 {
        auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: tank,
                target: tank,
                spell_id: 195630,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    assert!(auras::has_aura(run.app.world(), tank, 195630));
    crate::combat::replay_healer_hit(run.app.world_mut(), source, tank, &avoided_hit("Miss"))
        .unwrap();
    run.app.update();
    assert!(auras::has_aura(run.app.world(), tank, 195630));
    crate::combat::replay_healer_hit(run.app.world_mut(), source, tank, &avoided_hit("Dodge"))
        .unwrap();
    run.app.update();
    assert!(!auras::has_aura(run.app.world(), tank, 195630));
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn simc_dummy_bleed_application_is_a_zero_damage_physical_hit() {
    use crate::auras::{
        self, RemoveReason,
        procs::{AuraEvent, PROC_TAKE_HARMFUL_SPELL},
    };
    use crate::spell_cast::script_registry::{SpellScriptHook, SpellScriptRegistry};
    use bevy::prelude::*;
    #[derive(Resource)]
    struct SeenHit {
        source: Entity,
        target: Entity,
        count: u32,
    }
    fn observe(world: &mut World, event: &AuraEvent) {
        let mut seen = world.resource_mut::<SeenHit>();
        if event.actor == seen.source
            && event.target == Some(seen.target)
            && event.spell_id == Some(0)
            && !event.periodic
            && event.damage == 0.0
            && event.target_mask[0] & PROC_TAKE_HARMFUL_SPELL != 0
        {
            seen.count += 1;
        }
    }
    static HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AuraEvent(observe)];
    static MODULES: &[&[SpellScriptHook]] = &[HOOKS];
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(268, 1).unwrap();
    let tank = run.actor;
    let source = run.targets[0];
    assert!(auras::has_aura(run.app.world(), tank, 117906));
    auras::remove_auras(run.app.world_mut(), tank, RemoveReason::Cancelled, |aura| {
        aura.spell_id == 195630
    });
    let before = run
        .app
        .world()
        .get::<shared::components::Health>(tank)
        .unwrap()
        .current;
    crate::spell_cast::effects::replay_simc_dummy_bleed_application(
        run.app.world_mut(),
        source,
        tank,
    );
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Health>(tank)
            .unwrap()
            .current,
        before
    );
    assert!(
        auras::has_aura(run.app.world(), tank, 195630),
        "physical application hit grants Elusive Brawler"
    );
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(source.to_bits())
                    && event.target == Some(tank.to_bits())
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && !event.periodic
                    && event.amount == 0
            })
    );
    run.app.insert_resource(SeenHit {
        source,
        target: tank,
        count: 0,
    });
    run.app.insert_resource(SpellScriptRegistry::new(MODULES));
    run.app.update();
    assert_eq!(
        run.app.world().resource::<SeenHit>().count,
        1,
        "one direct victim spell-hit event"
    );
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn critical_blackout_kick_grants_elusive_footwork_stack() {
    use crate::auras::{self, ActiveAuras, RemoveReason};
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(268, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let tank = run.actor;
    let target = run.targets[0];
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(tank)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 387046)
    );
    auras::remove_auras(run.app.world_mut(), tank, RemoveReason::Cancelled, |aura| {
        aura.spell_id == 195630
    });
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(tank)
        .unwrap()
        .crit_pct = 100.0;
    crate::spell_cast::creature_cast(run.app.world_mut(), tank, 205523, Some(target), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.spell_id == Some(205523)
                && event.crit
                && event.target == Some(target.to_bits()))
    );
    let stacks = run
        .app
        .world()
        .get::<ActiveAuras>(tank)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 195630)
        .map_or(0, |aura| aura.stacks);
    assert_eq!(
        stacks, 2,
        "one base mastery stack and one selected Elusive Footwork stack"
    );
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn elusive_brawler_only_procs_from_blackout_and_incoming_physical_hits() {
    use crate::auras::{self, RemoveReason};
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(268, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let tank = run.actor;
    let source = run.targets[0];
    auras::remove_auras(run.app.world_mut(), tank, RemoveReason::Cancelled, |aura| {
        aura.spell_id == 195630
    });
    for spell in [100780, 121253] {
        crate::spell_cast::creature_cast(run.app.world_mut(), tank, spell, Some(source), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        assert!(
            !auras::has_aura(run.app.world(), tank, 195630),
            "spell {spell} must not grant mastery"
        );
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), tank, 205523, Some(source), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        auras::has_aura(run.app.world(), tank, 195630),
        "Blackout Kick grants mastery"
    );
    auras::remove_auras(run.app.world_mut(), tank, RemoveReason::Cancelled, |aura| {
        aura.spell_id == 195630
    });
    crate::combat::replay_healer_hit(run.app.world_mut(), source, tank, &avoided_hit("Hit"))
        .unwrap();
    run.app.update();
    assert!(
        auras::has_aura(run.app.world(), tank, 195630),
        "incoming physical melee hit grants mastery"
    );
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn simc_dummy_bleed_tick_grants_elusive_brawler_but_self_damage_does_not() {
    use crate::auras::{self, RemoveReason};
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(268, 1).unwrap();
    let tank = run.actor;
    let source = run.targets[0];
    assert!(auras::has_aura(run.app.world(), tank, 117906));
    auras::remove_auras(run.app.world_mut(), tank, RemoveReason::Cancelled, |aura| {
        aura.spell_id == 195630
    });
    crate::spell_cast::effects::replay_simc_dummy_bleed(run.app.world_mut(), tank, tank, 1_000.0);
    assert!(
        !auras::has_aura(run.app.world(), tank, 195630),
        "self damage does not grant mastery"
    );
    crate::spell_cast::effects::replay_simc_dummy_bleed(
        run.app.world_mut(),
        source,
        tank,
        25_000.0,
    );
    assert!(
        auras::has_aura(run.app.world(), tank, 195630),
        "incoming physical bleed tick grants mastery"
    );
}

fn avoided_hit(outcome: &str) -> super::healer_intake::Hit {
    super::healer_intake::Hit {
        tick: 1,
        source_guid: 0,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 1_000.0,
        level_scaling_factor: 1.0,
        kind: super::healer_intake::HitKind::Melee {
            hand: 0,
            outcome: outcome.into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    }
}

#[test]
#[ignore = "uses native TDB parry-only proc metadata and real replayed outcomes"]
fn victim_avoidance_proc_honors_native_hit_mask() {
    use crate::auras::{AuraApplication, AuraInstance, AuraSource};
    use bevy::prelude::*;
    #[derive(Component, Default)]
    struct Hits(u32);
    fn count(
        world: &mut World,
        unit: Entity,
        _: &AuraInstance,
        _: &crate::auras::procs::AuraEvent,
    ) {
        world.get_mut::<Hits>(unit).unwrap().0 += 1;
    }
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let actor = run.actor;
    let source = run.targets[0];
    run.app
        .world_mut()
        .entity_mut(actor)
        .insert(Hits::default());
    run.app
        .world_mut()
        .resource_mut::<crate::auras::scripts::AuraScripts>()
        .0
        .entry(23547)
        .or_default()
        .on_damage_proc = Some(count);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 23547,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(run.app.world(), actor, 23547));
    for outcome in ["Miss", "Dodge", "Parry"] {
        crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &avoided_hit(outcome))
            .unwrap();
        run.app.update();
        let expected = u32::from(outcome == "Parry");
        assert_eq!(
            run.app.world().get::<Hits>(actor).unwrap().0,
            expected,
            "{outcome}"
        );
    }
}

#[test]
#[ignore = "uses selected MID2 Grand Crusader and real missed, dodged and parried attacks"]
fn grand_crusader_resets_shield_after_each_avoidance_kind() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let source = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 85043));
    let shield = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(31935)
        .unwrap()
        .clone();
    let remaining = |run: &super::dps_parity::Run| {
        run.app
            .world()
            .get::<crate::spell_cast::SpellHistory>(actor)
            .unwrap()
            .remaining_cooldown_ms(&shield, crate::spell_cast::now(run.app.world()))
    };
    for outcome in ["Miss", "Dodge", "Parry"] {
        for _ in 0..40 {
            run.app.update();
        }
        if remaining(&run) != 0 {
            for _ in 0..320 {
                run.app.update();
            }
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 31935, Some(source), false)
            .unwrap();
        let mut observed = false;
        for _ in 0..40 {
            if remaining(&run) < 100 {
                crate::spell_cast::creature_cast(
                    run.app.world_mut(),
                    actor,
                    31935,
                    Some(source),
                    false,
                )
                .unwrap();
                for _ in 0..40 {
                    run.app.update();
                }
            }
            let before = remaining(&run);
            crate::combat::replay_healer_hit(
                run.app.world_mut(),
                source,
                actor,
                &avoided_hit(outcome),
            )
            .unwrap();
            run.app.update();
            if before > 100 && remaining(&run) == 0 {
                observed = true;
                break;
            }
            for _ in 0..21 {
                run.app.update();
            }
        }
        assert!(
            observed,
            "{outcome} did not reset an active Avenger's Shield cooldown"
        );
    }
}

#[test]
#[ignore = "uses real MID2 Holy Bulwark consumption and Avenger's Shield"]
fn reflection_of_radiance_can_reset_shield_from_self_absorbs() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    for _ in 0..40 {
        run.app.update();
    }
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 31935, Some(target), false)
        .unwrap();
    let shield = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(31935)
        .unwrap()
        .clone();
    let remaining = |run: &super::dps_parity::Run| {
        run.app
            .world()
            .get::<crate::spell_cast::SpellHistory>(actor)
            .unwrap()
            .remaining_cooldown_ms(&shield, crate::spell_cast::now(run.app.world()))
    };
    assert!(remaining(&run) > 0);
    for _ in 0..30 {
        let (_, absorbed) =
            crate::auras::absorb_damage(run.app.world_mut(), actor, None, 1, 1_000.0);
        assert!(absorbed > 0.0);
        if remaining(&run) == 0 {
            return;
        }
    }
    panic!("real self-absorb events did not reset Avenger's Shield");
}

#[test]
#[ignore = "uses the selected MID2 player and real Blessed Hammer casts"]
fn blessed_hammer_can_reset_avengers_shield_through_grand_crusader() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let shield = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(31935)
        .unwrap()
        .clone();
    let remaining = |run: &super::dps_parity::Run| {
        run.app
            .world()
            .get::<crate::spell_cast::SpellHistory>(actor)
            .unwrap()
            .remaining_cooldown_ms(&shield, crate::spell_cast::now(run.app.world()))
    };
    let mut resets = 0;
    for _ in 0..20 {
        if remaining(&run) == 0 {
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                31935,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..40 {
                run.app.update();
            }
        }
        let before = remaining(&run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 204019, Some(target), false)
            .unwrap();
        run.app.update();
        if before > 100 && remaining(&run) == 0 {
            resets += 1;
        }
        for _ in 0..100 {
            run.app.update();
        }
    }
    assert!(
        resets > 0,
        "selected Grand Crusader did not reset an active Avenger's Shield cooldown"
    );
}

#[test]
#[ignore = "uses the selected MID2 player and a real Consecration cast"]
fn protection_consecration_field_persists_after_cast() {
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    for _ in 0..200 {
        run.app.update();
    }
    let actor = run.actor;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 26573, Some(actor), false)
        .unwrap();
    run.app.update();
    assert!(crate::spell_cast::scripts::has_active_consecration(
        run.app.world_mut(),
        actor
    ));
    for _ in 0..200 {
        run.app.update();
    }
    assert!(crate::spell_cast::scripts::has_active_consecration(
        run.app.world_mut(),
        actor
    ));
}

#[test]
#[ignore = "uses the selected MID2 player and actual Avenging Wrath cast"]
fn zealots_paragon_extends_reduced_avenging_wrath_on_real_hammer() {
    use crate::auras::ActiveAuras;
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    for _ in 0..40 {
        run.app.update();
    }
    let actor = run.actor;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 31884, Some(actor), false)
        .unwrap();
    run.app.update();
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 31884)
        .unwrap();
    assert_eq!(
        aura.duration_ms, 12_000,
        "selected Righteous Protector reduces the native duration"
    );
    let expiry = aura.expires_at.unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241413, Some(target), false)
        .unwrap();
    run.app.update();
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 31884)
        .unwrap();
    assert!(
        (aura.expires_at.unwrap() - expiry - 1.0).abs() < 0.001,
        "{aura:?}"
    );
}

#[test]
#[ignore = "uses the selected MID2 player and real Protection spender casts"]
fn divine_guidance_stacks_on_spenders_and_is_consumed_by_consecration() {
    use crate::auras::{ActiveAuras, AuraSource};
    use crate::spell_cast::powers::PowerPools;
    use shared::components::{Health, PowerType};
    use shared::protocol::CombatLogKind;
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let stacks = |run: &super::dps_parity::Run| {
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 433106 && aura.source == AuraSource::Cast)
            .map_or(0, |aura| aura.stacks)
    };
    for expected in 1..=5 {
        let mut pools = run.app.world_mut().get_mut::<PowerPools>(actor).unwrap();
        let pool = pools
            .pools
            .iter_mut()
            .find(|pool| pool.power == PowerType::HolyPower)
            .unwrap();
        pool.current = pool.max;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 53600, Some(target), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        assert_eq!(stacks(&run), expected);
    }
    {
        let mut health = run.app.world_mut().get_mut::<Health>(actor).unwrap();
        health.current = health.max * 0.5;
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 26573, Some(actor), false)
        .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert_eq!(stacks(&run), 0);
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(433808)
                && event.amount > 0)
    );
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(actor.to_bits())
                && event.kind == CombatLogKind::Heal
                && event.spell_id == Some(433807)
                && event.amount > 0)
    );
}

#[test]
#[ignore = "uses real Protection casts and the replay Bloodlust path"]
fn tank_replay_bloodlust_hastes_the_first_global_cooldown() {
    fn active_after_900ms(bloodlust: bool) -> bool {
        let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        if bloodlust {
            super::tank_replay::apply_replay_bloodlust(run.app.world_mut(), actor).unwrap();
        }
        let spell = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(204019)
            .unwrap()
            .clone();
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 204019, Some(target), false)
            .unwrap();
        let world = run.app.world();
        world
            .get::<crate::spell_cast::SpellHistory>(actor)
            .unwrap()
            .on_gcd(&spell, crate::spell_cast::history_now(world) + 0.9)
    }
    assert!(active_after_900ms(false), "ordinary GCD remains active");
    assert!(
        !active_after_900ms(true),
        "Bloodlust must haste the first GCD before the next app update"
    );
}

#[test]
#[ignore = "uses selected MID2 Protection and the literal action list"]
fn protection_sotr_casts_during_an_active_global_cooldown() {
    use crate::spell_cast::powers::PowerPools;
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::PowerType;
    use shared::protocol::CombatLogKind;
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 204019, Some(target), false)
        .unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::spell_cast::SpellHistory>(actor)
            .unwrap()
            .has_active_gcd(crate::spell_cast::history_now(run.app.world()))
    );
    run.app
        .world_mut()
        .get_mut::<PowerPools>(actor)
        .unwrap()
        .get_mut(PowerType::HolyPower)
        .unwrap()
        .current = 5.0;
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    super::dps_parity::drive_literal_tick(&mut run).unwrap();
    assert!(
        run.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(53600)
                && event.kind == CombatLogKind::CastSuccess),
        "SotR must cast while the unrelated GCD is active"
    );
    assert!(
        run.app
            .world()
            .get::<crate::spell_cast::SpellHistory>(actor)
            .unwrap()
            .has_active_gcd(crate::spell_cast::history_now(run.app.world()))
    );
}

#[test]
#[ignore = "uses real Protection Judgment Holy Power overflow"]
fn replay_resource_ledger_records_judgment_overflow() {
    use crate::spell_cast::powers::PowerPools;
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::PowerType;
    use shared::protocol::CombatLogKind;
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    run.app
        .world_mut()
        .get_mut::<PowerPools>(actor)
        .unwrap()
        .get_mut(PowerType::HolyPower)
        .unwrap()
        .current = 5.0;
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 275779, Some(target), false)
        .unwrap();
    let event = run.app.world().resource::<ObservedCombatEvents>().0[start..]
        .iter()
        .find(|event| event.spell_id == Some(220637) && event.kind == CombatLogKind::Energize)
        .expect("Judgment grants Holy Power");
    assert_eq!(
        super::tank_replay::resource_ledger_entry(event, actor, 3.25),
        Some(serde_json::json!({
            "time_seconds":3.25,"spell_id":220637,"gained":0,"wasted":1
        }))
    );
}

#[test]
#[ignore = "uses the selected MID2 Protection player"]
fn protection_replay_starts_with_blessed_hammer_holy_power() {
    use crate::spell_cast::powers::PowerPools;
    use shared::components::PowerType;
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let actor = run.actor;
    super::tank_replay::apply_simc_starting_holy_power(&mut run);
    let power = run
        .app
        .world()
        .get::<PowerPools>(actor)
        .unwrap()
        .get(PowerType::HolyPower)
        .unwrap()
        .current;
    assert_eq!(power, 1.0);
    run.app
        .world_mut()
        .get_mut::<PowerPools>(actor)
        .unwrap()
        .get_mut(PowerType::HolyPower)
        .unwrap()
        .current = 3.0;
    super::tank_replay::apply_simc_starting_holy_power(&mut run);
    let power = run
        .app
        .world()
        .get::<PowerPools>(actor)
        .unwrap()
        .get(PowerType::HolyPower)
        .unwrap()
        .current;
    assert_eq!(power, 1.0, "SimC caps combat-start Holy Power at one");
}

#[test]
#[ignore = "uses the MID2 Protection precombat plan and raid buffs"]
fn tank_raid_buffs_apply_before_precombat_bulwark_snapshot() {
    let _seed = crate::random::seed_scope(70448);
    let buffs = [
        crate::simc_tank_timeline::RaidBuff {
            name: "mark_of_the_wild".into(),
            spell_id: 1126,
        },
        crate::simc_tank_timeline::RaidBuff {
            name: "power_word_fortitude".into(),
            spell_id: 21562,
        },
    ];
    let mut run = super::dps_parity::prepare_registered_player_with(66, 1, |run| {
        super::tank_replay::apply_simc_raid_buffs(run, &buffs)
    })
    .unwrap();
    let actor = run.actor;
    let max = run
        .app
        .world()
        .get::<shared::components::Health>(actor)
        .unwrap()
        .max;
    let vers = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .versatility_pct;
    let (_, shield) = crate::auras::absorb_damage(run.app.world_mut(), actor, None, 1, max);
    assert!((max - 1_492_640.0).abs() < 10.0, "max health {max}");
    assert!((vers - 23.8889).abs() < 0.01, "versatility {vers}");
    assert!(
        (shield - 277_382.3).abs() < 10.0,
        "precombat shield {shield}"
    );
}

#[test]
#[ignore = "uses selected MID2 Holy Bulwark parent refresh"]
fn holy_bulwark_refresh_regrants_initial_absorb() {
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let actor = run.actor;
    assert!(crate::auras::has_aura(run.app.world(), actor, 432496));
    let max = run
        .app
        .world()
        .get::<shared::components::Health>(actor)
        .unwrap()
        .max;
    let vers = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .versatility_pct;
    let (_, consumed) = crate::auras::absorb_damage(run.app.world_mut(), actor, None, 1, max);
    let expected = max * 0.15 * (1.0 + vers / 100.0);
    assert!(
        (consumed - expected).abs() < 2.0,
        "initial={consumed} expected={expected}"
    );
    assert!(!crate::auras::has_aura(run.app.world(), actor, 432607));
    let before_counts = super::tank_replay::bulwark_observations(run.app.world(), actor);
    assert!(before_counts.0 >= 1, "precombat start was not captured");
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 432496,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let after_counts = super::tank_replay::bulwark_observations(run.app.world(), actor);
    assert_eq!(after_counts.0, before_counts.0);
    assert_eq!(after_counts.1, before_counts.1 + 1);
    let (_, refreshed) = crate::auras::absorb_damage(run.app.world_mut(), actor, None, 1, max);
    assert!(
        (refreshed - expected).abs() < 2.0,
        "refresh={refreshed} expected={expected}"
    );
}

#[test]
#[ignore = "uses the selected MID2 player and native Holy Bulwark casts"]
fn holy_bulwark_absorbs_incoming_hits_and_grows_to_native_cap() {
    fn cast_bulwark() -> super::dps_parity::Run {
        let (profile, traits) = super::dps_parity::read_reference(66).unwrap();
        let mut run = super::dps_parity::spawn_reference(
            profile,
            &traits,
            1,
            crate::followers::simulator::ParityActor::Player { race: 6 },
        )
        .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            432459,
            Some(run.actor),
            false,
        )
        .unwrap();
        for _ in 0..2 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 432496));
        run
    }
    let mut run = cast_bulwark();
    let max = run
        .app
        .world()
        .get::<shared::components::Health>(run.actor)
        .unwrap()
        .max;
    let vers = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .versatility_pct;
    let (_, consumed) =
        crate::auras::absorb_damage(run.app.world_mut(), run.actor, None, 1, max * 0.01);
    assert!((consumed - max * 0.01).abs() < 2.0);
    for _ in 0..40 {
        run.app.update();
    }
    let (_, remaining) = crate::auras::absorb_damage(run.app.world_mut(), run.actor, None, 1, max);
    let expected = max * (0.15 * (1.0 + vers / 100.0) + 0.02 - 0.01);
    assert!(
        (remaining - expected).abs() < 2.0,
        "{remaining} != {expected}"
    );

    let mut run = cast_bulwark();
    let max = run
        .app
        .world()
        .get::<shared::components::Health>(run.actor)
        .unwrap()
        .max;
    for _ in 0..360 {
        run.app.update();
    }
    let (_, absorbed) = crate::auras::absorb_damage(run.app.world_mut(), run.actor, None, 1, max);
    assert!(
        (absorbed - max * 0.30).abs() < 2.0,
        "{absorbed} != {}",
        max * 0.30
    );
}

#[test]
#[ignore = "uses the selected MID2 Protection player and native Valiance"]
fn valiance_free_word_of_glory_shortens_holy_armaments_recharge() {
    use crate::auras::{AuraApplication, AuraSource};
    // Returns how far Word of Glory moved the pending Holy Armaments recharge.
    let recharge_shift = |free_word: bool| {
        let (profile, traits) = super::dps_parity::read_reference(66).unwrap();
        let mut run = super::dps_parity::spawn_reference(
            profile,
            &traits,
            1,
            crate::followers::simulator::ParityActor::Player { race: 6 },
        )
        .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        let actor = run.actor;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 432459, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let pending = |run: &super::dps_parity::Run| {
            let history = run
                .app
                .world()
                .get::<crate::spell_cast::SpellHistory>(actor)
                .unwrap();
            crate::spell_cast::test_harness::charge_recovery(history, 2220)
                .1
                .unwrap()
        };
        let before = pending(&run);
        if free_word {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id: 327510,
                    effect_mask: u32::MAX,
                    source: AuraSource::Cast,
                },
            );
        } else {
            let mut pools = run
                .app
                .world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
                .unwrap();
            pools
                .get_mut(shared::components::PowerType::HolyPower)
                .unwrap()
                .current = 3.0;
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 85673, Some(actor), false)
            .unwrap();
        run.app.update();
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 327510),
            "Word of Glory consumes free Shining Light"
        );
        pending(&run) - before
    };
    let free = recharge_shift(true);
    let paid = recharge_shift(false);
    assert!(
        (free + 3.0).abs() < 1e-3,
        "free Word of Glory moved recharge by {free}"
    );
    assert!(
        paid.abs() < 1e-3,
        "paid Word of Glory moved recharge by {paid}"
    );
}

#[test]
#[ignore = "uses the selected MID2 Protection player and native Afterimage"]
fn afterimage_threshold_word_of_glory_echoes_onto_the_paladin() {
    use crate::auras::{AuraApplication, AuraSource};
    let (profile, traits) = super::dps_parity::read_reference(66).unwrap();
    let mut run = super::dps_parity::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 6 },
    )
    .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    let actor = run.actor;
    let world = run.app.world_mut();
    crate::auras::apply_aura(
        world,
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 400745,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    assert!(crate::auras::set_self_cast_stacks(world, actor, 400745, 20));
    world
        .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
        .unwrap()
        .get_mut(shared::components::PowerType::HolyPower)
        .unwrap()
        .current = 3.0;
    let mut health = world.get_mut::<shared::components::Health>(actor).unwrap();
    health.current = health.max * 0.1;
    let before = world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::creature_cast(world, actor, 85673, Some(actor), false).unwrap();
    run.app.update();
    let heals: Vec<_> = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[before..]
        .iter()
        .filter(|event| {
            event.kind == shared::protocol::CombatLogKind::Heal
                && event.spell_id == Some(85673)
                && event.target == Some(actor.to_bits())
        })
        .map(|event| (event.amount + event.overflow, event.crit))
        .collect();
    println!("AFTERIMAGE_HEALS {heals:?}");
    assert_eq!(
        heals.len(),
        2,
        "Word of Glory and its Afterimage echo: {heals:?}"
    );
    let stacks = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 400745)
        .map_or(0, |aura| aura.stacks);
    assert_eq!(stacks, 3, "20 stored + 3 spent - 20 echoed");
}

fn owned_aura(run: &super::dps_parity::Run, spell_id: u32) -> Option<crate::auras::AuraInstance> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| aura.spell_id == spell_id && aura.caster == run.actor)
        .cloned()
}

fn remaining_seconds(run: &super::dps_parity::Run, spell_id: u32) -> f64 {
    let expiry = owned_aura(run, spell_id)
        .and_then(|aura| aura.expires_at)
        .expect("timed owned aura");
    expiry - crate::spell_cast::now(run.app.world())
}

fn apply_self_aura(run: &mut super::dps_parity::Run, spell_id: u32) {
    let actor = run.actor;
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn engaged_protection_paladin() -> super::dps_parity::Run {
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    for _ in 0..40 {
        run.app.update();
    }
    run
}

fn set_holy_power(run: &mut super::dps_parity::Run, amount: f32) {
    let actor = run.actor;
    run.app
        .world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
        .unwrap()
        .get_mut(shared::components::PowerType::HolyPower)
        .unwrap()
        .current = amount;
}

#[test]
#[ignore = "uses the selected MID2 Protection player and native Laying Down Arms"]
fn laying_down_arms_rewards_refreshed_and_faded_armaments() {
    let mut run = engaged_protection_paladin();
    let actor = run.actor;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 633, Some(actor), false).unwrap();
    run.app.update();
    let lay_on_hands = |run: &super::dps_parity::Run| {
        let world = run.app.world();
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(633)
            .unwrap();
        world
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .unwrap()
            .remaining_cooldown_ms(spell, crate::spell_cast::now(world))
    };
    let initial = lay_on_hands(&run);
    assert!(owned_aura(&run, 327510).is_none());

    apply_self_aura(&mut run, 432502);
    assert!(
        owned_aura(&run, 327510).is_none(),
        "a fresh armament is not laid down"
    );
    apply_self_aura(&mut run, 432502);
    // SimC keeps the 20 s remainder, capped at two durations; native Pandemic would give 26 s.
    let refreshed = remaining_seconds(&run, 432502);
    assert!(
        (refreshed - 40.0).abs() < 0.05,
        "refreshed Sacred Weapon lasts {refreshed}"
    );
    assert_eq!(owned_aura(&run, 327510).map(|aura| aura.stacks), Some(1));
    assert_eq!(initial - lay_on_hands(&run), 15_000);

    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Expired,
        |aura| aura.spell_id == 432502,
    );
    assert_eq!(owned_aura(&run, 327510).map(|aura| aura.stacks), Some(2));
    assert_eq!(initial - lay_on_hands(&run), 30_000);
}

#[test]
#[ignore = "uses the selected MID2 Protection player and native Shining Light"]
fn word_of_glory_spends_one_shining_light_stack() {
    let mut run = engaged_protection_paladin();
    let actor = run.actor;
    apply_self_aura(&mut run, 327510);
    apply_self_aura(&mut run, 327510);
    assert_eq!(owned_aura(&run, 327510).map(|aura| aura.stacks), Some(2));
    set_holy_power(&mut run, 0.0);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 85673, Some(actor), false)
        .unwrap();
    run.app.update();
    assert_eq!(owned_aura(&run, 327510).map(|aura| aura.stacks), Some(1));
}

#[test]
#[ignore = "uses the selected MID2 Protection player and native Blessing of the Forge"]
fn blessing_of_the_forge_wings_summon_weapon_and_echo_spenders() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;

    // Returns Forge's Reckoning damage and Sacred Word healing after one Shield and Word.
    let spenders = |wings: bool| {
        let mut run = engaged_protection_paladin();
        let actor = run.actor;
        let target = run.targets[0];
        if wings {
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 31884, None, false)
                .unwrap();
            run.app.update();
            assert!((remaining_seconds(&run, 434255) - 20.0).abs() < 0.1);
            assert!(
                owned_aura(&run, 432502).is_some(),
                "wings summon Sacred Weapon"
            );
        }
        for (spell, spell_target) in [(53600, target), (85673, actor)] {
            for _ in 0..40 {
                run.app.update();
            }
            set_holy_power(&mut run, 3.0);
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                spell,
                Some(spell_target),
                false,
            )
            .unwrap();
        }
        for _ in 0..10 {
            run.app.update();
        }
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        let total = |spell: u32, kind: CombatLogKind, unit: bevy::prelude::Entity| {
            events
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.target == Some(unit.to_bits())
                        && event.spell_id == Some(spell)
                        && event.kind == kind
                })
                .map(|event| event.amount)
                .sum::<i32>()
        };
        (
            total(447258, CombatLogKind::Damage, target),
            events
                .iter()
                .any(|event| event.spell_id == Some(447246) && event.kind == CombatLogKind::Heal),
        )
    };
    assert_eq!(spenders(false), (0, false));
    let (reckoning, sacred_word) = spenders(true);
    assert!(
        reckoning > 0,
        "Shield echoes Forge's Reckoning on its target"
    );
    assert!(sacred_word, "Word of Glory echoes Sacred Word");
}

#[test]
#[ignore = "uses the selected MID2 Protection player and native Masterwork"]
fn masterwork_hammers_forge_lesser_weapon_strikes_on_the_dummy() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;

    let mut run = engaged_protection_paladin();
    let actor = run.actor;
    let target = run.targets[0];
    let stacks =
        |run: &super::dps_parity::Run, spell| owned_aura(run, spell).map_or(0, |aura| aura.stacks);
    let before = stacks(&run, 1271436);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 432472, Some(actor), false)
        .unwrap();
    run.app.update();
    assert_eq!(
        stacks(&run, 1271436),
        before + 3,
        "Sacred Weapon hardcast adds three"
    );
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 204019, Some(target), false)
        .unwrap();
    run.app.update();
    assert_eq!(
        stacks(&run, 1271436),
        before + 2,
        "Blessed Hammer spends one"
    );
    assert_eq!(
        stacks(&run, 1239091),
        5,
        "one forged Lesser Weapon holds five charges"
    );
    // Auto attacks over 6 s; Lesser Weapon's native ICD is 0.75 s.
    for _ in 0..120 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let count = |spell: u32, kind: CombatLogKind| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == kind
            })
            .count()
    };
    let strikes = count(1239282, CombatLogKind::Damage);
    let heals = count(1239276, CombatLogKind::Heal);
    assert!(strikes > 0, "Lesser Weapon strikes the dummy");
    assert!(
        events.iter().any(|event| event.spell_id == Some(1239282)
            && event.target == Some(target.to_bits())
            && event.amount > 0),
        "strikes deal damage to the dummy"
    );
    assert_eq!(
        usize::from(stacks(&run, 1239091)) + strikes + heals,
        5,
        "each proc spends one charge"
    );
}

#[test]
#[ignore = "uses the selected MID2 Protection player and native Sacred Weapon"]
fn sacred_weapon_procs_strike_the_dummy_with_the_lone_target_bonus() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;

    let strikes = |run: &super::dps_parity::Run, unit: bevy::prelude::Entity| {
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(unit.to_bits())
                    && event.spell_id == Some(432616)
                    && event.kind == CombatLogKind::Damage
            })
            .map(|event| event.amount)
            .collect::<Vec<_>>()
    };
    // The worn weapon procs from auto attacks over 15 s of its 20 s lifetime.
    let mut run = engaged_protection_paladin();
    apply_self_aura(&mut run, 432502);
    for _ in 0..300 {
        run.app.update();
    }
    assert!(
        !strikes(&run, run.targets[0]).is_empty(),
        "Sacred Weapon strikes the dummy"
    );
    // Lowest strike per dummy; crits only raise amounts and Holy damage ignores armor.
    let lowest = |dummies: usize| {
        let mut run = super::dps_parity::prepare_registered_player(66, dummies).unwrap();
        super::dps_parity::engage(&mut run);
        let (actor, primary) = (run.actor, run.targets[0]);
        for _ in 0..12 {
            crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 432616, Some(primary), 1);
            run.app.update();
        }
        run.targets
            .iter()
            .map(|&unit| {
                strikes(&run, unit)
                    .into_iter()
                    .min()
                    .expect("every dummy is struck")
            })
            .collect::<Vec<_>>()
    };
    let lone = lowest(1);
    let pair = lowest(2);
    assert_eq!(pair.len(), 2);
    let ratio = lone[0] as f32 / pair[0] as f32;
    assert!(
        (ratio - 1.5).abs() < 0.02,
        "lone-target strike ratio {ratio}"
    );
}

#[test]
#[ignore = "uses the selected MID2 Protection player and precombat APL"]
fn protection_default_precombat_casts_holy_bulwark() {
    let run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 432496));
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 432478));
}

#[test]
#[ignore = "uses the selected MID2 66 player and native armament override"]
fn protection_holy_armaments_alternate_after_real_casts() {
    let (profile, traits) = super::dps_parity::read_reference(66).unwrap();
    let mut run = super::dps_parity::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 6 },
    )
    .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 432459)
            .unwrap(),
        432459
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        432459,
        Some(run.actor),
        false,
    )
    .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 432478));
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 432459)
            .unwrap(),
        432472
    );
    let replacement =
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 432459)
            .unwrap();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        replacement,
        Some(run.actor),
        false,
    )
    .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 432478));
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 432459)
            .unwrap(),
        432459
    );
}

#[test]
#[ignore = "uses the real MID2 Protection profile and native fields"]
fn protection_apl_reads_selected_apex_and_native_consecration() {
    use super::tank_replay::protection_rotation::{self, Context};
    use server::simc_apl::Facts;
    let (profile, traits) = super::dps_parity::read_reference(66).unwrap();
    let potion = profile.consumables["potion"].clone();
    let mut run = super::dps_parity::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 6 },
    )
    .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    let mut bindings = protection_rotation::bindings();
    let equipment = super::dps_parity::literal_rotation::equipment::Equipment::load(
        run.app.world(),
        run.actor,
        &mut bindings,
    )
    .unwrap();
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let active =
        crate::spell_cast::scripts::has_active_consecration(run.app.world_mut(), run.actor);
    let context = Context::new(
        run.app.world(),
        run.actor,
        &run.targets,
        &bindings,
        &equipment,
        encounter,
        active,
        None,
        &potion,
    );
    assert_eq!(context.value("apex.3").unwrap(), 1.0);
    assert_eq!(
        context
            .value("talent.instrument_of_the_divine.enabled")
            .unwrap(),
        1.0
    );
    assert_eq!(context.value("consecration.up").unwrap(), 0.0);
    assert_eq!(context.value("potion.liquid_luster").unwrap(), 1.0);
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 26573, None, false).unwrap();
    for _ in 0..5 {
        run.app.update();
    }
    let active =
        crate::spell_cast::scripts::has_active_consecration(run.app.world_mut(), run.actor);
    let context = Context::new(
        run.app.world(),
        run.actor,
        &run.targets,
        &bindings,
        &equipment,
        encounter,
        active,
        None,
        &potion,
    );
    assert_eq!(context.value("consecration.up").unwrap(), 1.0);
    assert!(
        context.value("next_armament").is_err(),
        "uninitialized armament state must not be invented"
    );
}

#[test]
#[ignore = "uses the native Consecration ground field"]
fn protection_apl_consecration_tracks_field_not_caster_position() {
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[26573, 81297], 2).unwrap();
    assert!(!crate::spell_cast::scripts::has_active_consecration(
        h.app.world_mut(),
        h.caster
    ));
    h.cast_and_deliver(26573, None);
    h.advance(5);
    assert!(crate::spell_cast::scripts::has_active_consecration(
        h.app.world_mut(),
        h.caster
    ));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::Position {
            x: 40.0,
            y: 0.0,
            z: 0.0,
        });
    assert!(crate::spell_cast::scripts::has_active_consecration(
        h.app.world_mut(),
        h.caster
    ));
    h.advance(340);
    assert!(!crate::spell_cast::scripts::has_active_consecration(
        h.app.world_mut(),
        h.caster
    ));
}

#[test]
#[ignore = "reads the pinned MID2 Protection profile"]
fn protection_apl_selects_shield_using_the_real_profile_conditions() {
    use server::simc_apl::{Action, ActionFacts, Facts};
    struct FactsForShield {
        full_power: bool,
    }
    impl Facts for FactsForShield {
        fn value(&self, name: &str) -> Result<f64, String> {
            Ok(match name {
                "holy_power" if self.full_power => 5.0,
                "talent.instrument_of_the_divine" | "buff.valor.up" | "buff.divine_purpose.up"
                    if !self.full_power =>
                {
                    1.0
                }
                _ => 0.0,
            })
        }
    }
    impl ActionFacts for FactsForShield {
        fn value_for(&self, _: &Action, name: &str) -> Result<f64, String> {
            self.value(name)
        }
    }
    let (profile, _) = super::dps_parity::read_reference(66).unwrap();
    let program = super::tank_replay::protection_program(&profile.options["actions"]).unwrap();
    let select = |full_power| {
        program
            .select(
                "default",
                &FactsForShield { full_power },
                &mut program.state(),
                |action| Ok(action.name == "shield_of_the_righteous"),
            )
            .unwrap()
    };
    assert_eq!(select(true).unwrap().name, "shield_of_the_righteous");
    assert!(select(false).is_none());
}

#[test]
#[ignore = "requires canonical MID2 Vengeance profile"]
fn traced_mortal_wounds_reduce_dummy_healing_for_the_encounter() {
    use crate::simc_tank_timeline::RaidBuff;
    fn heal(wounded: bool) -> f32 {
        let _seed = crate::random::seed_scope(70_448);
        let mut run = super::dps_parity::prepare_registered_player(581, 1).unwrap();
        let debuffs = if wounded {
            vec![RaidBuff {
                name: "mortal_wounds".into(),
                spell_id: 115804,
            }]
        } else {
            Vec::new()
        };
        super::tank_replay::apply_simc_dummy_debuffs(&mut run, &debuffs).unwrap();
        for _ in 0..220 {
            run.app.update();
        }
        let actor = run.actor;
        let target = run.targets[0];
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current = health.max / 2.0;
        let before = health.current;
        let spell = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(213011)
            .unwrap()
            .clone();
        let ctx = crate::spell_cast::effects::EffectCtx {
            caster: actor,
            spell: &spell,
            depth: 1,
            dest: None,
            chain_multiplier: 1.0,
        };
        crate::spell_cast::effects::apply_heal(
            run.app.world_mut(),
            &ctx,
            target,
            1000.0,
            false,
            false,
        );
        run.app
            .world()
            .get::<shared::components::Health>(target)
            .unwrap()
            .current
            - before
    }
    let base = heal(false);
    let reduced = heal(true);
    assert!(
        base > 0.0 && (reduced / base - 0.5).abs() < 0.01,
        "dummy heal {base}/{reduced}"
    );
}

#[test]
#[ignore = "requires canonical MID2 Vengeance profile"]
fn simc_bronze_buff_shortens_real_infernal_strike_recovery() {
    use crate::simc_tank_timeline::RaidBuff;
    fn recovery(bronze: bool) -> f64 {
        let _seed = crate::random::seed_scope(70_448);
        let mut run = super::dps_parity::prepare_registered_player(581, 1).unwrap();
        let buffs = if bronze {
            vec![RaidBuff {
                name: "blessing_of_the_bronze".into(),
                spell_id: 381741,
            }]
        } else {
            Vec::new()
        };
        super::tank_replay::apply_simc_raid_buffs(&mut run, &buffs).unwrap();
        if !bronze {
            run.app.update();
            run.app.update();
        }
        super::dps_parity::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        let destination = crate::spell_cast::targets::position(run.app.world(), target).unwrap();
        crate::spell_cast::cast_for_player_with_targets(
            run.app.world_mut(),
            actor,
            189110,
            crate::spell_cast::CastTargets {
                unit: target,
                primary: Some(target),
                destination: Some(destination.to_array()),
            },
        )
        .unwrap();
        let world = run.app.world();
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(189110)
            .unwrap();
        let bindings = crate::simc_apl_native::Bindings::default();
        let encounter = crate::simc_apl_native::Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        };
        crate::simc_apl_native::NativeFacts::new(world, actor, &bindings, encounter, 1)
            .cooldown_for_spell(spell, "full_recharge_time")
            .unwrap()
    }
    let baseline = recovery(false);
    let reduced = recovery(true);
    assert!(
        baseline > 0.0 && (reduced / baseline - 0.85).abs() < 0.01,
        "base={baseline} bronze={reduced}"
    );
}

#[test]
#[ignore = "requires canonical MID2 Vengeance profile"]
fn simc_dummy_debuffs_modify_real_damage_by_school() {
    use crate::simc_tank_timeline::RaidBuff;
    fn damage(debuffs: &[RaidBuff], spell: u32) -> i32 {
        let _seed = crate::random::seed_scope(70_448);
        let mut run = super::dps_parity::prepare_registered_player(581, 1).unwrap();
        super::tank_replay::apply_simc_dummy_debuffs(&mut run, debuffs).unwrap();
        super::dps_parity::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            spell,
            Some(target),
            1
        ));
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.spell_id == Some(spell)
                    && event.target == Some(target.to_bits())
                    && event.kind == shared::protocol::CombatLogKind::Damage
            })
            .unwrap()
            .amount
    }
    let debuffs: Vec<_> = [
        ("chaos_brand", 1490),
        ("mystic_touch", 113746),
        ("hunters_mark", 259556),
    ]
    .into_iter()
    .map(|(name, spell_id)| RaidBuff {
        name: name.into(),
        spell_id,
    })
    .collect();
    // The Vengeance precombat already applies Chaos Brand; the trace must not stack it twice.
    for (spell, multiplier) in [(247455, 1.03), (228478, 1.05 * 1.03)] {
        let base = damage(&[], spell);
        let buffed = damage(&debuffs, spell);
        assert!(
            (buffed as f64 / base as f64 - multiplier).abs() < 0.01,
            "spell {spell}: base {base}, debuffed {buffed}"
        );
    }
}

#[test]
#[ignore = "requires canonical MID2 profile inputs and a SimC tank-dummy trace"]
fn unregistered_tank_driver_fails_instead_of_using_a_follower() {
    let path = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let error = super::tank_replay::replay_simc(0, Path::new(&path), 300, 70448).unwrap_err();
    assert_eq!(error, "NATIVE_LITERAL_DRIVER_UNAVAILABLE:0");
}

#[test]
#[ignore = "requires canonical MID2 profile inputs and the six SimC tank traces"]
fn pressure_scale_keeps_same_seed_attack_outcomes_for_every_tank() {
    let traces = std::env::var("HEALER_TANK_TRACE_DIR").expect("HEALER_TANK_TRACE_DIR required");
    let chances = |attack: &serde_json::Value| {
        ["parry_pct", "dodge_pct", "block_pct"].map(|chance| attack[chance].as_f64().unwrap())
    };
    // Blood's Dance of Midnight and Brewmaster's Elusive Brawler change avoidance
    // through the tank's own procs and decisions; equal chances must roll equally.
    for spec in [66, 73, 104, 250, 268, 581] {
        let path = Path::new(&traces).join(format!("{spec}.simc-trace.log"));
        let outcomes = |scale| {
            let report = super::tank_replay::replay_simc_with_pressure(
                spec, &path, 300, 70448, false, scale,
            )
            .unwrap();
            report["incoming_outcome_sequence"]
                .as_array()
                .unwrap()
                .clone()
        };
        let normal = outcomes(1.0);
        let pressured = outcomes(2.0);
        let shared = normal.len().min(pressured.len());
        assert!(shared >= 40, "spec {spec}: only {shared} shared attacks");
        let mismatch = (0..shared).find(|&attack| {
            let same_chances = chances(&normal[attack]) == chances(&pressured[attack]);
            same_chances && normal[attack]["outcome"] != pressured[attack]["outcome"]
        });
        assert_eq!(
            mismatch,
            None,
            "spec {spec}: equal chances rolled differently ({} at 1.0x, {} at 2.0x)",
            mismatch.map_or(serde_json::Value::Null, |attack| normal[attack].clone()),
            mismatch.map_or(serde_json::Value::Null, |attack| pressured[attack].clone()),
        );
        if matches!(spec, 66 | 73 | 104 | 581) {
            let differing = (0..shared)
                .filter(|&attack| normal[attack]["outcome"] != pressured[attack]["outcome"])
                .count();
            assert_eq!(differing, 0, "spec {spec}: attack outcomes differ");
        }
    }
}

#[test]
#[ignore = "requires canonical MID2 profile inputs and a real SimC trace"]
fn scaled_real_trace_doubles_raw_pressure_and_shortens_survival() {
    let path = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let path = Path::new(&path);
    let original = crate::simc_tank_timeline::read_trace(path, "Tank_Dummy_Dungeon").unwrap();
    let mut scaled = crate::simc_tank_timeline::read_trace(path, "Tank_Dummy_Dungeon").unwrap();
    super::tank_replay::scale_pressure(&mut scaled, 2.0);
    for (before, after) in original.iter().zip(&scaled) {
        assert_eq!(after.amount, before.amount * 2.0);
        assert_eq!(after.time_ms, before.time_ms);
        assert_eq!(after.kind, before.kind);
        assert_eq!(after.melee_table, before.melee_table);
    }
    let normal =
        super::tank_replay::replay_simc_with_pressure(66, path, 300, 70448, false, 1.0).unwrap();
    let doubled =
        super::tank_replay::replay_simc_with_pressure(66, path, 300, 70448, false, 2.0).unwrap();
    let normal_time = normal["time_to_death"].as_f64().unwrap_or(300.0);
    let doubled_time = doubled["time_to_death"].as_f64().unwrap();
    assert!(
        doubled_time < normal_time,
        "normal {normal_time}, doubled {doubled_time}"
    );
    assert_eq!(doubled["pressure_scale"], 2.0);
}

#[test]
#[ignore = "runs the MID2 Protection APL and SimC tank-dummy pressure"]
fn protection_paladin_simc_apl_uses_shield_of_the_righteous() {
    let path = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report = super::tank_replay::replay_simc(66, Path::new(&path), 300, 70448).unwrap();
    assert_eq!(report["spec"], 66);
    assert_eq!(report["level"], 90);
    assert_eq!(report["rotation_status"], "registered_literal_simc");
    assert!(
        report["tank_casts"]["53600"].as_u64().unwrap_or(0) > 0,
        "{report}"
    );
    assert!(
        report["active_mitigation_uptime"].as_f64().unwrap() > 0.0,
        "{report}"
    );
    assert!(report["damage_taken"].as_f64().unwrap() > 0.0, "{report}");
    assert!(
        report["absorb_per_second"].as_f64().unwrap() > 0.0,
        "{report}"
    );
    assert!(
        report["tank_casts"]["85673"].as_u64().unwrap_or(0) > 0,
        "{report}"
    );
    assert!(
        report["self_heal_per_second"].as_f64().unwrap() > 0.0,
        "{report}"
    );
    assert_eq!(report["external_healing"], false);
    assert!(report["bloodlust_start_seconds"].is_null());
    let shield = report["absorbs_by_spell"]["432607"].as_f64().unwrap();
    assert!(
        (shield
            - report["absorb_per_second"].as_f64().unwrap()
                * report["duration_seconds"].as_f64().unwrap())
        .abs()
            < 1.0,
        "{report}"
    );
    assert!(
        report["self_healing_by_spell"]["85673"].as_f64().unwrap() > 0.0,
        "{report}"
    );
    println!("SIMC_TANK_REPLAY {report}");
}

#[test]
#[ignore = "runs the MID2 Protection APL with the SimC dummy external healer"]
fn protection_simc_dummy_healing_supports_full_comparison() {
    let path = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::tank_replay::replay_simc_with_healing(66, Path::new(&path), 300, 70448, true)
            .unwrap();
    assert_eq!(report["external_healing"], true);
    assert_eq!(report["bloodlust_start_seconds"], 0.0);
    let uptime = report["aura_uptime"]["2825"].as_f64().unwrap();
    assert!(
        (0.13..0.14).contains(&uptime),
        "Bloodlust must expire after 40 seconds: {report}"
    );
    assert_eq!(report["duration_seconds"], 300.0, "{report}");
    let categories = report["damage_taken_by_kind"].as_object().unwrap();
    for kind in ["swing", "strike", "bleed"] {
        assert!(categories[kind].as_f64().unwrap() > 0.0, "{report}");
    }
    let total: f64 = categories
        .values()
        .map(|amount| amount.as_f64().unwrap())
        .sum();
    assert_eq!(total, report["damage_taken"].as_f64().unwrap());
    let before_absorbs = report["incoming_before_absorbs"].as_object().unwrap();
    for kind in ["swing", "strike", "bleed"] {
        assert!(
            before_absorbs[kind].as_f64().unwrap() >= categories[kind].as_f64().unwrap(),
            "{report}"
        );
    }
    let means = report["defense_mean"].as_object().unwrap();
    assert!(
        means["parry_pct"].as_f64().unwrap() > 0.0 && means["armor"].as_f64().unwrap() > 0.0,
        "{report}"
    );
    let outcomes = report["incoming_outcomes"].as_object().unwrap();
    let by_outcome: f64 = report["incoming_by_outcome"]
        .as_object()
        .unwrap()
        .values()
        .map(|amount| amount.as_f64().unwrap())
        .sum();
    assert!(
        (by_outcome
            - before_absorbs["swing"].as_f64().unwrap()
            - before_absorbs["strike"].as_f64().unwrap())
        .abs()
            < 0.1,
        "{report}"
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|(kind, _)| kind.starts_with("swing:"))
            .map(|(_, count)| count.as_u64().unwrap())
            .sum::<u64>(),
        148
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|(kind, _)| kind.starts_with("strike:"))
            .map(|(_, count)| count.as_u64().unwrap())
            .sum::<u64>(),
        10
    );
    assert!(report["time_to_death"].is_null(), "{report}");
    assert!(
        report["external_healing_effective"].as_f64().unwrap() > 0.0,
        "{report}"
    );
    assert!(
        report["absorbs_by_spell"]["432607"].as_f64().unwrap() > 0.0,
        "{report}"
    );
    assert!(
        report["aura_uptime"]["132403"].as_f64().unwrap() > 0.0,
        "{report}"
    );
    assert!(
        report["tank_casts"]["1241413"].as_u64().unwrap_or(0) > 0,
        "{report}"
    );
    assert!(
        report["tank_casts"]["275779"].as_u64().unwrap_or(0) > 0,
        "{report}"
    );
    assert!(
        report["self_healing_by_spell"]["433807"]
            .as_f64()
            .unwrap_or(0.0)
            > 0.0,
        "{report}"
    );
    println!("SIMC_HEALED_TANK_REPLAY {report}");
}

#[test]
#[ignore = "runs a registered parity actor against SimC dummy pressure"]
fn registered_parity_driver_runs_during_real_incoming_damage() {
    let path = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let hits =
        crate::simc_tank_timeline::read_trace(Path::new(&path), "Tank_Dummy_Dungeon").unwrap();
    let _rng = crate::random::seed_scope(70448);
    // Feral is an existing player driver used to exercise shared dispatch, not a tank measurement.
    let run = super::dps_parity::prepare_registered_player(103, 1).unwrap();
    let report = super::tank_replay::replay_registered_run(run, &hits, 300, &[]).unwrap();
    assert_eq!(report["spec"], 103);
    assert_eq!(report["level"], 90);
    assert_eq!(report["rotation_status"], "registered_literal_simc");
    assert!(report["incoming_events"].as_u64().unwrap() > 0, "{report}");
    assert!(report["damage_taken"].as_f64().unwrap() > 0.0, "{report}");
    assert!(
        !report["tank_casts"].as_object().unwrap().is_empty(),
        "{report}"
    );
}

#[test]
#[ignore = "uses the native recorded tank source and recipient"]
fn tank_replay_counts_parried_incoming_damage_as_mitigated() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/dungeons/healer-benchmark/stockade-seed1.intake.json");
    let mut intake: Intake = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut hit = intake.hits[0].clone();
    hit.raw = 100.0;
    if let super::healer_intake::HitKind::Melee {
        ref mut outcome, ..
    } = hit.kind
    {
        *outcome = "Parry".into();
    } else {
        panic!("recorded first hit must be melee");
    }
    intake.end_tick = hit.tick;
    intake.hits = vec![hit];
    let report = super::tank_replay::replay(&intake).unwrap();
    assert_eq!(report["raw_damage"], 100.0);
    assert_eq!(report["damage_taken"], 0.0);
    assert_eq!(report["mitigation_pct"], 100.0);
}

#[test]
#[ignore = "uses native recipient avoidance and the SimC dummy strike policy"]
fn simc_dummy_melee_uses_recipient_avoidance_and_strikes_can_block() {
    use crate::melee::MeleeOutcome;
    let _rng = crate::random::seed_scope(1);
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[], 5).unwrap();
    let mut stats = h
        .app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap();
    stats.defense.dodge = 0.0;
    stats.defense.parry = 100.0;
    stats.defense.block = 100.0;
    drop(stats);
    let mut saw_parry = false;
    for _ in 0..100 {
        let (swing, _) =
            crate::combat::roll_simc_dummy_outcome(h.app.world(), h.npc, h.caster, false, 0.0, 0.0)
                .unwrap();
        assert_eq!(swing, MeleeOutcome::Parry);
        saw_parry |= swing == MeleeOutcome::Parry;
        let (strike, _) =
            crate::combat::roll_simc_dummy_outcome(h.app.world(), h.npc, h.caster, true, 0.0, 0.0)
                .unwrap();
        assert_eq!(strike, MeleeOutcome::Block);
    }
    assert!(saw_parry);
    let mut stats = h
        .app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap();
    stats.defense.parry = 0.0;
    stats.defense.block = 0.0;
    drop(stats);
    for _ in 0..100 {
        assert_eq!(
            crate::combat::roll_simc_dummy_outcome(h.app.world(), h.npc, h.caster, false, 1.0, 0.0)
                .unwrap()
                .0,
            MeleeOutcome::Miss
        );
        assert_eq!(
            crate::combat::roll_simc_dummy_outcome(h.app.world(), h.npc, h.caster, false, 0.0, 1.0)
                .unwrap()
                .0,
            MeleeOutcome::Crit
        );
    }
}

#[test]
#[ignore = "uses MID2 plate tank profiles and the native stat pipeline"]
fn plate_tank_profiles_receive_simc_base_dodge() {
    for spec in [66, 73] {
        let run = super::dps_parity::prepare_registered_player(spec, 1).unwrap();
        let dodge = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .defense
            .dodge;
        assert!((dodge - 3.0).abs() < 0.01, "spec {spec} base dodge {dodge}");
    }
}

#[test]
#[ignore = "runs all six native MID2 tank dummy replays"]
fn all_tanks_use_dungeon_season_combat_inputs() {
    let directory = Path::new(
        "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/healer-tank-replay",
    );
    for spec in [66, 73, 104, 250, 268, 581] {
        let trace = directory.join(format!("{spec}.simc-trace.log"));
        let report =
            super::tank_replay::replay_simc_with_healing(spec, &trace, 300, 70448, true).unwrap();
        assert_eq!(report["spec"], spec, "{report}");
        assert_eq!(
            report["tank_dummy_scenario"]["attacker_level"], 92,
            "{report}"
        );
        let k = report["tank_dummy_scenario"]["armor_constant"]
            .as_f64()
            .unwrap();
        assert!((k - 4531.03).abs() < 0.01, "spec {spec} K={k}");
        assert!(report["raw_damage"].as_f64().unwrap() > 0.0, "{report}");
    }
}

#[test]
#[ignore = "uses native player defense and actual melee outcome rolls"]
fn higher_level_attacker_reduces_player_avoidance() {
    use crate::melee::MeleeOutcome;
    use shared::components::UnitLevel;
    let _rng = crate::random::seed_scope(70448);
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[], 12).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert(UnitLevel(90));
    h.app.world_mut().entity_mut(h.npc).insert(UnitLevel(92));
    let mut stats = h
        .app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap();
    stats.defense.dodge = 3.0;
    stats.defense.parry = 3.0;
    stats.defense.block = 0.0;
    drop(stats);
    for _ in 0..1000 {
        let (outcome, difference) =
            crate::combat::roll_simc_dummy_outcome(h.app.world(), h.npc, h.caster, false, 0.0, 0.0)
                .unwrap();
        assert_eq!(difference, -2);
        assert_eq!(outcome, MeleeOutcome::Hit);
    }
    h.app.world_mut().entity_mut(h.npc).insert(UnitLevel(90));
    let avoided = (0..1000)
        .filter(|_| {
            crate::combat::roll_simc_dummy_outcome(h.app.world(), h.npc, h.caster, false, 0.0, 0.0)
                .unwrap()
                .0
                .avoided()
        })
        .count();
    assert!(avoided > 0, "equal-level attacks retain defense");
}

#[test]
#[ignore = "uses native ExpectedStat and real incoming damage"]
fn dungeon_season_armor_tuning_changes_incoming_swing_damage() {
    use shared::components::{Health, UnitLevel};
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[], 12).unwrap();
    let data = crate::test_data::world_data();
    h.app.insert_resource(data.armor_constants.clone());
    h.app.world_mut().entity_mut(h.caster).insert((
        UnitLevel(90),
        Health {
            current: 10000.0,
            max: 10000.0,
        },
    ));
    h.app.world_mut().entity_mut(h.npc).insert(UnitLevel(92));
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .armor = 3430.0;
    let hit = super::healer_intake::Hit {
        tick: 1,
        source_guid: 0,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 1000.0,
        level_scaling_factor: 1.0,
        kind: super::healer_intake::HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: -2,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    crate::combat::replay_healer_hit(h.app.world_mut(), h.npc, h.caster, &hit).unwrap();
    let ordinary = 10000.0 - h.health(h.caster);
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 10000.0;
    let tuning = data.expected_stats.armor_tuning(3025, 120).unwrap();
    h.app.world_mut().entity_mut(h.npc).insert(tuning);
    crate::combat::replay_healer_hit(h.app.world_mut(), h.npc, h.caster, &hit).unwrap();
    let dungeon = 10000.0 - h.health(h.caster);
    assert_eq!(ordinary, 500.0);
    assert_eq!(dungeon, 569.0);
}

#[test]
#[ignore = "uses native Punish aura and real incoming damage"]
fn punish_reduces_only_its_casters_incoming_swing_strike_and_bleed() {
    use shared::components::Health;
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[275335], 1).unwrap();
    h.app
        .insert_resource(crate::test_data::world_data().armor_constants.clone());
    let owner = h.caster;
    let source = h.npc;
    let other = crate::spell_cast::test_harness::spawn_player(&mut h.app, 1, 4.0);
    for target in [owner, other] {
        h.app.world_mut().entity_mut(target).insert(Health {
            current: 10_000.0,
            max: 10_000.0,
        });
        h.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(target)
            .unwrap()
            .versatility_pct = 20.0;
    }
    let hit = super::healer_intake::Hit {
        tick: 1,
        source_guid: 0,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 1000.0,
        level_scaling_factor: 1.0,
        kind: super::healer_intake::HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    for _ in 0..5 {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: owner,
                target: source,
                spell_id: 275335,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    h.advance(2);
    for target in [owner, other] {
        let expected = if target == owner { 765.0 } else { 900.0 };
        for kind in 0..3 {
            h.app.world_mut().get_mut::<Health>(target).unwrap().current = 10_000.0;
            match kind {
                0 => crate::combat::replay_healer_hit(h.app.world_mut(), source, target, &hit)
                    .unwrap(),
                1 => crate::spell_cast::effects::replay_simc_dummy_strike(
                    h.app.world_mut(),
                    source,
                    target,
                    1000.0,
                    false,
                    false,
                ),
                _ => crate::spell_cast::effects::replay_simc_dummy_bleed(
                    h.app.world_mut(),
                    source,
                    target,
                    1000.0,
                ),
            }
            let damage = 10_000.0 - h.health(target);
            assert_eq!(damage, expected, "kind={kind} target={target:?}");
        }
    }
    crate::auras::remove_auras(
        h.app.world_mut(),
        source,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 275335,
    );
    h.advance(2);
    h.app.world_mut().get_mut::<Health>(owner).unwrap().current = 10_000.0;
    crate::combat::replay_healer_hit(h.app.world_mut(), source, owner, &hit).unwrap();
    assert_eq!(10_000.0 - h.health(owner), 900.0, "removal restores damage");
}

#[test]
#[ignore = "uses native dungeon-season tuning and a real blocked hit"]
fn tank_report_block_reduction_matches_dungeon_combat() {
    use shared::components::{Health, UnitLevel};
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[], 12).unwrap();
    let data = crate::test_data::world_data();
    h.app.insert_resource(data.armor_constants.clone());
    h.app.world_mut().entity_mut(h.caster).insert((
        UnitLevel(90),
        Health {
            current: 100_000.0,
            max: 100_000.0,
        },
    ));
    h.app.world_mut().entity_mut(h.npc).insert((
        UnitLevel(92),
        data.expected_stats.armor_tuning(3025, 120).unwrap(),
    ));
    let mut stats = h
        .app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap();
    stats.armor = 0.0;
    stats.defense.shield_block = 3185.0;
    drop(stats);
    let reported = super::tank_replay::sampled_blocked_pct(h.app.world(), h.npc, h.caster).unwrap();
    let hit = super::healer_intake::Hit {
        tick: 1,
        source_guid: 0,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 10_000.0,
        level_scaling_factor: 1.0,
        kind: super::healer_intake::HitKind::Melee {
            hand: 0,
            outcome: "Block".into(),
            level_difference: -2,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    crate::combat::replay_healer_hit(h.app.world_mut(), h.npc, h.caster, &hit).unwrap();
    let actual = 100.0 * (h.health(h.caster) - 90_000.0) / 10_000.0;
    assert!((reported - 41.2777).abs() < 0.01, "reported={reported}");
    assert!(
        (reported - actual).abs() < 0.02,
        "reported={reported} actual={actual}"
    );
}

#[test]
#[ignore = "uses native recipient armor and absorb data"]
fn simc_dummy_bleed_uses_native_absorbs_but_not_armor() {
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[17], 5).unwrap();
    h.app
        .insert_resource(crate::test_data::world_data().armor_constants.clone());
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .armor = 5000.0;
    let before = h.health(h.caster);
    crate::spell_cast::effects::replay_simc_dummy_bleed(h.app.world_mut(), h.npc, h.caster, 200.0);
    assert_eq!(
        before - h.health(h.caster),
        200.0,
        "physical bleed bypasses armor"
    );
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 17,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let shielded = h.health(h.caster);
    crate::spell_cast::effects::replay_simc_dummy_bleed(h.app.world_mut(), h.npc, h.caster, 200.0);
    assert_eq!(
        h.health(h.caster),
        shielded,
        "native shield absorbs the bleed"
    );
}

#[test]
#[ignore = "requires the MID2 Protection Paladin profile and canonical world database"]
fn protection_paladin_uses_parity_player_profile_and_equipment() {
    let canonical = Path::new("/home/osso/Projects/world-of-osso/game-server/data/world.db")
        .canonicalize()
        .unwrap();
    assert_eq!(
        crate::test_data::world_db_path().canonicalize().unwrap(),
        canonical
    );
    assert_eq!(
        Path::new(&std::env::var("DPS_PARITY_WORLD_DB").unwrap())
            .canonicalize()
            .unwrap(),
        canonical
    );
    let (profile, traits) = super::dps_parity::read_reference(66).unwrap();
    assert_eq!(profile.level, "90");
    assert_eq!(profile.race, "tauren");
    let mut expected: Vec<_> = profile.equipment.iter().map(|item| item.item_id).collect();
    expected.sort_unstable();
    let run = super::dps_parity::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 6 },
    )
    .unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Level>(run.actor)
            .unwrap()
            .0,
        90
    );
    let mut actual: Vec<_> = run
        .app
        .world()
        .get::<crate::stats::EquippedItems>(run.actor)
        .unwrap()
        .items
        .iter()
        .map(|(_, _, id)| *id)
        .collect();
    actual.sort_unstable();
    assert_eq!(actual, expected);
    let loadout = run
        .app
        .world()
        .get::<crate::class_progression::TraitLoadout>(run.actor)
        .unwrap();
    assert!(
        loadout
            .active_talent_spells()
            .any(|spell| spell.spell_id == 433568)
    );
    let modifiers = run
        .app
        .world()
        .get::<crate::stats::EquippedItemModifiers>(run.actor)
        .unwrap();
    assert!(
        modifiers.temporary_enchants.is_empty(),
        "selected Rite must suppress the conditional oil"
    );
    assert_eq!(run.targets.len(), 1);
}

#[test]
#[ignore = "replays a recorded Stockade intake against the canonical world database"]
fn protection_paladin_replays_recorded_damage_without_healer() {
    let canonical = Path::new("/home/osso/Projects/world-of-osso/game-server/data/world.db")
        .canonicalize()
        .unwrap();
    assert_eq!(
        crate::test_data::world_db_path().canonicalize().unwrap(),
        canonical
    );
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/dungeons/healer-benchmark/stockade-seed1.intake.json");
    let intake: Intake = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let report = super::tank_replay::replay(&intake).unwrap();
    assert_eq!(report["spec"], 66);
    let raw = report["raw_damage"].as_f64().unwrap();
    let taken = report["damage_taken"].as_f64().unwrap();
    assert!(raw > 0.0 && taken > 0.0 && taken <= raw, "{report}");
    assert!(
        (report["mitigation_pct"].as_f64().unwrap() - 100.0 * (raw - taken) / raw).abs() < 1e-6
    );
    let duration = report["duration_seconds"].as_f64().unwrap();
    assert!(duration > 0.0);
    assert!((0.0..=1.0).contains(&report["active_mitigation_uptime"].as_f64().unwrap()));
    assert!(report["largest_spike"].as_f64().unwrap() > 0.0);
    assert!(
        !report["tank_casts"].as_object().unwrap().is_empty(),
        "{report}"
    );
    if !report["time_to_death"].is_null() {
        assert_eq!(report["time_to_death"].as_f64().unwrap(), duration);
    }
    println!("TANK_REPLAY {report}");
}

#[test]
#[ignore = "uses selected MID2 Protection player and real Avenger's Shield"]
fn vanguard_shield_triggers_delayed_glory_and_power() {
    use crate::auras::{AuraApplication, AuraSource};
    use crate::spell_cast::powers::PowerPools;
    use shared::components::PowerType;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..40 {
        run.app.update();
    }
    let power = |run: &super::dps_parity::Run| {
        run.app
            .world()
            .get::<PowerPools>(actor)
            .unwrap()
            .pools
            .iter()
            .find(|p| p.power == PowerType::HolyPower)
            .unwrap()
            .current
    };
    run.app
        .world_mut()
        .get_mut::<PowerPools>(actor)
        .unwrap()
        .pools
        .iter_mut()
        .find(|p| p.power == PowerType::HolyPower)
        .unwrap()
        .current = 0.0;
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 1268810,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 31935, Some(target), false)
        .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert_eq!(
        power(&run),
        0.0,
        "Glory must not grant power before its delay"
    );
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(
        power(&run),
        3.0,
        "delayed Glory grants selected talent Holy Power"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1269179),
        "delayed Glory must grant Valor"
    );
}

#[test]
#[ignore = "uses selected MID2 Protection player and real Shield of the Righteous"]
fn valor_empowers_one_shield_and_blazes_secondary_targets() {
    use crate::auras::{AuraApplication, AuraSource};
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;

    fn shield(valor_stacks: u8) -> (i32, bool, Vec<u64>, u64) {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::dps_parity::prepare_registered_player(66, 2).unwrap();
        super::dps_parity::engage(&mut run);
        for _ in 0..40 {
            run.app.update();
        }
        let actor = run.actor;
        let target = run.targets[0];
        for _ in 0..valor_stacks {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id: 1269179,
                    effect_mask: 1,
                    source: AuraSource::Cast,
                },
            );
        }
        let _hit_seed = crate::random::seed_scope(454545);
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            53600,
            Some(target),
            0
        ));
        for _ in 0..10 {
            run.app.update();
        }
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        let damage = events
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(53600)
                    && event.kind == CombatLogKind::Damage
            })
            .expect("shield hit")
            .amount;
        let blaze = events
            .iter()
            .filter(|event| event.spell_id == Some(1269224) && event.kind == CombatLogKind::Damage)
            .filter_map(|event| event.target)
            .collect();
        (
            damage,
            crate::auras::has_aura(run.app.world(), actor, 1269179),
            blaze,
            run.targets[1].to_bits(),
        )
    }
    let (normal, _, no_blaze, _) = shield(0);
    let (empowered, remains, blaze, secondary) = shield(1);
    let (two_stacks, two_remains, _, _) = shield(2);
    assert!(
        (two_stacks as f32 / normal as f32 - 3.0).abs() < 0.01,
        "{normal} vs {two_stacks}"
    );
    assert!(!two_remains, "Shield consumes all Valor stacks");
    assert!(
        (empowered as f32 / normal as f32 - 2.0).abs() < 0.01,
        "{normal} vs {empowered}"
    );
    assert!(!remains, "Shield consumes Valor");
    assert!(no_blaze.is_empty());
    assert_eq!(
        blaze,
        vec![secondary],
        "Blaze hits the secondary, not the primary"
    );
}

#[test]
#[ignore = "uses real MID2 Protection Blessed Hammer and dummy attacks"]
fn blessed_hammer_absorbs_only_next_hit_from_its_target() {
    use shared::components::Health;
    let mut run = super::dps_parity::prepare_registered_player(66, 2).unwrap();
    super::dps_parity::engage(&mut run);
    for _ in 0..40 {
        run.app.update();
    }
    let actor = run.actor;
    let attacker = run.targets[0];
    let other = run.targets[1];
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 432607,
    );
    let damage = |run: &mut super::dps_parity::Run, source| {
        let before = run.app.world().get::<Health>(actor).unwrap().current;
        crate::spell_cast::effects::replay_simc_dummy_bleed(
            run.app.world_mut(),
            source,
            actor,
            25000.0,
        );
        before - run.app.world().get::<Health>(actor).unwrap().current
    };
    let normal = damage(&mut run, attacker);
    assert!(normal > 0.0, "baseline hit must reach health");
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        204301,
        Some(attacker),
        0
    ));
    let other_damage = damage(&mut run, other);
    let reduced = damage(&mut run, attacker);
    let next = damage(&mut run, attacker);
    assert!(
        (other_damage - normal).abs() < 2.0,
        "other attacker consumed the debuff: {normal} {other_damage}"
    );
    let ap = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .attack_power;
    assert!(
        (normal - reduced - 0.30 * ap).abs() < 3.0,
        "expected AP-scaled absorb: {normal} {reduced} AP {ap}"
    );
    assert!(
        (next - normal).abs() < 2.0,
        "absorb must be one hit: {normal} {next}"
    );
}

#[test]
#[ignore = "uses selected MID2 Protection and real Avenger's Shield"]
fn avengers_shield_grants_native_strength_in_adversity_parry() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    for _ in 0..40 {
        run.app.update();
    }
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        31935,
        Some(target),
        0
    ));
    for _ in 0..4 {
        run.app.update();
    }
    let after = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .defense
        .parry;
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 393038)
        .expect("real parry aura");
    assert_eq!(aura.stacks, 1);
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 393038,
    );
    run.app.update();
    let before = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .defense
        .parry;
    assert!(
        (after - before - 3.0).abs() < 0.01,
        "parry {before} -> {after}"
    );
}

#[test]
#[ignore = "uses selected MID2 Protection and real multi-target Avenger's Shield"]
fn strength_in_adversity_scales_with_actual_shield_targets() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 3).unwrap();
    super::dps_parity::engage(&mut run);
    for _ in 0..40 {
        run.app.update();
    }
    let actor = run.actor;
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        actor,
        31935,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 393038)
        .expect("parry aura");
    let hits = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(31935)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(
        aura.stacks as usize, hits,
        "each actual Shield hit grants a stack"
    );
    assert_eq!(hits, 3, "the fixture must exercise three Shield targets");
}

#[test]
#[ignore = "uses selected MID2 Protection and the real Devotion Aura"]
fn devotion_aura_reduces_incoming_bleed() {
    use shared::components::Health;
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    for _ in 0..40 {
        run.app.update();
    }
    let actor = run.actor;
    let source = run.targets[0];
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap()
            .contains(465)
    );
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 465),
        "precombat must cast Devotion Aura"
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 432607 || aura.spell_id == 465 || aura.spell_id == 344218,
    );
    let hit = |run: &mut super::dps_parity::Run| {
        let before = run.app.world().get::<Health>(actor).unwrap().current;
        crate::spell_cast::effects::replay_simc_dummy_bleed(
            run.app.world_mut(),
            source,
            actor,
            25000.0,
        );
        before - run.app.world().get::<Health>(actor).unwrap().current
    };
    let before = hit(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 465, Some(actor), false).unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    let after = hit(&mut run);
    assert!(
        before > 0.0 && after < before * 0.98,
        "Devotion Aura did not reduce bleed: {before} -> {after}"
    );
}

#[test]
#[ignore = "uses the selected Protection set and real Consecration field"]
fn protection_two_piece_illuminates_enemies_in_consecration() {
    use shared::components::Position;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 1296658));
    let chance = |world: &bevy::prelude::World| {
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(275779)
            .unwrap();
        crate::spell_cast::effects::crit_chance(world, actor, target, spell)
    };
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        crate::auras::RemoveReason::Expired,
        |aura| aura.spell_id == 204242 && aura.caster == actor,
    );
    let before = chance(run.app.world());
    let consecration = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(26573)
        .unwrap()
        .clone();
    crate::spell_cast::scripts::spawn_consecration(run.app.world_mut(), actor, &consecration);
    for _ in 0..30 {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), target, 204242));
    let illuminated = chance(run.app.world());
    assert!(
        (illuminated - before - 5.0).abs() < 0.01,
        "before={before} illuminated={illuminated}"
    );
    run.app.world_mut().get_mut::<Position>(target).unwrap().x += 100.0;
    run.app.update();
    assert!(!crate::auras::has_aura(run.app.world(), target, 204242));
    assert!((chance(run.app.world()) - before).abs() < 0.01);
}

#[test]
#[ignore = "uses selected MID2 Focused Enmity and real Avenger's Shield impacts"]
fn focused_enmity_counts_engaged_enemies_not_chain_targets() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::Position;
    use shared::protocol::CombatLogKind;

    fn shield(second_engaged: bool) -> f64 {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::dps_parity::prepare_registered_player(66, 2).unwrap();
        super::dps_parity::engage(&mut run);
        let actor = run.actor;
        let primary = run.targets[0];
        let secondary = run.targets[1];
        run.app
            .world_mut()
            .get_mut::<Position>(secondary)
            .unwrap()
            .x += 100.0;
        if !second_engaged {
            crate::combat::combat_stop(run.app.world_mut(), secondary);
        }
        let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
        let _hit_seed = crate::random::seed_scope(454545);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 31935, Some(primary), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let events = &run.app.world().resource::<ObservedCombatEvents>().0[start..];
        let hit = events
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(primary.to_bits())
                    && event.spell_id == Some(31935)
                    && event.kind == CombatLogKind::Damage
            })
            .expect("Shield must hit primary");
        f64::from(hit.amount) / if hit.crit { 2.0 } else { 1.0 }
    }
    let single = shield(false);
    let multiple = shield(true);
    assert!(
        (1.99..2.01).contains(&(single / multiple)),
        "single={single} multiple={multiple}"
    );
}

#[test]
#[ignore = "uses selected MID2 Ferren Marcus and real Avenger's Shield chain"]
fn ferren_marcus_empowers_only_primary_shield_target() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 2).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    let secondary = run.targets[1];
    let rank = crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 378762, 0);
    assert!((rank - 20.0).abs() < 0.01, "selected Fervor rank: {rank}");
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let _hit_seed = crate::random::seed_scope(454545);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 31935, Some(primary), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[start..];
    let damage = |target| {
        let hit = events
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target)
                    && event.spell_id == Some(31935)
                    && event.kind == CombatLogKind::Damage
            })
            .expect("Shield must hit both chain targets");
        f64::from(hit.amount) / if hit.crit { 2.0 } else { 1.0 }
    };
    let first = damage(primary.to_bits());
    let second = damage(secondary.to_bits());
    // Native per-target damage variance is independently rolled for each impact.
    assert!(
        (1.12..1.28).contains(&(first / second)),
        "primary={first} secondary={second}"
    );
}

#[test]
#[ignore = "uses selected MID2 Greater Judgment and real Protection spell impacts"]
fn protection_judgment_empowers_and_is_consumed_by_shield() {
    use crate::spell_cast::powers::PowerPools;
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::PowerType;
    use shared::protocol::CombatLogKind;
    fn shield(with_mark: bool) -> i32 {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
        super::dps_parity::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 275779, Some(target), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let aura = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 197277 && aura.caster == actor)
            .expect("Judgment must mark its target");
        assert!(
            aura.effects
                .iter()
                .any(|effect| effect.index == 0 && (effect.amount - 50.0).abs() < 0.01)
        );
        let before_stacks = aura.stacks;
        if !with_mark {
            crate::auras::remove_auras(
                run.app.world_mut(),
                target,
                crate::auras::RemoveReason::Consumed,
                |aura| aura.spell_id == 197277 && aura.caster == actor,
            );
        }
        run.app
            .world_mut()
            .get_mut::<PowerPools>(actor)
            .unwrap()
            .get_mut(PowerType::HolyPower)
            .unwrap()
            .current = 3.0;
        let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
        let _hit_seed = crate::random::seed_scope(454545);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 53600, Some(target), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        if with_mark {
            let after = run
                .app
                .world()
                .get::<crate::auras::ActiveAuras>(target)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 197277 && aura.caster == actor)
                .map_or(0, |aura| aura.stacks);
            assert_eq!(
                after + 1,
                before_stacks,
                "Shield must consume one Judgment use"
            );
        }
        run.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(53600)
                    && event.kind == CombatLogKind::Damage
            })
            .expect("Shield must damage its target")
            .amount
    }
    let without = shield(false);
    let with = shield(true);
    assert!(
        without > 0 && (with as f64 / without as f64 - 1.5).abs() < 0.01,
        "Judgment must increase Shield damage by 50%: {without} -> {with}"
    );
}

#[test]
#[ignore = "uses selected MID2 Bulwark and real Shield casts"]
fn avengers_shield_grants_bulwark_for_the_next_shield() {
    use crate::spell_cast::powers::PowerPools;
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::PowerType;
    use shared::protocol::CombatLogKind;
    fn shield(with_bulwark: bool) -> i32 {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
        super::dps_parity::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 31935, Some(target), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let aura = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 386652)
            .expect("Avenger's Shield must grant Bulwark");
        assert_eq!(aura.stacks, 1);
        if !with_bulwark {
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Consumed,
                |aura| aura.spell_id == 386652,
            );
        }
        run.app
            .world_mut()
            .get_mut::<PowerPools>(actor)
            .unwrap()
            .get_mut(PowerType::HolyPower)
            .unwrap()
            .current = 3.0;
        let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
        let _hit_seed = crate::random::seed_scope(454545);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 53600, Some(target), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 386652),
            "Shield must consume Bulwark"
        );
        run.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(53600)
                    && event.kind == CombatLogKind::Damage
            })
            .expect("Shield must damage its target")
            .amount
    }
    let without = shield(false);
    let with = shield(true);
    assert!(
        without > 0 && (with as f64 / without as f64 - 1.1).abs() < 0.01,
        "one Bulwark stack must add 10% Shield damage: {without} -> {with}"
    );
}

#[test]
#[ignore = "uses selected MID2 Instrument and real Shield of the Righteous damage"]
fn instrument_scales_shield_damage_from_pre_spend_holy_power() {
    use crate::spell_cast::powers::PowerPools;
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::PowerType;
    use shared::protocol::CombatLogKind;
    fn shield(power: f32) -> i32 {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
        super::dps_parity::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        run.app
            .world_mut()
            .get_mut::<PowerPools>(actor)
            .unwrap()
            .get_mut(PowerType::HolyPower)
            .unwrap()
            .current = power;
        let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
        let _hit_seed = crate::random::seed_scope(454545);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 53600, Some(target), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        run.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(53600)
                    && event.kind == CombatLogKind::Damage
            })
            .expect("Shield must damage its target")
            .amount
    }
    let three = shield(3.0);
    let five = shield(5.0);
    assert!(
        three > 0 && (five as f64 / three as f64 - 2.0).abs() < 0.01,
        "two extra Holy Power must double Shield damage: {three} -> {five}"
    );
}

#[test]
#[ignore = "uses selected MID2 Protection and native Shield of the Righteous"]
fn protection_shield_uses_modified_armor_effect() {
    use crate::spell_cast::powers::PowerPools;
    use shared::components::PowerType;
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    run.app
        .world_mut()
        .get_mut::<PowerPools>(actor)
        .unwrap()
        .get_mut(PowerType::HolyPower)
        .unwrap()
        .current = 5.0;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 53600, Some(target), false)
        .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 132403)
        .expect("Shield must grant armor");
    let amount = aura
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    assert!(
        (amount - 192.0).abs() < 0.01,
        "Protection-modified Shield armor: {amount}"
    );
    let with = *run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap();
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 132403,
    );
    for _ in 0..4 {
        run.app.update();
    }
    let without = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap();
    assert!(
        with.armor - without.armor > with.primary.strength * 1.9,
        "Shield must add its modified Strength-based armor: {} -> {}",
        without.armor,
        with.armor
    );
}

#[test]
#[ignore = "uses selected MID2 Protection and real incoming magic damage"]
fn protection_shield_reduces_real_magic_damage() {
    use super::healer_intake::{Hit, HitKind};
    use crate::spell_cast::powers::PowerPools;
    use shared::components::{Health, PowerType};
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let source = run.targets[0];
    run.app
        .world_mut()
        .get_mut::<PowerPools>(actor)
        .unwrap()
        .get_mut(PowerType::HolyPower)
        .unwrap()
        .current = 5.0;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 53600, Some(source), false)
        .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| [432607, 1239002, 204301].contains(&aura.spell_id),
    );
    let hit = Hit {
        tick: 0,
        source_guid: 0,
        pack: 0,
        target_slot: 0,
        school: 4,
        raw: 10000.0,
        level_scaling_factor: 1.0,
        kind: HitKind::Spell {
            spell_id: 133,
            depth: 0,
            bonus_source_guid: None,
            effect_index: None,
            periodic: false,
            crit: false,
            resolved_source: true,
            blocked: false,
        },
    };
    let damage = |run: &mut super::dps_parity::Run| {
        let max = run.app.world().get::<Health>(actor).unwrap().max;
        run.app
            .world_mut()
            .get_mut::<Health>(actor)
            .unwrap()
            .current = max;
        crate::spell_cast::effects::replay_healer_hit(run.app.world_mut(), source, actor, &hit)
            .unwrap();
        max - run.app.world().get::<Health>(actor).unwrap().current
    };
    let with = damage(&mut run);
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 132403,
    );
    for _ in 0..4 {
        run.app.update();
    }
    let without = damage(&mut run);
    assert!(
        without > 0.0 && (with / without - 0.88).abs() < 0.005,
        "magic: {with}/{without}"
    );
}

#[test]
#[ignore = "uses selected MID2 Protection and native Rite of Sanctification"]
fn protection_rite_grants_native_strength_and_armor() {
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let items = run
        .app
        .world()
        .get::<crate::stats::EquippedItemModifiers>(actor)
        .unwrap();
    assert!(
        items
            .temporary_enchants
            .contains(&(shared::protocol::EquipmentSlot::MainHand, 7143))
    );
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 433550)
        .expect("Rite must grant its enchant aura");
    assert!(
        aura.effects
            .iter()
            .any(|effect| effect.index == 0 && (effect.amount - 2.0).abs() < 0.01)
    );
    assert!(
        aura.effects
            .iter()
            .any(|effect| effect.index == 1 && (effect.amount - 5.0).abs() < 0.01)
    );
    let with = *run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap();
    run.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItemModifiers>(actor)
        .unwrap()
        .temporary_enchants
        .retain(|(_, enchant)| *enchant != 7143);
    for _ in 0..4 {
        run.app.update();
    }
    assert!(!crate::auras::has_aura(run.app.world(), actor, 433550));
    let without = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap();
    assert!(
        with.primary.strength > without.primary.strength,
        "Rite must increase strength"
    );
    assert!(with.armor > without.armor, "Rite must increase armor");
}

#[test]
#[ignore = "uses selected MID2 Protection and native Faith's Armor"]
fn protection_shield_grants_faiths_armor_with_native_modifier() {
    use crate::spell_cast::powers::PowerPools;
    use shared::components::PowerType;
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    run.app
        .world_mut()
        .get_mut::<PowerPools>(actor)
        .unwrap()
        .get_mut(PowerType::HolyPower)
        .unwrap()
        .current = 5.0;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 53600, Some(target), false)
        .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 379017)
        .expect("Shield must grant Faith's Armor");
    let effect = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    assert!(
        (effect.amount - 5.0).abs() < 0.01,
        "Protection modifies the native 20% to 5%: {}",
        effect.amount
    );
    let with = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .armor;
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 379017,
    );
    for _ in 0..4 {
        run.app.update();
    }
    let without = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .armor;
    assert!(
        with > without,
        "Faith's Armor must increase live armor: {without} -> {with}"
    );
}

#[test]
#[ignore = "uses real tank-dummy bleed and Protection player"]
fn tank_dummy_bleed_ignores_target_armor() {
    use shared::components::Health;
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let source = run.targets[0];
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 432607,
    );
    let mut damage = Vec::new();
    for armor in [0.0, 100_000.0] {
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .armor = armor;
        let mut health = run.app.world_mut().get_mut::<Health>(actor).unwrap();
        health.current = health.max;
        let before = health.current;
        crate::spell_cast::effects::replay_simc_dummy_bleed(
            run.app.world_mut(),
            source,
            actor,
            10_000.0,
        );
        damage.push(before - run.app.world().get::<Health>(actor).unwrap().current);
    }
    assert!(
        damage[0] > 0.0 && (damage[0] - damage[1]).abs() < 1.0,
        "zero/high armor bleed damage: {damage:?}"
    );
}

#[test]
#[ignore = "uses selected MID2 Protection and real Blessing of Dusk"]
fn dusk_reduces_real_bleed_damage_at_all_health_levels() {
    use shared::components::Health;
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let source = run.targets[0];
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1241945)
        .expect("selected Dusk aura");
    let maximum = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert!(
        (maximum - 20.0).abs() < 0.01,
        "Protection-modified Dusk maximum: {maximum}"
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 432607,
    );
    for (health_pct, expected) in [(1.0, 0.8), (0.5, 0.8), (0.1, 0.8)] {
        let max = run.app.world().get::<Health>(actor).unwrap().max;
        run.app
            .world_mut()
            .get_mut::<Health>(actor)
            .unwrap()
            .current = max * health_pct;
        let before = run.app.world().get::<Health>(actor).unwrap().current;
        crate::spell_cast::effects::replay_simc_dummy_bleed(
            run.app.world_mut(),
            source,
            actor,
            10_000.0,
        );
        let actual = before - run.app.world().get::<Health>(actor).unwrap().current;
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 1241945,
        );
        run.app
            .world_mut()
            .get_mut::<Health>(actor)
            .unwrap()
            .current = before;
        crate::spell_cast::effects::replay_simc_dummy_bleed(
            run.app.world_mut(),
            source,
            actor,
            10_000.0,
        );
        let baseline = before - run.app.world().get::<Health>(actor).unwrap().current;
        assert!(
            (actual / baseline - expected).abs() < 0.002,
            "health {health_pct}: {actual}/{baseline} expected {expected}"
        );
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 1241945,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::KnownSpell,
            },
        );
    }
}

#[test]
#[ignore = "uses selected MID2 Crusader's Judgment and native avoidance procs"]
fn grand_crusader_advances_shared_judgment_charge() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 275779, Some(target), false)
        .unwrap();
    let recharge = |run: &super::dps_parity::Run| {
        let history = run
            .app
            .world()
            .get::<crate::spell_cast::SpellHistory>(actor)
            .unwrap();
        crate::spell_cast::test_harness::charge_recovery(history, 1663).1
    };
    let before = recharge(&run).expect("Judgment must start recharging");
    for _ in 0..100 {
        crate::combat::replay_healer_hit(run.app.world_mut(), target, actor, &avoided_hit("Parry"))
            .unwrap();
        run.app.update();
        let after = recharge(&run);
        if after.is_some_and(|deadline| before - deadline > 2.9)
            || (after.is_none() && before - crate::spell_cast::now(run.app.world()) > 0.1)
        {
            return;
        }
    }
    panic!("Grand Crusader did not advance the shared charge by 3 seconds");
}

#[test]
#[ignore = "uses selected MID2 Protection Divine Purpose and real Holy Power spenders"]
fn protection_divine_purpose_procs_and_pays_for_next_shield() {
    use crate::spell_cast::powers::PowerPools;
    use shared::components::PowerType;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 223817));
    for _ in 0..50 {
        for _ in 0..40 {
            run.app.update();
        }
        run.app
            .world_mut()
            .get_mut::<PowerPools>(actor)
            .unwrap()
            .get_mut(PowerType::HolyPower)
            .unwrap()
            .current = 3.0;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 53600, Some(target), false)
            .unwrap();
        if crate::auras::has_aura(run.app.world(), actor, 223819) {
            for _ in 0..40 {
                run.app.update();
            }
            run.app
                .world_mut()
                .get_mut::<PowerPools>(actor)
                .unwrap()
                .get_mut(PowerType::HolyPower)
                .unwrap()
                .current = 0.0;
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                53600,
                Some(target),
                false,
            )
            .expect("Divine Purpose makes the next shield free");
            assert_eq!(
                run.app
                    .world()
                    .get::<PowerPools>(actor)
                    .unwrap()
                    .get(PowerType::HolyPower)
                    .unwrap()
                    .current,
                0.0
            );
            return;
        }
    }
    panic!("selected Divine Purpose never procced from 50 real spenders");
}

#[test]
#[ignore = "uses the approved MID2 Holy and Protection players and native armament spells"]
fn holy_armaments_generate_holy_power_only_for_holy() {
    use crate::spell_cast::powers::PowerPools;
    use shared::components::PowerType;
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let holy = super::dps_parity::spawn_second_reference_player(&mut run, 65).unwrap();
    let protection = run.actor;
    for (actor, expected) in [(protection, 0.0), (holy, 3.0)] {
        for spell in [432459, 432472] {
            run.app
                .world_mut()
                .get_mut::<PowerPools>(actor)
                .unwrap()
                .get_mut(PowerType::HolyPower)
                .unwrap()
                .current = 0.0;
            assert!(crate::spell_cast::try_cast_triggered(
                run.app.world_mut(),
                actor,
                spell,
                Some(actor),
                1
            ));
            for _ in 0..20 {
                run.app.update();
            }
            let actual = run
                .app
                .world()
                .get::<PowerPools>(actor)
                .unwrap()
                .get(PowerType::HolyPower)
                .unwrap()
                .current;
            assert_eq!(actual, expected, "spec actor {actor:?}, spell {spell}");
        }
    }
}

#[test]
#[ignore = "uses selected MID2 Protection and real Divine Toll target dispatch"]
fn protection_divine_toll_launches_shield_and_generates_power() {
    use crate::spell_cast::powers::PowerPools;
    use shared::components::PowerType;
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..40 {
        run.app.update();
    }
    run.app
        .world_mut()
        .get_mut::<PowerPools>(actor)
        .unwrap()
        .get_mut(PowerType::HolyPower)
        .unwrap()
        .current = 0.0;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 375576, Some(target), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let power = run
        .app
        .world()
        .get::<PowerPools>(actor)
        .unwrap()
        .get(PowerType::HolyPower)
        .unwrap()
        .current;
    assert_eq!(
        power, 1.0,
        "Toll gains one Holy Power for the selected enemy"
    );
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(31935)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0),
        "Toll's shield must damage the enemy"
    );
}

#[test]
#[ignore = "uses real MID2 Protection and three engaged Toll targets"]
fn protection_divine_toll_grants_power_and_shield_per_target() {
    use crate::spell_cast::powers::PowerPools;
    use shared::components::PowerType;
    let mut run = super::dps_parity::prepare_registered_player(66, 3).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    for _ in 0..40 {
        run.app.update();
    }
    run.app
        .world_mut()
        .get_mut::<PowerPools>(actor)
        .unwrap()
        .get_mut(PowerType::HolyPower)
        .unwrap()
        .current = 0.0;
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        actor,
        375576,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let power = run
        .app
        .world()
        .get::<PowerPools>(actor)
        .unwrap()
        .get(PowerType::HolyPower)
        .unwrap()
        .current;
    assert_eq!(power, 3.0);
    let stacks = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 393038)
        .expect("Toll shield parry aura")
        .stacks;
    assert_eq!(
        stacks, 3,
        "the final Toll Shield replaces earlier casts with its three targets"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(31935)
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && event.amount > 0),
            "missing shield damage on {target:?}"
        );
    }
}

#[test]
#[ignore = "uses actual selected Divine Inspiration and repeated native Judgment hits"]
fn divine_inspiration_earns_alternating_self_armaments() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 432964));
    let before_procs = super::tank_replay::bulwark_observations(run.app.world(), actor).2;
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..200 {
        for _ in 0..60 {
            run.app.update();
        }
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            actor,
            275779,
            Some(target),
            false,
        ) {
            Ok(())
            | Err(
                crate::spell_cast::CastFailure::NoCharges
                | crate::spell_cast::CastFailure::OnCooldown,
            ) => {}
            Err(error) => panic!("Judgment refused: {error:?}"),
        }
    }
    for _ in 0..20 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let applications: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(actor.to_bits())
                && matches!(event.spell_id, Some(432496 | 432502))
                && matches!(
                    event.kind,
                    shared::protocol::CombatLogKind::AuraApplied
                        | shared::protocol::CombatLogKind::AuraRefreshed
                )
        })
        .filter_map(|event| event.spell_id)
        .collect();
    assert!(
        applications.len() >= 2,
        "actual hits must earn at least two inspirations: {applications:?}"
    );
    assert!(
        applications.windows(2).all(|pair| pair[0] != pair[1]),
        "inspiration must alternate: {applications:?}"
    );
    let procs = super::tank_replay::bulwark_observations(run.app.world(), actor).2 - before_procs;
    assert_eq!(procs as usize, applications.len());
}

#[test]
#[ignore = "uses real Protection Blessed Hammer field and native damage effects"]
fn blessed_hammer_field_hits_nearby_enemies_twice_on_schedule() {
    use shared::components::Position;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 2).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let near = run.targets[0];
    let far = run.targets[1];
    for _ in 0..40 {
        run.app.update();
    }
    let center = *run.app.world().get::<Position>(actor).unwrap();
    run.app.world_mut().entity_mut(near).insert(Position {
        x: center.x + 3.0,
        ..center
    });
    run.app.world_mut().entity_mut(far).insert(Position {
        x: center.x + 20.0,
        ..center
    });
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 204019, Some(near), false)
        .unwrap();
    let count = |run: &super::dps_parity::Run, target: bevy::prelude::Entity| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[start..]
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(204301)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count()
    };
    for _ in 0..20 {
        run.app.update();
    }
    assert_eq!(
        count(&run, near),
        0,
        "no instant hit before the first pulse"
    );
    for _ in 0..15 {
        run.app.update();
    }
    assert_eq!(count(&run, near), 1, "first pulse at 1.25 seconds");
    for _ in 0..60 {
        run.app.update();
    }
    assert_eq!(count(&run, near), 2, "second pulse at 3.75 seconds");
    assert_eq!(count(&run, far), 0, "outside radius must not be hit");
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(count(&run, near), 2, "field expires without a third pulse");
}

#[test]
#[ignore = "uses selected Divine Inspiration and real Blessed Hammer casts"]
fn blessed_hammer_can_earn_divine_inspiration() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..200 {
        for _ in 0..60 {
            run.app.update();
        }
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            actor,
            204019,
            Some(target),
            false,
        ) {
            Ok(())
            | Err(
                crate::spell_cast::CastFailure::NoCharges
                | crate::spell_cast::CastFailure::OnCooldown,
            ) => {}
            Err(error) => panic!("Blessed Hammer refused: {error:?}"),
        }
    }
    for _ in 0..20 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let applications: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(actor.to_bits())
                && matches!(event.spell_id, Some(432496 | 432502))
                && matches!(
                    event.kind,
                    shared::protocol::CombatLogKind::AuraApplied
                        | shared::protocol::CombatLogKind::AuraRefreshed
                )
        })
        .filter_map(|event| event.spell_id)
        .collect();
    assert!(
        applications.len() >= 2,
        "actual Blessed Hammer casts must earn Inspirations: {applications:?}"
    );
    assert!(
        applications.windows(2).all(|pair| pair[0] != pair[1]),
        "armaments must alternate: {applications:?}"
    );
}

#[test]
#[ignore = "uses equipped MID2 Protection set and real generator impacts"]
fn protection_mid2_edict_follows_generator_hits_only_with_four_piece() {
    use crate::auras::{ActiveAuras, AuraSource, RemoveReason};
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 1296659 && matches!(aura.source, AuraSource::ItemSet(_))),
        "the profile must equip the real four-piece"
    );
    let count = |run: &super::dps_parity::Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1300662)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count()
    };
    for (spell, expected) in [(275779, 1), (1241413, 1), (204019, 2)] {
        for _ in 0..80 {
            run.app.update();
        }
        let before = count(&run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, spell, Some(target), false)
            .unwrap();
        for _ in 0..100 {
            run.app.update();
        }
        assert_eq!(count(&run) - before, expected, "source spell {spell}");
    }
    crate::auras::remove_auras(run.app.world_mut(), actor, RemoveReason::Consumed, |aura| {
        aura.spell_id == 1296659 && matches!(aura.source, AuraSource::ItemSet(_))
    });
    for _ in 0..80 {
        run.app.update();
    }
    let before = count(&run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 275779, Some(target), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(
        count(&run),
        before,
        "without the equipped set aura no Edict can be earned"
    );
}

#[test]
#[ignore = "uses real triggered Edict damage and selected native Inspiration"]
fn unrelenting_edict_impacts_can_earn_divine_inspiration() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::dps_parity::prepare_registered_player(66, 1).unwrap();
    super::dps_parity::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..200 {
        for _ in 0..60 {
            run.app.update();
        }
        crate::spell_cast::cast_triggered_with_effect0_amount(
            run.app.world_mut(),
            actor,
            1300662,
            Some(target),
            1,
            100.0,
        );
    }
    for _ in 0..20 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let applications: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(actor.to_bits())
                && matches!(event.spell_id, Some(432496 | 432502))
                && matches!(
                    event.kind,
                    shared::protocol::CombatLogKind::AuraApplied
                        | shared::protocol::CombatLogKind::AuraRefreshed
                )
        })
        .filter_map(|event| event.spell_id)
        .collect();
    assert!(
        applications.len() >= 2,
        "real Edict hits must earn Inspirations: {applications:?}"
    );
    assert!(applications.windows(2).all(|pair| pair[0] != pair[1]));
}
