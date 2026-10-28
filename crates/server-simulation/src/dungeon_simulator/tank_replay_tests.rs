use super::healer_intake::Intake;
use std::path::Path;

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
        |aura| matches!(aura.spell_id, 77535 | 207203 | 326809),
    );
    let max = run.app.world().get::<Health>(target).unwrap().max;
    run.app
        .world_mut()
        .get_mut::<Health>(target)
        .unwrap()
        .current = max * 0.30;
    crate::spell_cast::effects::replay_simc_dummy_strike(
        run.app.world_mut(),
        source,
        target,
        max * 0.05,
        false,
        false,
    );
    run.app.update();
    assert!(
        crate::auras::has_aura(run.app.world(), target, 326808),
        "landed melee ability must proc Rune of Sanguination"
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
        let (_, absorbed) = crate::auras::absorb_damage(run.app.world_mut(), actor, 1, 1_000.0);
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
    let (_, consumed) = crate::auras::absorb_damage(run.app.world_mut(), run.actor, 1, max * 0.01);
    assert!((consumed - max * 0.01).abs() < 2.0);
    for _ in 0..40 {
        run.app.update();
    }
    let (_, remaining) = crate::auras::absorb_damage(run.app.world_mut(), run.actor, 1, max);
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
    let (_, absorbed) = crate::auras::absorb_damage(run.app.world_mut(), run.actor, 1, max);
    assert!(
        (absorbed - max * 0.30).abs() < 2.0,
        "{absorbed} != {}",
        max * 0.30
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
#[ignore = "uses selected MID2 Protection and real Blessing of Dusk"]
fn dusk_scales_real_bleed_damage_with_current_health() {
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
    for (health_pct, expected) in [(1.0, 1.0), (0.5, 0.9), (0.1, 0.82)] {
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
