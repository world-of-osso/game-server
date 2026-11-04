//! Class-party SimC replay receipts and native healer mechanics on the class party.
use shared::components::Health;
use std::path::Path;

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn retribution_divine_protection_casts_selected_shield_of_vengeance() {
    use crate::class_progression::TraitLoadout;
    for selected in [true, false] {
        let mut run = super::snapshot_party_with_specs(65, 90, [104, 65, 70, 251, 62]);
        let paladin = run.slots[2];
        let talents = run.app.world().get::<TraitLoadout>(paladin).unwrap();
        assert!(
            talents
                .active_talent_spells()
                .any(|talent| talent.spell_id == 1261562)
        );
        if !selected {
            run.app
                .world_mut()
                .entity_mut(paladin)
                .remove::<TraitLoadout>();
        }
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            paladin,
            403876,
            Some(paladin),
            false,
        )
        .expect("Divine Protection must be cast successfully");
        assert!(crate::auras::has_aura(run.app.world(), paladin, 403876));
        assert_eq!(
            crate::auras::has_aura(run.app.world(), paladin, 184662),
            selected
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn opening_prismatic_barrier_absorbs_matched_mage_health_and_versatility() {
    use shared::components::Health;
    // VERIFIED_WCL dtmp7Lg2abzyZVBG fight 3 pull 1: actor 2 has an opening
    // 235450 barrier; DB2 70448 DescriptionVariables define its full capacity.
    let builds: [crate::followers::simulator::RecordedBuild; 5] =
        serde_json::from_str(include_str!("fixtures/wcl_holy_paladin_builds.json")).unwrap();
    let (snapshot, _) = super::read_snapshot().unwrap();
    let mut run = super::spawn_class_party(
        &snapshot,
        65,
        90,
        1,
        Some([104, 65, 266, 251, 62]),
        Some(&builds),
    )
    .unwrap();
    let mage = run.slots[4];
    let trace: super::super::healer_wcl_trace::Trace = serde_json::from_str(
        r#"{"provenance":"VERIFIED_WCL dtmp7Lg2abzyZVBG fight 3 pull 1",
        "excluded_self":0,"party_specs":[104,65,266,251,62],
        "actor_slots":{"57":0,"3":1,"1":2,"58":3,"2":4},"bloodlust":[],"hits":[],
        "opening_auras":[{"source_actor":2,"target_actor":2,"spell_id":235450,
        "stacks":1,"evidence":"first_removebuff","remove_ms":17924}]}"#,
    )
    .unwrap();
    trace
        .apply_opening_auras(run.app.world_mut(), &run.slots)
        .unwrap();
    assert!(crate::auras::has_aura(run.app.world(), mage, 235450));
    let max = run.app.world().get::<Health>(mage).unwrap().max;
    let vers = crate::spell_cast::effects::versatility_multiplier(run.app.world(), mage);
    let (_, absorbed) = crate::auras::absorb_damage(run.app.world_mut(), mage, None, 1, max);
    let expected = max * 0.3 * vers;
    assert!(
        (absorbed - expected).abs() < 2.0,
        "opening barrier={absorbed} expected={expected}"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), mage, 235450, Some(mage), false).unwrap();
    for _ in 0..5 {
        run.app.update();
    }
    let (_, absorbed) = crate::auras::absorb_damage(run.app.world_mut(), mage, None, 1, max);
    assert!(
        (absorbed - expected).abs() < 2.0,
        "cast barrier={absorbed} expected={expected}"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn player_barriers_absorb_matched_db2_health_and_versatility() {
    use crate::auras::{AuraApplication, AuraSource};
    // DB2 70448 DescriptionVariables use s2 * MHP * (1 + versadmg) / 100.
    let builds: [crate::followers::simulator::RecordedBuild; 5] =
        serde_json::from_str(include_str!("fixtures/wcl_holy_paladin_builds.json")).unwrap();
    let (snapshot, _) = super::read_snapshot().unwrap();
    let mut run = super::spawn_class_party(
        &snapshot,
        65,
        90,
        1,
        Some([104, 65, 266, 251, 62]),
        Some(&builds),
    )
    .unwrap();
    for (actor, spell_id, fraction) in [
        (run.slots[4], 11426, 0.35),
        (run.slots[4], 235313, 0.30),
        (run.slots[1], 184662, 0.30),
        (run.slots[4], 414661, 0.24),
        (run.slots[4], 414662, 0.20),
        (run.slots[4], 414663, 0.20),
    ] {
        let max = run.app.world().get::<Health>(actor).unwrap().max;
        let vers = crate::spell_cast::effects::versatility_multiplier(run.app.world(), actor);
        crate::auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: actor,
                target: actor,
                spell_id,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
        let (_, absorbed) = crate::auras::absorb_damage(run.app.world_mut(), actor, None, 1, max);
        let expected = max * fraction * vers;
        assert!(
            (absorbed - expected).abs() < 2.0,
            "spell {spell_id} absorbed {absorbed}, expected {expected}"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn opening_holy_bulwark_matches_initial_cast_without_double_counting() {
    // DB2 70448 432496s2: initial 15% target health. Opening remaining
    // capacity is unknown; reconstructing a fresh grant is INFERRED_WCL.
    let builds: [crate::followers::simulator::RecordedBuild; 5] =
        serde_json::from_str(include_str!("fixtures/wcl_holy_paladin_builds.json")).unwrap();
    let (snapshot, _) = super::read_snapshot().unwrap();
    for mode in ["cast", "opening", "opening_with_parent"] {
        let mut run = super::spawn_class_party(
            &snapshot,
            65,
            90,
            1,
            Some([104, 65, 266, 251, 62]),
            Some(&builds),
        )
        .unwrap();
        let caster = run.slots[1];
        let target = run.slots[0];
        super::hold(&mut run, caster);
        for _ in 0..10 {
            run.app.update();
        }
        crate::auras::remove_auras(
            run.app.world_mut(),
            target,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.effects.iter().any(|effect| effect.aura_type == 69),
        );
        if mode == "cast" {
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                caster,
                432459,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..5 {
                run.app.update();
            }
        } else {
            let parent = if mode == "opening_with_parent" {
                r#",{"source_actor":3,"target_actor":57,"spell_id":432496,
                "stacks":1,"evidence":"first_removebuff","remove_ms":20000}"#
            } else {
                ""
            };
            let input = format!(
                r#"{{"provenance":"INFERRED_WCL test",
                "excluded_self":0,"party_specs":[104,65,266,251,62],
                "actor_slots":{{"57":0,"3":1,"1":2,"58":3,"2":4}},
                "bloodlust":[],"hits":[],"opening_auras":[
                {{"source_actor":3,"target_actor":57,"spell_id":432607,
                "stacks":1,"evidence":"first_removebuff","remove_ms":20000}}{parent}]}}"#
            );
            let trace: super::super::healer_wcl_trace::Trace =
                serde_json::from_str(&input).unwrap();
            trace
                .apply_opening_auras(run.app.world_mut(), &run.slots)
                .unwrap();
        }
        let max = run.app.world().get::<Health>(target).unwrap().max;
        let vers = crate::spell_cast::effects::versatility_multiplier(run.app.world(), caster);
        let taken = run
            .app
            .world()
            .get::<crate::auras::AuraStatMods>(target)
            .unwrap()
            .absorb_taken_mult;
        crate::auras::remove_auras(
            run.app.world_mut(),
            target,
            crate::auras::RemoveReason::Consumed,
            |aura| {
                aura.spell_id != 432607 && aura.effects.iter().any(|effect| effect.aura_type == 69)
            },
        );
        let (_, absorbed) = crate::auras::absorb_damage(run.app.world_mut(), target, None, 1, max);
        let expected = (max * 0.15 * vers * taken).round();
        assert!(
            (absorbed - expected).abs() < 2.0,
            "{mode}: {absorbed} != {expected}"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn solidarity_grants_self_before_ally_armament_arrives_only_when_selected() {
    use shared::components::Position;
    let builds: [crate::followers::simulator::RecordedBuild; 5] =
        serde_json::from_str(include_str!("fixtures/wcl_holy_paladin_builds.json")).unwrap();
    let (snapshot, _) = super::read_snapshot().unwrap();
    for (cast, aura) in [(432459, 432496), (432472, 432502)] {
        for selected in [true, false] {
            let mut run = super::spawn_class_party(
                &snapshot,
                65,
                90,
                1,
                Some([104, 65, 266, 251, 62]),
                Some(&builds),
            )
            .unwrap();
            for actor in run.slots {
                super::hold(&mut run, actor);
            }
            for _ in 0..10 {
                run.app.update();
            }
            let caster = run.slots[1];
            let ally = run.slots[0];
            if !selected {
                run.app
                    .world_mut()
                    .entity_mut(caster)
                    .remove::<crate::class_progression::TraitLoadout>();
            }
            let origin = *run.app.world().get::<Position>(caster).unwrap();
            let mut position = run.app.world_mut().get_mut::<Position>(ally).unwrap();
            position.x = origin.x + 20.0;
            position.y = origin.y;
            position.z = origin.z;
            crate::spell_cast::creature_cast(run.app.world_mut(), caster, cast, Some(ally), false)
                .unwrap();
            for _ in 0..3 {
                run.app.update();
            }
            assert_eq!(
                crate::auras::has_aura(run.app.world(), caster, aura),
                selected,
                "cast={cast} selected={selected}"
            );
            assert!(
                !crate::auras::has_aura(run.app.world(), ally, aura),
                "ally must await projectile"
            );
            for _ in 0..40 {
                run.app.update();
            }
            assert!(crate::auras::has_aura(run.app.world(), ally, aura));
            if selected && cast == 432459 {
                let max = run.app.world().get::<Health>(caster).unwrap().max;
                let (_, absorbed) =
                    crate::auras::absorb_damage(run.app.world_mut(), caster, None, 1, max);
                assert!(absorbed > max * 0.15, "self Bulwark must absorb {absorbed}");
            }
        }
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn holy_armament_policy_uses_charges_only_in_combat() {
    use shared::protocol::CombatLogKind;
    // INFERRED_WCL: 260 casts by four current-build players with consistent
    // charge reconstructions spent 6.0% of the observed time at two charges.
    let builds: [crate::followers::simulator::RecordedBuild; 5] =
        serde_json::from_str(include_str!("fixtures/wcl_holy_paladin_builds.json")).unwrap();
    let (snapshot, _) = super::read_snapshot().unwrap();
    let mut run = super::spawn_class_party(
        &snapshot,
        65,
        90,
        1,
        Some([104, 65, 266, 251, 62]),
        Some(&builds),
    )
    .unwrap();
    let healer = run.healer;
    let enemy = run.sources[0];
    super::hold(&mut run, healer);
    for _ in 0..5 {
        run.app.update();
    }
    let mut rotation = server_follower_apl::holy_paladin_rotation::HolyPaladin::load(
        run.app.world_mut(),
        healer,
        crate::simc_apl_native::Encounter::dungeon(0.0),
    )
    .unwrap();
    for _ in 0..80 {
        rotation
            .drive(run.app.world_mut(), healer, &run.slots, None)
            .unwrap();
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        !events
            .iter()
            .any(|event| event.source == Some(healer.to_bits())
                && matches!(event.spell_id, Some(432459 | 432472))
                && event.kind == CombatLogKind::CastSuccess)
    );
    let cursor = events.len();
    for _ in 0..180 {
        rotation
            .drive(run.app.world_mut(), healer, &run.slots, Some(enemy))
            .unwrap();
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..];
    let casts: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(healer.to_bits())
                && matches!(event.spell_id, Some(432459 | 432472))
                && event.kind == CombatLogKind::CastSuccess
        })
        .map(|event| event.spell_id.unwrap())
        .collect();
    assert_eq!(casts, [432459, 432472], "{events:?}");
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn holy_valiance_consumes_infusion_and_reduces_armament_recharge() {
    use crate::auras::{AuraApplication, AuraSource};
    use crate::spell_cast::{SpellHistory, test_harness::charge_recovery};
    // VERIFIED_WCL 1AWhVvdLbpy2gPak fight 36: 54149 is consumed by Flash
    // and Judgment; DB2 70448 432919 reduces recharge by three seconds.
    let builds: [crate::followers::simulator::RecordedBuild; 5] =
        serde_json::from_str(include_str!("fixtures/wcl_holy_paladin_builds.json")).unwrap();
    let (snapshot, _) = super::read_snapshot().unwrap();
    for action in [19750, 275773] {
        let mut run = super::spawn_class_party(
            &snapshot,
            65,
            90,
            1,
            Some([104, 65, 266, 251, 62]),
            Some(&builds),
        )
        .unwrap();
        let healer = run.healer;
        let target = if action == 19750 {
            run.tank
        } else {
            run.sources[0]
        };
        super::hold(&mut run, healer);
        for _ in 0..5 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), healer, 432459, Some(healer), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        crate::auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: healer,
                target: healer,
                spell_id: 54149,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
        assert!(crate::auras::has_aura(run.app.world(), healer, 54149));
        let before = charge_recovery(run.app.world().get::<SpellHistory>(healer).unwrap(), 2220)
            .1
            .unwrap();
        crate::spell_cast::creature_cast(run.app.world_mut(), healer, action, Some(target), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let after = charge_recovery(run.app.world().get::<SpellHistory>(healer).unwrap(), 2220)
            .1
            .unwrap();
        assert!(
            (before - after - 3.0).abs() < 0.001,
            "action={action} before={before} after={after}"
        );
        assert!(!crate::auras::has_aura(run.app.world(), healer, 54149));
        if action == 19750 {
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                healer,
                action,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..40 {
                run.app.update();
            }
            let without_infusion =
                charge_recovery(run.app.world().get::<SpellHistory>(healer).unwrap(), 2220)
                    .1
                    .unwrap();
            assert!((without_infusion - after).abs() < 0.001);
        }
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn boundless_moonlight_heals_only_damage_inside_lunar_beam() {
    use shared::components::Position;
    use shared::protocol::CombatLogKind;
    // VERIFIED_WCL dtmp7Lg2abzyZVBG fight 3 pull 1: 425206 ticks from 15.382s.
    let builds: [crate::followers::simulator::RecordedBuild; 5] =
        serde_json::from_str(include_str!("fixtures/wcl_holy_paladin_builds.json")).unwrap();
    let (snapshot, _) = super::read_snapshot().unwrap();
    let mut run = super::spawn_class_party(
        &snapshot,
        65,
        90,
        1,
        Some([104, 65, 266, 251, 62]),
        Some(&builds),
    )
    .unwrap();
    let tank = run.tank;
    let target = run.sources[0];
    for unit in run.slots {
        super::hold(&mut run, unit);
    }
    for _ in 0..5 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), tank, 5487, Some(tank), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    run.app.world_mut().get_mut::<Health>(tank).unwrap().current *= 0.1;
    crate::spell_cast::creature_cast(run.app.world_mut(), tank, 204066, Some(target), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), tank, 425217));
    let cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::creature_cast(run.app.world_mut(), tank, 33917, Some(target), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..];
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(tank.to_bits())
                && event.spell_id == Some(33917)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(tank.to_bits())
                && event.spell_id == Some(425206)
                && event.kind == CombatLogKind::Heal
                && event.amount > 0),
        "{events:?}"
    );
    run.app.world_mut().get_mut::<Position>(target).unwrap().x += 15.0;
    // Let the previous inside-beam damage flush before the outside-only window.
    for _ in 0..25 {
        run.app.update();
    }
    let cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::creature_cast(run.app.world_mut(), tank, 8921, Some(target), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..];
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(tank.to_bits())
                && event.spell_id == Some(164812)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "{events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| event.source == Some(tank.to_bits()) && event.spell_id == Some(425206)),
        "outside beam: {events:?}"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn holy_masterwork_spends_armament_charges_on_nearby_allies() {
    use crate::auras::ActiveAuras;
    // VERIFIED_WCL dtmp7Lg2abzyZVBG fight 3 pull 1: Holy Shock at 12.701s
    // consumes Masterwork: Bulwark; Lesser Bulwark reaches actor 58 at 12.717s.
    let builds: [crate::followers::simulator::RecordedBuild; 5] =
        serde_json::from_str(include_str!("fixtures/wcl_holy_paladin_builds.json")).unwrap();
    let (snapshot, _) = super::read_snapshot().unwrap();
    for (armament, counter, child) in [(432459, 1271383, 1239002), (432472, 1271436, 1239091)] {
        let mut run = super::spawn_class_party(
            &snapshot,
            65,
            90,
            1,
            Some([104, 65, 266, 251, 62]),
            Some(&builds),
        )
        .unwrap();
        let healer = run.healer;
        let tank = run.tank;
        super::hold(&mut run, healer);
        for _ in 0..5 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), healer, armament, Some(tank), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let stacks = |run: &super::ClassParty| {
            run.app
                .world()
                .get::<ActiveAuras>(healer)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == counter && aura.caster == healer)
                .map_or(0, |aura| aura.stacks)
        };
        assert_eq!(stacks(&run), 3);
        crate::spell_cast::creature_cast(run.app.world_mut(), healer, 20473, Some(tank), false)
            .unwrap();
        for _ in 0..10 {
            run.app.update();
        }
        assert_eq!(stacks(&run), 2);
        let recipients: Vec<_> = run
            .slots
            .into_iter()
            .filter(|unit| {
                run.app
                    .world()
                    .get::<ActiveAuras>(*unit)
                    .is_some_and(|auras| {
                        auras
                            .iter()
                            .any(|aura| aura.spell_id == child && aura.caster == healer)
                    })
            })
            .collect();
        assert_eq!(recipients.len(), 1, "armament={armament}");
        assert!(
            run.slots[2..].contains(&recipients[0]),
            "expected nearby damage dealer"
        );
        if child == 1239091 {
            let stacks = run
                .app
                .world()
                .get::<ActiveAuras>(recipients[0])
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == child && aura.caster == healer)
                .unwrap()
                .stacks;
            assert_eq!(stacks, 5);
        }
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn glistening_radiance_accumulates_only_at_full_health_and_caps() {
    use crate::auras::{AuraApplication, AuraSource};
    // VERIFIED_WCL dtmp7Lg2abzyZVBG fight 3 pull 1: 1241866 absorbs on actor 1.
    let builds: [crate::followers::simulator::RecordedBuild; 5] =
        serde_json::from_str(include_str!("fixtures/wcl_holy_paladin_builds.json")).unwrap();
    let (snapshot, _) = super::read_snapshot().unwrap();
    for beacon in [53563, 156910] {
        let mut run = super::spawn_class_party(
            &snapshot,
            65,
            90,
            1,
            Some([104, 65, 266, 251, 62]),
            Some(&builds),
        )
        .unwrap();
        let healer = run.healer;
        let target = run.slots[4];
        super::hold(&mut run, healer);
        for _ in 0..5 {
            run.app.update();
        }
        let max = run.app.world().get::<Health>(target).unwrap().max;
        run.app
            .world_mut()
            .get_mut::<Health>(target)
            .unwrap()
            .current = max * 0.8;
        crate::auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: healer,
                target,
                spell_id: beacon,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
        for _ in 0..120 {
            run.app.update();
        }
        assert!(!crate::auras::has_aura(run.app.world(), target, 1241866));
        run.app
            .world_mut()
            .get_mut::<Health>(target)
            .unwrap()
            .current = max;
        for _ in 0..400 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), target, 1241866));
        let (_, absorbed) = crate::auras::absorb_damage(run.app.world_mut(), target, None, 1, max);
        let expected = max * 0.06;
        assert!(
            (absorbed - expected).abs() < 2.0,
            "beacon={beacon} absorbed={absorbed} expected={expected}"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn holy_divine_guidance_heals_three_allies_per_spender() {
    use crate::auras::{ActiveAuras, AuraSource};
    use crate::spell_cast::powers::PowerPools;
    use shared::components::PowerType;
    use shared::protocol::CombatLogKind;
    // VERIFIED_WCL dtmp7Lg2abzyZVBG fight 3 pull 1, 37.911s.
    let builds: [crate::followers::simulator::RecordedBuild; 5] =
        serde_json::from_str(include_str!("fixtures/wcl_holy_paladin_builds.json")).unwrap();
    let (snapshot, _) = super::read_snapshot().unwrap();
    let mut amounts = Vec::new();
    for spender_count in [1, 2] {
        let mut run = super::spawn_class_party(
            &snapshot,
            65,
            90,
            1,
            Some([104, 65, 266, 251, 62]),
            Some(&builds),
        )
        .unwrap();
        let healer = run.healer;
        let tank = run.tank;
        super::hold(&mut run, healer);
        for _ in 0..5 {
            run.app.update();
        }
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(healer)
            .unwrap()
            .crit_pct = 0.0;
        for _ in 0..spender_count {
            let mut pools = run.app.world_mut().get_mut::<PowerPools>(healer).unwrap();
            let pool = pools
                .pools
                .iter_mut()
                .find(|pool| pool.power == PowerType::HolyPower)
                .unwrap();
            pool.current = pool.max;
            crate::spell_cast::creature_cast(run.app.world_mut(), healer, 85673, Some(tank), false)
                .unwrap();
            for _ in 0..40 {
                run.app.update();
            }
        }
        let aura = run
            .app
            .world()
            .get::<ActiveAuras>(healer)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 433106 && aura.source == AuraSource::Cast)
            .unwrap();
        assert_eq!(aura.stacks, spender_count);
        for unit in run.slots {
            run.app.world_mut().get_mut::<Health>(unit).unwrap().current *= 0.1;
        }
        let cursor = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            healer,
            26573,
            Some(healer),
            1
        ));
        for _ in 0..10 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[cursor..];
        let heals: Vec<_> = events
            .iter()
            .filter(|event| {
                event.source == Some(healer.to_bits())
                    && event.spell_id == Some(1310389)
                    && event.kind == CombatLogKind::Heal
            })
            .collect();
        assert_eq!(heals.len(), 3, "{events:?}");
        assert!(heals.iter().all(|event| event.amount > 0));
        let received: Vec<_> = heals
            .iter()
            .filter_map(|event| {
                let slot = run
                    .slots
                    .iter()
                    .position(|unit| Some(unit.to_bits()) == event.target)?;
                let amount =
                    (event.amount + event.overflow) as f32 / if event.crit { 2.0 } else { 1.0 };
                Some((slot, amount))
            })
            .collect();
        amounts.push(received);
        assert!(
            !run.app
                .world()
                .get::<ActiveAuras>(healer)
                .unwrap()
                .iter()
                .any(|aura| aura.spell_id == 433106 && aura.source == AuraSource::Cast)
        );
    }
    let (first, second) = amounts[0]
        .iter()
        .find_map(|(slot, first)| {
            amounts[1]
                .iter()
                .find(|(other, _)| slot == other)
                .map(|(_, second)| (*first, *second))
        })
        .expect("both casts must heal at least one common party member");
    assert!((second / first - 2.0).abs() < 0.05, "{amounts:?}");
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn holy_armaments_trigger_hammer_and_anvil_healing() {
    use shared::protocol::CombatLogKind;
    // VERIFIED_WCL dtmp7Lg2abzyZVBG fight 3 pull 1, 10.339s and 13.904s.
    let builds: [crate::followers::simulator::RecordedBuild; 5] =
        serde_json::from_str(include_str!("fixtures/wcl_holy_paladin_builds.json")).unwrap();
    let (snapshot, _) = super::read_snapshot().unwrap();
    for armament in [432459, 432472] {
        let mut run = super::spawn_class_party(
            &snapshot,
            65,
            90,
            1,
            Some([104, 65, 266, 251, 62]),
            Some(&builds),
        )
        .unwrap();
        let healer = run.healer;
        let tank = run.tank;
        super::hold(&mut run, healer);
        for _ in 0..5 {
            run.app.update();
        }
        let traits = run
            .app
            .world()
            .get::<crate::class_progression::TraitLoadout>(healer)
            .unwrap();
        for talent in [433718, 1271553] {
            assert!(
                traits
                    .active_talent_spells()
                    .any(|entry| entry.spell_id == talent)
            );
        }
        run.app.world_mut().get_mut::<Health>(tank).unwrap().current *= 0.1;
        let before = run.app.world().get::<Health>(tank).unwrap().current;
        let cursor = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        crate::spell_cast::creature_cast(run.app.world_mut(), healer, armament, Some(tank), false)
            .unwrap();
        for _ in 0..10 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[cursor..];
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(healer.to_bits())
                    && event.target == Some(tank.to_bits())
                    && event.spell_id == Some(433722)
                    && event.kind == CombatLogKind::Heal
                    && event.amount > 0),
            "armament={armament}: {events:?}"
        );
        assert!(run.app.world().get::<Health>(tank).unwrap().current > before);
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn preservation_leaping_flames_heal_allies_without_hostile_reach() {
    use shared::protocol::CombatLogKind;
    let mut run = super::snapshot_party(1468, 90, 1);
    let healer = run.healer;
    super::hold(&mut run, healer);
    for _ in 0..5 {
        run.app.update();
    }
    for unit in run.slots {
        run.app.world_mut().get_mut::<Health>(unit).unwrap().current *= 0.5;
    }
    assert!(crate::spell_cast::cast_triggered_with_overrides(
        run.app.world_mut(),
        healer,
        370901,
        Some(healer),
        1,
        crate::spell_cast::TriggeredSpellOverrides {
            aura_stacks: Some(2),
            ..Default::default()
        },
    ));
    let world = run.app.world();
    let ally_reach =
        crate::followers::spell_unintended_reach(world, healer, 361469, run.tank).unwrap();
    let enemy_reach =
        crate::followers::spell_unintended_reach(world, healer, 361469, run.sources[0]).unwrap();
    assert_eq!(ally_reach, (0.0, 0.0));
    assert!(
        enemy_reach.0 > 0.0,
        "enemy-targeted jumps retain hostile reach"
    );
    assert!(crate::followers::action_safe(
        world,
        healer,
        run.tank,
        Some(361469)
    ));
    let cursor = world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::creature_cast(run.app.world_mut(), healer, 361469, Some(run.tank), false)
        .unwrap();
    for _ in 0..60 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..];
    let healed: std::collections::HashSet<_> = events
        .iter()
        .filter(|e| {
            e.source == Some(healer.to_bits())
                && e.spell_id == Some(361509)
                && e.kind == CombatLogKind::Heal
        })
        .filter_map(|e| e.target)
        .collect();
    assert_eq!(
        healed.len(),
        3,
        "primary and two different allies: {healed:?}"
    );
    assert!(healed.contains(&run.tank.to_bits()));
    assert!(
        healed
            .iter()
            .all(|target| run.slots.iter().any(|unit| unit.to_bits() == *target))
    );
    assert!(!crate::auras::has_aura(run.app.world(), healer, 370901));
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn preservation_keeps_routine_heals_before_living_flame_at_moderate_injury() {
    use shared::protocol::CombatLogKind;
    // No retail trigger separates Living Flame casts from ready-but-uncast moments;
    // see docs/specs/healer-tank-replay/preservation-living-flame.md.
    for health in [0.60, 0.70] {
        let mut run = super::snapshot_party(1468, 90, 1);
        let healer = run.healer;
        super::hold(&mut run, healer);
        for _ in 0..5 {
            run.app.update();
        }
        run.app
            .world_mut()
            .get_mut::<Health>(run.tank)
            .unwrap()
            .current *= health;
        let mut rotation = server_follower_apl::preservation_evoker_rotation::Preservation::load(
            run.app.world_mut(),
            healer,
            crate::simc_apl_native::Encounter::dungeon(0.0),
        )
        .unwrap();
        let cursor = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        rotation
            .drive(run.app.world_mut(), healer, &run.slots, &run.sources)
            .unwrap();
        for _ in 0..80 {
            run.app.update();
        }
        let first = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[cursor..]
            .iter()
            .find(|e| e.source == Some(healer.to_bits()) && e.kind == CombatLogKind::CastSuccess)
            .map(|e| (e.spell_id, e.target));
        assert_eq!(
            first,
            Some((Some(364343), Some(run.tank.to_bits()))),
            "health={health}: Echo, not Living Flame, on the injured tank"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn preservation_fire_breath_waits_for_observed_timeout() {
    use shared::protocol::CombatLogKind;
    for (count, elapsed, health, expected) in [
        (1, 0.0, 1.0, false),
        (5, 0.0, 0.80, false),
        (1, 27.0, 0.80, true),
        (5, 27.0, 0.50, true),
    ] {
        let mut run = super::snapshot_party(1468, 90, count);
        let healer = run.healer;
        super::hold(&mut run, healer);
        for _ in 0..5 {
            run.app.update();
        }
        super::engage(&mut run);
        run.app
            .world_mut()
            .get_mut::<Health>(run.tank)
            .unwrap()
            .current *= health;
        let now = crate::spell_cast::now(run.app.world());
        let mut rotation = server_follower_apl::preservation_evoker_rotation::Preservation::load(
            run.app.world_mut(),
            healer,
            crate::simc_apl_native::Encounter::dungeon(now - elapsed),
        )
        .unwrap();
        let cursor = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        rotation
            .drive(run.app.world_mut(), healer, &run.slots, &run.sources)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[cursor..];
        let cast = events.iter().any(|e| {
            e.source == Some(healer.to_bits())
                && e.spell_id == Some(357208)
                && e.kind == CombatLogKind::CastSuccess
        });
        let casts: Vec<_> = events
            .iter()
            .filter(|e| e.source == Some(healer.to_bits()) && e.kind == CombatLogKind::CastSuccess)
            .map(|e| e.spell_id)
            .collect();
        assert_eq!(
            cast, expected,
            "count={count} elapsed={elapsed} health={health}: casts={casts:?}"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn preservation_spends_merithra_above_the_old_health_cutoff() {
    use crate::auras::{AuraApplication, AuraSource};
    use shared::protocol::CombatLogKind;
    let mut run = super::snapshot_party(1468, 90, 1);
    let healer = run.healer;
    super::hold(&mut run, healer);
    for _ in 0..5 {
        run.app.update();
    }
    let health = run.app.world_mut().get_mut::<Health>(run.tank).unwrap();
    health.into_inner().current *= 0.90;
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: healer,
            target: healer,
            spell_id: 1256579,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    let mut rotation = server_follower_apl::preservation_evoker_rotation::Preservation::load(
        run.app.world_mut(),
        healer,
        crate::simc_apl_native::Encounter::dungeon(0.0),
    )
    .unwrap();
    let cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..8 {
        rotation
            .drive(run.app.world_mut(), healer, &run.slots, &[run.sources[0]])
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
    }
    let casts: Vec<_> = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..]
        .iter()
        .filter(|event| {
            event.source == Some(healer.to_bits()) && event.kind == CombatLogKind::CastSuccess
        })
        .filter_map(|event| event.spell_id)
        .collect();
    assert!(casts.contains(&1256581), "casts={casts:?}");
}

#[test]
#[ignore = "class-party death-removal audit over the measurement snapshot"]
fn class_party_death_audit() {
    let spec = std::env::var("HEALER_TANK_SPEC").unwrap().parse().unwrap();
    let level = std::env::var("HEALER_TANK_LEVEL").unwrap().parse().unwrap();
    let seed = std::env::var("HEALER_TANK_SEED").unwrap().parse().unwrap();
    let trace = std::env::var("HEALER_TANK_TIMELINE").unwrap();
    let report = super::replay_simc(spec, level, Path::new(&trace), 120, seed).unwrap();
    println!("HEALER_DEATH_AUDIT_REPORT={report}");
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn grove_guardians_heal_the_class_followers_target() {
    use shared::protocol::CombatLogKind;

    for source in [18562, 48438] {
        let mut run = super::snapshot_party(105, 90, 1);
        let healer = run.healer;
        super::hold(&mut run, healer);
        for _ in 0..5 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), run.healer, 1226140));
        run.app
            .world_mut()
            .get_mut::<Health>(run.tank)
            .unwrap()
            .current *= 0.25;
        if source == 18562 {
            crate::spell_cast::cast_triggered(
                run.app.world_mut(),
                run.healer,
                774,
                Some(run.tank),
                1,
            );
        }
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.healer,
            source,
            Some(run.tank),
            false,
        )
        .expect("cast selected Restoration spell");
        let mut treant = None;
        let mut healed = false;
        for _ in 0..160 {
            run.app.update();
            let world = run.app.world_mut();
            if treant.is_none() {
                treant = world
                    .query::<(
                        bevy::prelude::Entity,
                        &crate::spell_cast::summon::SummonedCreature,
                    )>()
                    .iter(world)
                    .find_map(|(actor, summon)| {
                        (summon.owner == run.healer && summon.spell_id == 102693).then_some(actor)
                    });
            }
            if let Some(actor) = treant {
                healed = world
                    .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                    .0
                    .iter()
                    .any(|event| {
                        event.source == Some(actor.to_bits())
                            && event.target == Some(run.tank.to_bits())
                            && event.kind == CombatLogKind::Heal
                            && event.spell_id == Some(422090)
                            && event.amount > 0
                    });
                if healed {
                    break;
                }
            }
        }
        assert!(treant.is_some(), "spell {source} must summon a treant");
        assert!(
            healed,
            "spell {source}'s treant must heal the original target"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn grove_nourish_applies_the_owners_harmony() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::protocol::CombatLogKind;

    let mut run = super::snapshot_party(105, 90, 1);
    let healer = run.healer;
    super::hold(&mut run, healer);
    for _ in 0..5 {
        run.app.update();
    }
    let stats = run
        .app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(healer)
        .unwrap();
    let mastery = stats.mastery_pct;
    assert!(mastery > 0.0);
    run.app
        .world_mut()
        .get_mut::<Health>(run.tank)
        .unwrap()
        .current *= 0.1;
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 102693, Some(run.tank), 1);
    let world = run.app.world_mut();
    let treant = world
        .query::<(bevy::prelude::Entity, &SummonedCreature)>()
        .iter(world)
        .find_map(|(actor, summon)| {
            (summon.owner == healer && summon.spell_id == 102693).then_some(actor)
        })
        .expect("Grove Guardian summoned");
    crate::spell_cast::cast_triggered(world, treant, 422090, Some(run.tank), 1);
    crate::spell_cast::cast_triggered(world, healer, 774, Some(run.tank), 1);
    crate::spell_cast::cast_triggered(world, treant, 422090, Some(run.tank), 1);
    let heals: Vec<_> = world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(treant.to_bits())
                && event.target == Some(run.tank.to_bits())
                && event.kind == CombatLogKind::Heal
                && event.spell_id == Some(422090)
        })
        .collect();
    assert_eq!(heals.len(), 2, "both Nourish casts must heal the target");
    let normalized = |event: &shared::protocol::CombatLogEvent| {
        event.amount as f32 / if event.crit { 2.0 } else { 1.0 }
    };
    let ratio = normalized(heals[1]) / normalized(heals[0]);
    let expected = 1.0 + mastery / 100.0;
    let variance = world
        .resource::<crate::spell_info::SpellStore>()
        .get(422090)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .variance;
    let lower = expected * (1.0 - variance) / (1.0 + variance);
    let upper = expected * (1.0 + variance) / (1.0 - variance);
    assert!(
        ratio >= lower && ratio <= upper,
        "one owned HoT: {ratio} not in {lower}..={upper}"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn restoration_druid_harmony_increases_swiftmend_for_owned_hots() {
    use shared::protocol::CombatLogKind;
    let mut run = super::snapshot_party(105, 90, 1);
    let healer = run.healer;
    let tank = run.tank;
    super::hold(&mut run, healer);
    for _ in 0..5 {
        run.app.update();
    }
    let mastery = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(healer)
        .unwrap()
        .mastery_pct;
    assert!(mastery > 0.0);
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(healer)
        .unwrap()
        .crit_pct = 0.0;
    run.app.world_mut().get_mut::<Health>(tank).unwrap().current *= 0.1;
    let cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(18562)
        .unwrap()
        .clone();
    let ctx = crate::spell_cast::effects::EffectCtx {
        caster: healer,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::apply_heal(run.app.world_mut(), &ctx, tank, 10000.0, false, false);
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 774, Some(tank), 1);
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 155777, Some(tank), 1);
    crate::spell_cast::effects::apply_heal(run.app.world_mut(), &ctx, tank, 10000.0, false, false);
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..];
    let heals: Vec<_> = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.source == Some(healer.to_bits())
                && event.target == Some(tank.to_bits())
                && event.spell_id == Some(18562)
        })
        .collect();
    assert_eq!(heals.len(), 2, "both Swiftmends must resolve: {events:?}");
    let normalized = |event: &shared::protocol::CombatLogEvent| {
        event.amount as f32 / if event.crit { 2.0 } else { 1.0 }
    };
    let ratio = normalized(heals[1]) / normalized(heals[0]);
    let expected = 1.0 + 2.0 * mastery / 100.0;
    assert!(
        (ratio - expected).abs() < 0.01,
        "two owned HoTs: {ratio} vs {expected}"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn restoration_harmony_counts_harmonious_lifebloom_and_periodic_heals() {
    use shared::protocol::CombatLogKind;
    let bytes = std::fs::read(fixture("wcl_worldnature_builds.json")).unwrap();
    let builds: [crate::followers::simulator::RecordedBuild; 5] =
        serde_json::from_slice(&bytes).unwrap();
    let (snapshot, _) = super::read_snapshot().unwrap();
    let mut run = super::spawn_class_party(
        &snapshot,
        105,
        90,
        1,
        Some([66, 105, 70, 1480, 262]),
        Some(&builds),
    )
    .unwrap();
    let healer = run.healer;
    let tank = run.tank;
    let foreign = run.slots[2];
    super::hold(&mut run, healer);
    for _ in 0..5 {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), healer, 392256));
    let mastery = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(healer)
        .unwrap()
        .mastery_pct;
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(healer)
        .unwrap()
        .crit_pct = 0.0;
    run.app.world_mut().get_mut::<Health>(tank).unwrap().current *= 0.1;
    let spells = run.app.world().resource::<crate::spell_info::SpellStore>();
    let swiftmend = spells.get(18562).unwrap().clone();
    let immune = spells.get(474683).unwrap().clone();
    let cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::cast_triggered(run.app.world_mut(), foreign, 774, Some(tank), 1);
    for spell in [&swiftmend, &immune] {
        let ctx = crate::spell_cast::effects::EffectCtx {
            caster: healer,
            spell,
            depth: 1,
            dest: None,
            chain_multiplier: 1.0,
        };
        crate::spell_cast::effects::apply_heal(
            run.app.world_mut(),
            &ctx,
            tank,
            10000.0,
            false,
            false,
        );
        if spell.id == 18562 {
            crate::spell_cast::effects::apply_heal(
                run.app.world_mut(),
                &ctx,
                tank,
                10000.0,
                false,
                true,
            );
        }
    }
    for spell in [774, 155777, 33763] {
        crate::spell_cast::cast_triggered(run.app.world_mut(), healer, spell, Some(tank), 1);
    }
    for spell in [&swiftmend, &immune] {
        let ctx = crate::spell_cast::effects::EffectCtx {
            caster: healer,
            spell,
            depth: 1,
            dest: None,
            chain_multiplier: 1.0,
        };
        crate::spell_cast::effects::apply_heal(
            run.app.world_mut(),
            &ctx,
            tank,
            10000.0,
            false,
            false,
        );
        if spell.id == 18562 {
            crate::spell_cast::effects::apply_heal(
                run.app.world_mut(),
                &ctx,
                tank,
                10000.0,
                false,
                true,
            );
        }
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..];
    let values = |spell, periodic| {
        events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Heal
                    && event.source == Some(healer.to_bits())
                    && event.target == Some(tank.to_bits())
                    && event.spell_id == Some(spell)
                    && event.periodic == periodic
            })
            .map(|event| event.amount as f32 / if event.crit { 2.0 } else { 1.0 })
            .collect::<Vec<_>>()
    };
    for periodic in [false, true] {
        let heals = values(18562, periodic);
        assert_eq!(heals.len(), 2);
        let ratio = heals[1] / heals[0];
        let expected = 1.0 + 5.0 * mastery / 100.0;
        assert!(
            (ratio - expected).abs() < 0.01,
            "Harmony periodic={periodic}: {ratio} vs {expected}"
        );
    }
    let heals = values(474683, false);
    assert_eq!(heals.len(), 2);
    assert_eq!(
        heals[0], heals[1],
        "modifier-immune percent heal must not gain Harmony"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn dummy_pressure_uses_unscaled_same_level_units() {
    for level in [70, 90] {
        let run = super::snapshot_party(105, level, 1);
        let world = run.app.world();
        let source = run.sources[0];
        assert_eq!(
            world
                .get::<shared::components::UnitLevel>(source)
                .unwrap()
                .0,
            level
        );
        assert_eq!(
            world
                .get::<shared::components::UnitLevel>(run.tank)
                .unwrap()
                .0,
            level
        );
        assert!(
            world
                .get::<shared::level_scaling::LevelScaling>(source)
                .is_none()
        );
        assert!(
            world
                .get::<shared::level_scaling::LevelScaling>(run.tank)
                .is_none()
        );
        assert_eq!(
            crate::level_scaling::damage_to_hp_scale(world, source, run.tank),
            1.0
        );
        eprintln!(
            "REPLAY_SCALE level={level} source_damage_to_hp=1 tank_health={}",
            world.get::<Health>(run.tank).unwrap().max
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn death_audit_records_real_aura_removals() {
    let mut run = super::snapshot_party(105, 70, 1);
    super::settle_party_buffs(&mut run).unwrap();
    run.app.init_resource::<crate::auras::DeathAuraAudit>();
    run.app
        .world_mut()
        .get_mut::<Health>(run.tank)
        .unwrap()
        .current = 0.0;
    run.app.update();
    let audit = run.app.world().resource::<crate::auras::DeathAuraAudit>();
    assert!(
        audit
            .0
            .iter()
            .any(|(unit, spells)| *unit == run.tank && spells.contains(&1126))
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn optimal_raid_environment_reaches_all_replay_actors() {
    let mut run = super::snapshot_party(256, 90, 1);
    let members = run.slots.to_vec();
    let enemy = run.sources[0];
    super::apply_optimal_raid_environment(&mut run.app, enemy, &members).unwrap();
    let world = run.app.world();
    for member in members {
        assert!(
            crate::auras::has_aura(world, member, 2825),
            "member {member:?} lacks Bloodlust"
        );
    }
    for spell in [1490, 113746] {
        assert!(
            crate::auras::has_aura(world, run.sources[0], spell),
            "enemy lacks {spell}"
        );
    }
}

/// Party 66/256/262/63/253 provides Power Word: Fortitude (Discipline), Skyfury
/// (Elemental) and Arcane Intellect (Fire): every slot holds exactly those three.
#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn replay_party_holds_exactly_the_buffs_its_composition_casts() {
    use server_follower_apl::party_buffs::{PARTY_BUFFS, holds_buff};
    let mut run = super::snapshot_party(256, 90, 1);
    super::settle_party_buffs(&mut run).unwrap();
    let world = run.app.world();
    for unit in run.slots {
        let held: Vec<u32> = PARTY_BUFFS
            .into_iter()
            .map(|(spell, _)| spell)
            .filter(|&spell| holds_buff(world, unit, spell).unwrap())
            .collect();
        assert_eq!(held, [1459, 21562, 462854], "slot {unit:?}");
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn beacon_transfers_healing_between_party_members() {
    let mut run = super::snapshot_party(65, 90, 1);
    let (tank, healer) = (run.tank, run.healer);
    // A DPS party member.
    let follower = run.slots[2];
    for (beacon, healed) in [(tank, follower), (follower, tank)] {
        for target in [beacon, healed] {
            let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
            health.current = health.max * 0.25;
        }
        crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 53563, Some(beacon), 1);
        let before = run.app.world().get::<Health>(beacon).unwrap().current;
        let healed_before = run.app.world().get::<Health>(healed).unwrap().current;
        crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 19750, Some(healed), 1);
        let after = run.app.world().get::<Health>(beacon).unwrap().current;
        let effective = run.app.world().get::<Health>(healed).unwrap().current - healed_before;
        let transferred = after - before;
        assert!(effective > 0.0);
        assert!(
            (transferred - effective * 0.15).abs() <= 2.0,
            "Beacon {beacon:?} received {transferred} from {effective} healing on {healed:?}"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn preservation_lifebind_aggregates_raw_healing_on_native_party() {
    let mut run = super::snapshot_party(1468, 90, 1);
    let (tank, healer) = (run.tank, run.healer);
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 360995, Some(tank), 1);
    assert!(crate::auras::has_aura(run.app.world(), healer, 373267));
    assert!(crate::auras::has_aura(run.app.world(), tank, 373267));
    for unit in [tank, healer] {
        let mut health = run.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.current = if unit == tank {
            health.max
        } else {
            health.max * 0.25
        };
    }
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..2 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 361509, Some(tank), 1);
    }
    for _ in 0..3 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    assert!(!events.iter().any(|event| event.spell_id == Some(373268)));
    for _ in 0..2 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let raw: i32 = events
        .iter()
        .filter(|event| {
            event.spell_id == Some(361509)
                && event.kind == shared::protocol::CombatLogKind::Heal
                && event.target == Some(tank.to_bits())
        })
        .map(|event| {
            assert_eq!(
                event.amount, event.overflow,
                "source heal must be entirely overheal"
            );
            event.amount
        })
        .sum();
    assert!(raw > 0);
    let transfers: Vec<_> = events
        .iter()
        .filter(|event| {
            event.spell_id == Some(373268)
                && event.kind == shared::protocol::CombatLogKind::Heal
                && event.target == Some(healer.to_bits())
        })
        .collect();
    assert_eq!(
        transfers.len(),
        1,
        "two heals must aggregate into one 200 ms transfer"
    );
    assert!((transfers[0].amount as f32 - raw as f32 * 0.6).abs() <= 2.0);

    let start = events.len() + start;
    for unit in [tank, healer] {
        let mut health = run.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.current = if unit == healer {
            health.max
        } else {
            health.max * 0.25
        };
    }
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 361509, Some(healer), 1);
    for _ in 0..5 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let heal = events
        .iter()
        .find(|event| {
            event.kind == shared::protocol::CombatLogKind::Heal
                && event.spell_id == Some(361509)
                && event.target == Some(healer.to_bits())
        })
        .expect("self-heal must land");
    assert_eq!(heal.amount, heal.overflow);
    let transfer = events
        .iter()
        .find(|event| {
            event.kind == shared::protocol::CombatLogKind::Heal
                && event.spell_id == Some(373268)
                && event.target == Some(tank.to_bits())
        })
        .expect("Lifebind must heal the bonded ally");
    assert!((transfer.amount as f32 - heal.amount as f32 * 0.6).abs() <= 2.0);
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn preservation_lifebind_transfers_periodic_overhealing() {
    let mut run = super::snapshot_party(1468, 90, 1);
    let (tank, healer) = (run.tank, run.healer);
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 360995, Some(tank), 1);
    for unit in [tank, healer] {
        let mut health = run.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.current = if unit == tank {
            health.max
        } else {
            health.max * 0.25
        };
    }
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 366155, Some(tank), 1);
    for _ in 0..45 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let tick = events
        .iter()
        .find(|event| {
            event.kind == shared::protocol::CombatLogKind::Heal
                && event.spell_id == Some(366155)
                && event.periodic
                && event.target == Some(tank.to_bits())
        })
        .expect("Reversion must tick on the bonded ally");
    assert_eq!(
        tick.amount, tick.overflow,
        "source tick must be fully overhealed"
    );
    let transfer = events
        .iter()
        .find(|event| {
            event.kind == shared::protocol::CombatLogKind::Heal
                && event.spell_id == Some(373268)
                && event.target == Some(healer.to_bits())
        })
        .expect("Lifebind must transfer the periodic heal");
    assert!((transfer.amount as f32 - tick.amount as f32 * 0.6).abs() <= 2.0);
}

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/dungeon_simulator/fixtures")
        .join(name)
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn simc_party_pulse_hits_all_five_class_party_members() {
    let report = super::replay_simc_party(
        65,
        90,
        &fixture("healer_simc_fixture.log"),
        &fixture("healer_simc_party_fixture.log"),
        12,
        70448,
    )
    .unwrap();
    assert_eq!(report["party_incoming_events"], 5);
    let party_damage = report["metrics"]["party_damage"].as_object().unwrap();
    assert_eq!(
        party_damage.len(),
        5,
        "every party member must take native damage: {report}"
    );
    // The tank can absorb its hit; the other four must take real health damage.
    assert!(
        party_damage
            .values()
            .filter(|v| v.as_u64().unwrap() > 0)
            .count()
            >= 4,
        "{report}"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn simc_party_pulse_replays_two_engaged_damageable_dummies() {
    let report = super::replay_simc_party_with_targets(
        256,
        90,
        &fixture("healer_simc_fixture.log"),
        &fixture("healer_simc_party_fixture.log"),
        12,
        70448,
        2,
    )
    .unwrap();
    assert_eq!(report["target_count"], 2);
    assert_eq!(report["party_incoming_events"], 5);
    let targets = report["damage_by_dummy"].as_object().unwrap();
    assert_eq!(
        targets.len(),
        2,
        "both engaged dummies must take damage: {report}"
    );
    assert!(targets.values().all(|value| value.as_u64().unwrap() > 0));
    assert!(report["pain_target_seconds"].as_f64().unwrap() > 0.0);
    assert!(report["atonement_target_seconds"].as_f64().unwrap() > 0.0);
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn simc_enemy_profile_changes_engaged_enemy_count() {
    let report = super::replay_simc_party_with_profile(
        270,
        90,
        &fixture("healer_simc_fixture.log"),
        &fixture("healer_simc_party_fixture.log"),
        5,
        70448,
        &fixture("healer_enemy_profile_test.json"),
    )
    .unwrap();
    let samples = report["enemy_profile"]["samples"].as_array().unwrap();
    let counts: Vec<_> = samples
        .iter()
        .map(|s| {
            (
                s["time_ms"].as_u64().unwrap(),
                s["observed"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(counts, [(0, 1), (1000, 2), (2000, 3), (3000, 2), (4000, 1)]);
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn simc_enemy_profile_retires_targets_during_active_casts() {
    let report = super::replay_simc_party_with_profile(
        270,
        90,
        &fixture("healer_simc_fixture.log"),
        &fixture("healer_simc_party_fixture.log"),
        35,
        1,
        &fixture("altar_fangs_wcl_pull1.json"),
    )
    .unwrap();
    let samples = report["enemy_profile"]["samples"].as_array().unwrap();
    assert!(
        samples
            .iter()
            .any(|sample| sample["time_ms"].as_u64().unwrap() >= 24_050)
    );
    assert!(
        samples
            .iter()
            .all(|sample| sample["expected"] == sample["observed"])
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn wcl_combatant_build_equips_recorded_items_and_talents() {
    let bytes = std::fs::read(fixture("wcl_worldnature_builds.json")).unwrap();
    let builds: [crate::followers::simulator::RecordedBuild; 5] =
        serde_json::from_slice(&bytes).unwrap();
    let (snapshot, _) = super::read_snapshot().unwrap();
    let party = super::spawn_class_party(
        &snapshot,
        105,
        90,
        1,
        Some([66, 105, 70, 1480, 262]),
        Some(&builds),
    )
    .unwrap();
    let world = party.app.world();
    let gear = world
        .get::<crate::stats::EquippedItems>(party.healer)
        .unwrap();
    assert!(gear.items.iter().any(|(_, _, item)| *item == 270167));
    assert!(gear.items.iter().any(|(_, _, item)| *item == 250214));
    let talents = world
        .get::<crate::class_progression::TraitLoadout>(party.healer)
        .unwrap();
    assert_eq!(talents.talent_rank(1213597), Some(1));
    assert_eq!(talents.talent_rank(392256), Some(1));
    assert_eq!(talents.talent_rank(474678), Some(1));
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn wcl_bloodlust_applies_only_to_the_recorded_slot_at_the_recorded_time() {
    let mut party = super::snapshot_party(105, 90, 1);
    let buffs = [super::super::healer_wcl_trace::TimedBloodlust {
        time_ms: 5000,
        slot: 1,
    }];
    let mut next = 0;
    super::apply_due_bloodlust(party.app.world_mut(), &party.slots, &buffs, &mut next, 4950)
        .unwrap();
    assert!(!crate::auras::has_aura(
        party.app.world(),
        party.slots[1],
        2825
    ));
    super::apply_due_bloodlust(party.app.world_mut(), &party.slots, &buffs, &mut next, 5000)
        .unwrap();
    assert!(crate::auras::has_aura(
        party.app.world(),
        party.slots[1],
        2825
    ));
    assert!(!crate::auras::has_aura(
        party.app.world(),
        party.slots[0],
        2825
    ));
    assert_eq!(next, 1);
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn wcl_intake_replays_hostile_and_deferred_pressure() {
    let report = super::replay_simc_party_with_wcl(
        1468,
        90,
        Path::new("not-used-for-wcl"),
        10,
        1,
        &fixture("altar_fangs_wcl_pull1.json"),
        &fixture("altar_fangs_wcl_pull1_damage.json"),
        None,
    )
    .unwrap();
    assert_eq!(report["timeline"], "wcl_damage_taken");
    assert_eq!(report["incoming_events"], 0);
    assert_eq!(report["party_incoming_events"], 0);
    assert_eq!(report["wcl_intake"]["records"], 64);
    let applied = report["wcl_intake"]["delivered"].as_u64().unwrap();
    let dead = report["wcl_intake"]["events_on_dead_targets_not_replayed"]
        .as_u64()
        .unwrap();
    let avoided = report["wcl_intake"]["avoided"].as_u64().unwrap();
    assert!(applied > 0);
    assert!(avoided > 0);
    assert_eq!(applied + avoided + dead, 64);
    assert_eq!(
        report["wcl_intake"]["self_rows_excluded_from_full_pull"],
        68
    );
    assert_eq!(report["tank_spec"], 268);
    let max_hp = report["party_initial_max_hp"].as_array().unwrap();
    assert_eq!(max_hp.len(), 5);
    assert!(max_hp.iter().all(|hp| hp.as_f64().unwrap() > 0.0));
    let casts = report["healer_cast_events"].as_array().unwrap();
    let cast_count: u64 = report["healer_casts"]
        .as_object()
        .unwrap()
        .values()
        .map(|n| n.as_u64().unwrap())
        .sum();
    assert!(cast_count > 0);
    assert_eq!(casts.len() as u64, cast_count);
    let heals = report["metrics"]["healing_events"].as_array().unwrap();
    let effective: u64 = heals
        .iter()
        .map(|event| event["effective"].as_u64().unwrap())
        .sum();
    let spell_effective: u64 = report["metrics"]["spell_healing"]
        .as_object()
        .unwrap()
        .values()
        .map(|spell| spell["effective"].as_u64().unwrap())
        .sum();
    assert!(effective > 0);
    assert_eq!(effective, spell_effective);
    assert!(
        !report["metrics"]["party_health_curve"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(report["wcl_intake"]["matches_recorded_party"], true);
    assert_eq!(
        report["wcl_intake"]["party_specs"],
        serde_json::json!([268, 1468, 62, 262, 262])
    );
    assert!(
        report["metrics"]["party_damage"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| v.as_u64().unwrap_or(0) > 0)
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot and SimC traces"]
fn simc_class_healer_replays_damage_and_reports_receipts() {
    // The regular low-pressure trace leaves the tank above Holy Shock's healing threshold.
    let trace = fixture("healer_simc_healing_pressure.log");
    assert_eq!(
        super::replay_simc(66, 90, &trace, 12, 70448).unwrap_err(),
        "Simulator party slot2 requiresHealer, spec66 isTank",
        "only a healer spec takes the healer slot"
    );
    let report = super::replay_simc(65, 90, &trace, 12, 70448).unwrap();
    assert_eq!(report["incoming_events"], 3);
    assert_eq!(report["level"], 90);
    assert_eq!(report["actors"], "class_followers");
    let snapshot = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let digest = {
        use sha2::Digest;
        format!(
            "{:x}",
            sha2::Sha256::digest(std::fs::read(snapshot).unwrap())
        )
    };
    assert_eq!(report["world_db_sha256"], digest.as_str());
    let metrics = &report["metrics"];
    let idle = metrics["idle_gcd_pct"].as_f64().unwrap();
    assert!(
        (0.0..100.0).contains(&idle),
        "the class healer must cast during the fixture: {report}"
    );
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(effective > 0.0 && raw >= effective, "{report}");
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 13);
}

#[test]
fn class_replay_refuses_the_canonical_world_database() {
    assert_eq!(
        super::hash_snapshot(Path::new(super::CANONICAL_WORLD_DB)).unwrap_err(),
        "HEALER_REPLAY_REQUIRES_SNAPSHOT_WORLD_DB"
    );
}
