//! Native Affliction literal fixture qualification using the pinned MID2 profile.
use super::*;

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_wither_landed_ticks_earn_native_nightfall() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    assert!(crate::auras::has_aura(run.app.world(), caster, 108558));
    let mut earned = false;
    for _ in 0..10 {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster,
                target,
                spell_id: 445474,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        for _ in 0..100 {
            run.app.update();
        }
        earned = crate::auras::has_aura(run.app.world(), caster, 264571);
        if earned {
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
            .any(|event| event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(445474)
                && event.periodic
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0)
    );
    // SimC76f8118 init1190–1225: actual Wither replaces Corruption in this threshold stream.
    assert!(
        earned,
        "actual owned Wither ticks must earn selected native Nightfall"
    );
    let before = super::affliction_nightfall_tests::uses(&run);
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, 686, Some(target), false)
        .unwrap();
    assert_eq!(
        super::affliction_nightfall_tests::uses(&run),
        before - 1,
        "one admitted instant Bolt spends one earned use, not every stored use"
    );
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none(),
        "earned native Nightfall makes the ordinary Shadow Bolt instant"
    );
    for _ in 0..20 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(686)
                && event.amount > 0
                && event.kind == shared::protocol::CombatLogKind::Damage
        }),
        "the admitted instant Shadow Bolt must actually land"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_shadow_bolt_landed_hits_earn_native_shard_instability() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    assert!(crate::auras::has_aura(run.app.world(), caster, 1260264));
    for _ in 0..40 {
        crate::spell_cast::creature_cast(run.app.world_mut(), caster, 686, Some(target), false)
            .unwrap();
        for _ in 0..70 {
            run.app.update();
        }
        if crate::auras::has_aura(run.app.world(), caster, 1260269) {
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
            .any(|event| event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(686)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0)
    );
    // SimC76f8118 sc_warlock_actions.cpp887–890/init1242–1245: selected talent,
    // landed Shadow Bolt, native effect1 PRD chance and actual1260269 buff.
    assert!(
        crate::auras::has_aura(run.app.world(), caster, 1260269),
        "landed native Shadow Bolts must earn the selected Shard Instability buff"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, 1259790, Some(target), false)
        .expect("earned Instability must admit free native Unstable Affliction at zero shards");
    for _ in 0..10 {
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), caster, 1260269),
        "actual instant UA must consume its admitted Instability use"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_selected_malefic_grasp_projects_to_real_native_channel() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let caster = run.actor;
    let target = run.targets[0];
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(caster)
            .unwrap()
            .contains(1261149)
    );
    // SimC76f8118 init181–183/actions2100: selected1261149 enables channel1261153.
    let spell = crate::spell_cast::knowledge::resolve_actor_action(
        run.app.world(),
        caster,
        1261149,
    )
    .expect(
        "selected Malefic Grasp must resolve its native channel instead of its passive talent root",
    );
    assert_eq!(spell, 1261153);
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, 205180, Some(caster), true)
        .unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, spell, Some(target), false)
        .unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .unwrap()
            .cast_type,
        shared::casting::CastType::Channel
    );
    for _ in 0..60 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(1261153)
                && event.periodic
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0),
        "the selected talent must enable actual periodic channel damage"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_agony_landed_ticks_energize_native_whole_shards() {
    use shared::components::PowerType;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    assert_eq!(
        crate::spell_cast::powers::power_value(run.app.world(), caster, PowerType::SoulShards)
            .unwrap()
            .0,
        0.0
    );
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: 980,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for _ in 0..360 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let damage_ticks = events
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(980)
                && event.periodic
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        damage_ticks >= 12,
        "actual landed Agony ticks are the prerequisite: {damage_ticks}"
    );
    let shards =
        crate::spell_cast::powers::power_value(run.app.world(), caster, PowerType::SoulShards)
            .unwrap()
            .0;
    // SimC76f8118 actions1479–1486: each threshold proc pays one displayed shard;
    // native17941 effect0 supplies10raw units, not one Destruction fragment.
    assert!(
        shards >= 10.0 && shards <= 50.0 && shards % 10.0 == 0.0,
        "landed Agony must fund whole native shards: ticks={damage_ticks}, raw={shards}"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_sayaad_reacts_to_owner_damage_only_when_not_passive() {
    for react in [
        shared::protocol::REACT_ASSIST,
        shared::protocol::REACT_PASSIVE,
    ] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_measurement(265, 1).unwrap();
        let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
        run.app
            .world_mut()
            .get_mut::<crate::pet_control::PetCharm>(pet)
            .unwrap()
            .react = react;
        // Precombat Haunt may already have engaged an Assist pet.
        run.app
            .world_mut()
            .entity_mut(pet)
            .remove::<crate::combat::AutoAttack>();
        run.app
            .world_mut()
            .entity_mut(pet)
            .insert(crate::networking::CombatTarget(None));
        engage(&mut run);
        let (caster, target) = (run.actor, run.targets[0]);
        crate::spell_cast::cast_triggered(run.app.world_mut(), caster, 686, Some(target), 0);
        for _ in 0..40 {
            run.app.update();
        }
        let victim = run
            .app
            .world()
            .get::<crate::combat::AutoAttack>(pet)
            .map(|attack| attack.victim);
        assert_eq!(
            victim,
            (react == shared::protocol::REACT_ASSIST).then_some(target)
        );
    }
}

