//! Fixture adapter over the production APL.
use crate::simc_apl_native::Encounter;
use bevy::prelude::*;
pub(super) use server_follower_apl::guardian_tank_rotation::*;

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    use server_follower_apl::{SpecRotation, literal_rotation::PrecombatPoll};
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world(), run.actor, encounter)?;
    super::literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    super::preparation::finish_pending_setup(run)?;
    while let PrecombatPoll::Selected { action, execution } =
        rotation.poll_precombat(run.app.world_mut(), run.actor, &run.targets)?
    {
        super::preparation::execute_precombat_plan(run, &mut rotation.driver, action, execution)?;
    }
    rotation.encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("guardian_tank_rotation");
impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_GUARDIAN_76f8118"
    }
    fn tank_active_mitigation_auras(&self) -> Option<&'static [u32]> {
        Some(&[192081, 22842, 22812, 200851])
    }
    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        server_follower_apl::SpecRotation::drive(self, world, actor, targets)
    }
    fn report(&self) -> serde_json::Value {
        serde_json::json!({"decisions": self.driver.decisions, "native_precombat_snapshot": self.driver.snapshot})
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires Guardian MID2 profile and native world data"]
    fn guardian_thrash_real_hit_uses_selected_damage_modifiers() {
        use crate::spell_cast::effects::factor_capture::FactorCapture;
        let mut run = super::super::prepare_registered_player(104, 70448).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 102558, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        run.app
            .insert_resource(FactorCapture::new(actor, target, 77758, "thrash", 1));
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
            .unwrap();
        let capture = run.app.world().resource::<FactorCapture>().export();
        let events = capture["events"].as_array().unwrap();
        let direct = events
            .iter()
            .find(|e| e["event"] == "direct_amount" && e["spell_id"] == 77758)
            .unwrap();
        let result = events
            .iter()
            .find(|e| e["event"] == "result" && e["spell_id"] == 77758)
            .unwrap();
        let base = direct["factors"]["native_scaled_amount"].as_f64().unwrap();
        let vers = result["factors"]["versatility"].as_f64().unwrap();
        let actual = result["factors"]["raw_amount"].as_f64().unwrap();
        // Guardian 8% and 125%, Killer Instinct 6%, Grievous Wounds 10%,
        // Lunar Calling 12%, Circle 5%, Fury of Nature rank two 30%.
        let expected = base * vers * 1.08 * 2.25 * 1.06 * 1.10 * 1.12 * 1.05 * 1.30 * 1.10;
        assert!(
            (actual / expected - 1.0).abs() < 0.026,
            "actual={actual} expected={expected} trace={capture}"
        );
    }

    #[test]
    #[ignore = "requires Guardian MID2 profile and native world data"]
    fn guardian_spirit_wrath_follows_real_mangle_with_delayed_damage() {
        let mut run = super::super::prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1308399, Some(actor), 1);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 33917, Some(target), false)
            .unwrap();
        let observed = |run: &super::super::Run| {
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(1308093)
                        && event.kind == shared::protocol::CombatLogKind::Damage
                })
                .count()
        };
        assert_eq!(
            observed(&run),
            0,
            "Spirit's Wrath cannot land at the Mangle cast"
        );
        for _ in 0..10 {
            run.app.update();
        }
        assert_eq!(
            observed(&run),
            1,
            "one real Mangle with an active spirit must land one delayed Spirit's Wrath"
        );
    }
    #[test]
    #[ignore = "requires Guardian MID2 profile and native world data"]
    fn guardian_answered_calling_requires_paid_rage_and_single_summon() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let count = |run: &mut super::super::Run| {
            let world = run.app.world_mut();
            world
                .query::<&crate::spell_cast::summon::SummonedCreature>()
                .iter(world)
                .filter(|summon| summon.owner == actor && summon.spell_id == 1308399)
                .count()
        };
        assert_eq!(
            count(&mut run),
            0,
            "form casting alone cannot summon the spirit"
        );
        for _ in 0..80 {
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                shared::components::PowerType::Rage,
                1000.0,
            );
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                192081,
                Some(actor),
                false,
            )
            .unwrap();
            for _ in 0..25 {
                run.app.update();
            }
            assert!(
                count(&mut run) <= 1,
                "a new proc cannot overlap the living spirit"
            );
            if count(&mut run) == 1 {
                break;
            }
        }
        assert_eq!(
            count(&mut run),
            1,
            "actual paid Rage must eventually summon the spirit"
        );
        assert!(crate::auras::has_aura(run.app.world(), actor, 1308647));
        for _ in 0..3 {
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                shared::components::PowerType::Rage,
                1000.0,
            );
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                192081,
                Some(actor),
                false,
            )
            .unwrap();
            for _ in 0..25 {
                run.app.update();
            }
        }
        assert_eq!(count(&mut run), 1, "active spirit blocks extra summons");
    }
    #[test]
    #[ignore = "requires Guardian MID2 profile and native world data"]
    fn guardian_blood_frenzy_gains_rage_only_with_owned_thrash() {
        let mut run = super::super::prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        let rage = |run: &super::super::Run| {
            crate::spell_cast::powers::power_value(
                run.app.world(),
                actor,
                shared::components::PowerType::Rage,
            )
            .unwrap()
            .0
        };
        for _ in 0..120 {
            run.app.update();
        }
        let before = rage(&run);
        for _ in 0..120 {
            run.app.update();
        }
        assert_eq!(
            rage(&run),
            before,
            "no Thrash target means no Blood Frenzy rage"
        );
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target,
                spell_id: 192090,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        // Out-of-combat decay drains each 3-rage tick within 2.4 s; check the peak.
        let mut peak = before;
        for _ in 0..120 {
            run.app.update();
            peak = peak.max(rage(&run));
        }
        assert!(
            peak > before,
            "an owned Thrash bleed must earn periodic rage"
        );
    }

    #[test]
    #[ignore = "requires Guardian MID2 profile and native world data"]
    fn guardian_ursocs_guidance_reduces_incarnation_for_paid_ironfur() {
        let mut run = super::super::prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        crate::spell_cast::scripts::start_hasted_cooldown(
            run.app.world_mut(),
            actor,
            102558,
            180000.0,
        );
        let remaining = |run: &super::super::Run| {
            let world = run.app.world();
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(102558)
                .unwrap();
            world
                .get::<crate::spell_cast::SpellHistory>(actor)
                .unwrap()
                .remaining_cooldown_ms(spell, crate::spell_cast::now(world))
        };
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            shared::components::PowerType::Rage,
            1000.0,
        );
        let before = remaining(&run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 192081, Some(actor), false)
            .unwrap();
        let after = remaining(&run);
        assert_eq!(
            before - after,
            1600,
            "40 paid rage divided by the native 25 rage/second divisor"
        );
    }

    #[test]
    #[ignore = "requires Guardian MID2 profile and SimC dungeon-dummy trace"]
    fn guardian_matted_fur_absorbs_real_dummy_damage() {
        let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("Guardian trace required");
        let report = super::super::super::tank_replay::replay_simc_with_healing(
            104,
            std::path::Path::new(&trace),
            300,
            70448,
            true,
        )
        .expect("Guardian tank replay");
        let absorbed = report["absorbs_by_spell"]["385787"].as_f64().unwrap_or(0.0);
        assert!(
            absorbed > 0.0,
            "defensive casts must earn a shield consumed by dummy hits: {report}"
        );
    }

    #[test]
    #[ignore = "requires Guardian MID2 profile and native world data"]
    fn guardian_ursocs_fury_shields_only_the_native_fraction_of_thrash_damage() {
        use crate::auras::ActiveAuras;
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let amount = |spell| {
            run.app
                .world()
                .get::<ActiveAuras>(actor)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == spell && aura.caster == actor)
                .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
                .map_or(0.0, |effect| effect.amount)
        };
        let multiplier =
            amount(377210) / 100.0 * (1.0 + amount(33873) / 100.0) * (1.0 + amount(377796) / 100.0);
        assert_eq!(amount(377210), 35.0);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
            .unwrap();
        let world = run.app.world();
        let damage = world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(77758)
                    && event.kind == CombatLogKind::Damage
            })
            .expect("real Thrash damage")
            .amount as f32;
        let shield = world
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 372505 && aura.caster == actor)
            .expect("earned Ursoc shield")
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .amount;
        assert!(damage > 0.0);
        assert!(
            (shield - damage * multiplier).abs() <= 2.0,
            "damage={damage} shield={shield} multiplier={multiplier}"
        );
    }

    #[test]
    #[ignore = "requires Guardian MID2 profile and native world data"]
    fn guardian_ironfur_stacks_expire_on_their_own_lifetimes() {
        let mut run = super::super::prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false)
            .unwrap();
        let ironfur = |run: &super::super::Run| {
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(actor)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 192081 && aura.caster == actor)
                .map_or(0, |aura| aura.stacks)
        };
        let armor = |run: &super::super::Run| {
            run.app
                .world()
                .get::<crate::stats::CharacterStats>(actor)
                .unwrap()
                .armor
        };
        // Three real casts 3 s apart; 0.05 s per update.
        let mut armor_by_stacks = Vec::new();
        for _ in 0..3 {
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                shared::components::PowerType::Rage,
                1000.0,
            );
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                192081,
                Some(actor),
                false,
            )
            .unwrap();
            run.app.update();
            armor_by_stacks.push(armor(&run));
            for _ in 0..59 {
                run.app.update();
            }
        }
        // Each stack lives 9 s (SimC trace: gain 6.044, decrement 15.044).
        for _ in 0..10 {
            run.app.update();
        }
        // 9.5 s: only the first cast has expired; a shared refresh would keep all three.
        assert_eq!(
            ironfur(&run),
            2,
            "casts at 3 s and 6 s are still live at 9.5 s"
        );
        for _ in 0..60 {
            run.app.update();
        }
        // 12.5 s: the cast at 3 s has expired too.
        assert_eq!(ironfur(&run), 1);
        assert!(
            (armor(&run) - armor_by_stacks[0]).abs() < 1.0,
            "one stack's armor remains"
        );
    }

    #[test]
    #[ignore = "requires Guardian MID2 profile and native world data"]
    fn guardian_thrash_and_moonfire_reduce_dummy_bleed_on_the_druid() {
        use crate::spell_cast::test_harness::ObservedCombatEvents;
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        // One 25,000 dummy bleed tick, measured with no shield to absorb it.
        let bleed = |run: &mut super::super::Run| {
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Cancelled,
                |aura| matches!(aura.spell_id, 372505 | 385787),
            );
            crate::spell_cast::effects::replay_simc_dummy_bleed(
                run.app.world_mut(),
                target,
                actor,
                25_000.0,
            );
            run.app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .rev()
                .find(|event| {
                    event.source == Some(target.to_bits())
                        && event.target == Some(actor.to_bits())
                        && event.kind == CombatLogKind::Damage
                })
                .expect("dummy bleed tick")
                .amount as f32
        };
        for spell in [77758, 8921] {
            for _ in 0..40 {
                run.app.update();
            }
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                spell,
                Some(target),
                false,
            )
            .unwrap();
        }
        for _ in 0..10 {
            run.app.update();
        }
        let stacks = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 192090 && aura.caster == actor)
            .expect("Bear Thrash bleed")
            .stacks;
        let debuffed = bleed(&mut run);
        crate::auras::remove_auras(
            run.app.world_mut(),
            target,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.caster == actor && matches!(aura.spell_id, 192090 | 164812),
        );
        let plain = bleed(&mut run);
        // SimC76f8118 104 trace: Rend and Tear Thrash -3% per stack, rank-two
        // Scintillating Moonlight Moonfire -12% on damage the target deals to the Druid.
        let expected = (1.0 - 0.03 * f32::from(stacks)) * 0.88;
        assert!(
            (debuffed / plain - expected).abs() < 0.002,
            "stacks={stacks} plain={plain} debuffed={debuffed} ratio={}",
            debuffed / plain
        );
    }

    #[test]
    #[ignore = "requires Guardian MID2 profile and SimC dungeon-dummy trace"]
    fn guardian_ursocs_fury_absorbs_real_dummy_damage() {
        let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("Guardian trace required");
        let report = super::super::super::tank_replay::replay_simc_with_healing(
            104,
            std::path::Path::new(&trace),
            300,
            70448,
            true,
        )
        .expect("Guardian tank replay");
        let absorbed = report["absorbs_by_spell"]["372505"].as_f64().unwrap_or(0.0);
        assert!(
            absorbed > 0.0,
            "actual Guardian attacks must earn a shield consumed by dummy hits: {report}"
        );
    }

    #[test]
    #[ignore = "requires Guardian MID2 profile and SimC dungeon-dummy trace"]
    fn guardian_replays_dungeon_dummy_with_literal_rotation() {
        let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("Guardian trace required");
        let report = super::super::super::tank_replay::replay_simc_with_healing(
            104,
            std::path::Path::new(&trace),
            300,
            70448,
            true,
        )
        .expect("Guardian tank replay");
        assert_eq!(report["spec"], 104);
        assert_eq!(report["rotation_status"], "registered_literal_simc");
        assert!(report["raw_damage"].as_f64().unwrap() > 0.0);
        assert!(report["active_mitigation_uptime"].as_f64().unwrap() > 0.0);
        assert!(
            report["periodic_damage_ticks_by_spell"]["192090"]
                .as_u64()
                .unwrap_or(0)
                > 0,
            "landed Bear Thrash must apply its real periodic bleed"
        );
        assert!(
            report["tank_casts"]["204066"].as_u64().unwrap_or(0) > 0,
            "Lunar Beam must cast on the enemy while its native destination remains caster-centered"
        );
    }

    #[test]
    #[ignore = "requires Guardian MID2 profile and native world data"]
    fn guardian_literal_adapter_executes_in_bear_form() {
        let mut run = super::super::prepare_registered_player(104, 1).unwrap();
        super::super::engage(&mut run);
        let start = run.app.world().resource::<bevy::prelude::Time>().elapsed();
        super::super::install_fixture_lifetimes(&mut run, start);
        let mut bear = false;
        let mut ironfur = false;
        for _ in 0..600 {
            super::super::drive_player(&mut run).unwrap();
            run.app.update();
            bear |= crate::auras::has_aura(run.app.world(), run.actor, 5487);
            ironfur |= crate::auras::has_aura(run.app.world(), run.actor, 192081);
        }
        assert!(bear, "native Guardian rotation enters Bear Form");
        assert!(
            ironfur,
            "native Guardian rotation earns and spends Rage on Ironfur"
        );
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(1261868)
                    && event.kind == shared::protocol::CombatLogKind::CastSuccess),
            "the selected Cat-form Heart of the Wild must cast on the enemy"
        );
        assert_eq!(
            super::super::tank_mitigation_auras(&run).unwrap(),
            &[192081, 22842, 22812, 200851]
        );
    }
}