fn sayaad_whiplash_melee(owned: bool) -> (f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(265, 1).unwrap();
    let owner = run.actor;
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), owner).unwrap();
    let target = run.targets[0];
    engage(&mut run);
    run.app
        .world_mut()
        .get_mut::<crate::pet_control::PetCharm>(pet)
        .unwrap()
        .action_bar[3..7]
        .fill(0);
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 6360,
    );
    let aura_caster = if owned { pet } else { owner };
    for _ in 0..10 {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: aura_caster,
                target,
                spell_id: 6360,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == aura_caster && aura.spell_id == 6360)
        .unwrap();
    assert_eq!(aura.stacks, 10);
    let bonus = f64::from(
        aura.effects
            .iter()
            .find(|effect| effect.index == 1)
            .unwrap()
            .amount,
    ) * f64::from(aura.stacks)
        / 100.0;
    crate::pet_control::order_companion_pet(run.app.world_mut(), owner, Some(target));
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    for _ in 0..280 {
        run.app.update();
        let hits: Vec<_> = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(pet.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && !event.crit
                    && !event.glancing
                    && event.blocked == 0
                    && event.absorbed == 0
            })
            .map(|event| event.amount)
            .collect();
        if hits.len() >= 2 {
            return (
                hits.iter().map(|&amount| f64::from(amount)).sum::<f64>() / hits.len() as f64,
                bonus,
            );
        }
    }
    panic!("actual Sayaad must land two noncritical white hits");
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_sayaad_melee_respects_owned_whiplash() {
    let (foreign, _) = sayaad_whiplash_melee(false);
    let (owned, bonus) = sayaad_whiplash_melee(true);
    // SimC76f8118 sc_warlock_pets.cpp447–452: the owned actual target's
    // Whiplash stacks affect all Sayaad damage, including real white swings.
    let ratio = owned / foreign;
    assert!(
        (ratio - (1.0 + bonus)).abs() < 0.02,
        "owned Whiplash melee ratio {ratio}, native expected {}",
        1.0 + bonus
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_sayaad_melee_uses_owner_critical_chance() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(265, 1).unwrap();
    let owner = run.actor;
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), owner).unwrap();
    let target = run.targets[0];
    engage(&mut run);
    crate::pet_control::order_companion_pet(run.app.world_mut(), owner, Some(target));
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    for _ in 0..12000 {
        run.app.update();
        let world = run.app.world();
        let swings: Vec<_> = world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(pet.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == shared::protocol::CombatLogKind::Damage
            })
            .collect();
        if swings.len() < 150 {
            continue;
        }
        // SimC76f8118 pet.cpp217–220 inherits the owner's live critical chance.
        let actual = swings.iter().filter(|event| event.crit).count() as f32 / swings.len() as f32;
        let owner_crit = world
            .get::<crate::stats::CharacterStats>(owner)
            .unwrap()
            .crit_pct
            / 100.0;
        assert!(
            (actual - owner_crit).abs() < 0.1,
            "actual Sayaad crit {actual}, owner crit {owner_crit}"
        );
        return;
    }
    panic!("actual Sayaad must land 150 melee swings");
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_sayaad_melee_inherits_live_owner_power() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(265, 1).unwrap();
    let owner = run.actor;
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), owner).unwrap();
    let target = run.targets[0];
    engage(&mut run);
    crate::pet_control::order_companion_pet(run.app.world_mut(), owner, Some(target));
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    for _ in 0..600 {
        run.app.update();
        let world = run.app.world();
        let event = world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.source == Some(pet.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && !event.crit
                    && !event.glancing
                    && event.blocked == 0
                    && event.absorbed == 0
            });
        let Some(event) = event else { continue };
        let owner_sp = world
            .get::<crate::stats::CharacterStats>(owner)
            .unwrap()
            .spell_power;
        // SimC76f8118 pets.cpp421–430: Sayaad AP is 2.565 times live owner SP.
        let contribution = owner_sp * 2.565 / 3.5 * 2.0;
        let mods = world.get::<crate::auras::AuraStatMods>(pet);
        let done = mods.map_or(1.0, |mods| {
            mods.damage_done(1) * mods.autoattack_damage_mult
        });
        let taken = world
            .get::<crate::auras::AuraStatMods>(target)
            .map_or(1.0, |mods| mods.damage_taken(1));
        let raw =
            (contribution * done * taken * crate::auras::summon_damage::multiplier(world, pet))
                .round();
        let pet_scaling = crate::level_scaling::UnitScaling::of(world, pet);
        let target_scaling = crate::level_scaling::UnitScaling::of(world, target);
        let armor = target_scaling.armor_against(
            world
                .get::<crate::level_scaling::CreatureArmor>(target)
                .unwrap()
                .0,
            &pet_scaling,
        );
        let floor = crate::combat::armor_mitigated(
            raw,
            armor,
            &pet_scaling,
            &target_scaling,
            Some(world.resource::<crate::level_scaling::ArmorConstants>()),
            mods,
        )
        .damage
        .floor();
        assert!(
            (event.amount as f32) >= floor,
            "actual Sayaad melee {} omits owner AP floor {floor}",
            event.amount
        );
        return;
    }
    panic!("actual Sayaad must land a noncritical melee hit");
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_sayaad_lash_inherits_live_owner_power() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(265, 1).unwrap();
    let owner = run.actor;
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), owner).unwrap();
    let target = run.targets[0];
    engage(&mut run);
    crate::pet_control::order_companion_pet(run.app.world_mut(), owner, Some(target));
    run.app.insert_resource(
        crate::spell_cast::effects::factor_capture::FactorCapture::new(
            pet,
            target,
            7814,
            "sayaad_lash",
            16,
        ),
    );
    for _ in 0..200 {
        run.app.update();
        let capture = run
            .app
            .world()
            .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
            .export();
        let Some(event) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 7814)
        else {
            continue;
        };
        let owner_sp = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(owner)
            .unwrap()
            .spell_power as f64;
        let spell_power = event["factors"]["s_power"].as_f64().unwrap();
        let attack_power = event["factors"]["a_power"].as_f64().unwrap();
        // SimC76f8118 sc_warlock_pets.cpp421–430: actual main Sayaad inherits live SP.
        assert!(
            (spell_power - owner_sp * 5.175).abs() < 1.0,
            "Sayaad SP {spell_power}, owner SP {owner_sp}"
        );
        assert!(
            (attack_power - owner_sp * 2.565).abs() < 1.0,
            "Sayaad AP {attack_power}, owner SP {owner_sp}"
        );
        return;
    }
    panic!("actual Sayaad must land native Lash");
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_literal_sayaad_attacks_primary_with_melee_and_lash() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(265, 1).unwrap();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor)
        .expect("the configured Sayaad must be summoned before combat");
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Npc>(pet)
            .unwrap()
            .template_id,
        1863
    );
    engage(&mut run);
    for _ in 0..400 {
        run.app.update();
        drive_player(&mut run).unwrap();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let damage = |spell| {
        events.iter().any(|event| {
            event.source == Some(pet.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == spell
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
        })
    };
    // SimC76f8118 sc_warlock_pets.cpp405–459: Sayaad uses melee and Lash.
    assert!(
        damage(None),
        "actual Sayaad melee must hit the primary target"
    );
    assert!(
        damage(Some(7814)),
        "actual Sayaad Lash must hit the primary target"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_literal_three_target_profile_keeps_foreground_history() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(265, 3).unwrap();
    engage(&mut run);
    for _ in 0..400 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .unwrap()
            .last_successful_cast_at(27243)
            .is_some(),
        "literal three-target precombat must execute actual Seed"
    );
    let combat = report::combat(run.app.world(), &run.targets, 20.0);
    assert!(combat["total_damage"].as_u64().unwrap() > 0);
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_literal_fixture_executes_profile_instead_of_assisted_kit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(265, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_some(),
        "265 must capture literal control before native setup"
    );
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
            .any(|row| row["spell_id"] == 980 && row["damage"].as_u64().unwrap() > 0),
        "literal Agony must damage the actual recipient"
    );
    assert!(
        !actions.iter().any(|row| row["spell_id"] == 702),
        "Curse of Weakness is absent from the pinned APL"
    );
}
