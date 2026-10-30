//! Fixture adapter over the production APL.
pub(super) use server_follower_apl::blood_tank_rotation::*;
use crate::simc_apl_native::Encounter;
use bevy::prelude::{Entity, World};


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

pub(super) const REPORT_KEY: Option<&str> = Some("blood_tank_rotation");
impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_BLOOD_76f8118"
    }
    fn tank_active_mitigation_auras(&self) -> Option<&'static [u32]> {
        Some(&[195181])
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
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_death_strike_sanguination_scales_with_actual_missing_health() {
        use shared::components::{Health, PowerType};
        use shared::protocol::CombatLogKind;
        fn hit(fraction: f32) -> i32 {
            let _seed = crate::random::seed_scope(70448);
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            assert!(crate::auras::has_aura(run.app.world(), actor, 326801));
            assert!(crate::auras::has_aura(run.app.world(), actor, 374261));
            let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
            health.current = health.max * fraction;
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                PowerType::RunicPower,
                1000.0,
            );
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                49998,
                Some(target),
                false,
            )
            .unwrap();
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .rev()
                .find(|event| {
                    event.spell_id == Some(49998)
                        && event.kind == CombatLogKind::Damage
                        && event.source == Some(actor.to_bits())
                })
                .expect("Death Strike hit")
                .amount
        }
        let full = hit(1.0) as f32;
        let half = hit(0.5) as f32;
        let low = hit(0.1) as f32;
        assert!(
            (half / full - 1.65).abs() < 0.01,
            "half={half}, full={full}"
        );
        assert!((low / full - 2.04).abs() < 0.01, "low={low}, full={full}");
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn bloody_reflection_requires_surviving_shield_and_respects_cooldown() {
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let attacker = run.targets[0];
        assert!(crate::auras::has_aura(run.app.world(), actor, 1279633));
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| matches!(aura.spell_id, 77535 | 207203),
        );
        crate::spell_cast::cast_triggered_with_effect0_amount(
            run.app.world_mut(),
            actor,
            77535,
            Some(actor),
            1,
            500_000.0,
        );
        let strike = |run: &mut super::super::Run| {
            crate::spell_cast::effects::replay_simc_dummy_strike(
                run.app.world_mut(),
                attacker,
                actor,
                10_000.0,
                false,
                false,
            )
        };
        let count = |run: &super::super::Run| {
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.spell_id == Some(1279656)
                        && event.kind == CombatLogKind::Damage
                        && event.target == Some(attacker.to_bits())
                })
                .count()
        };
        strike(&mut run);
        assert_eq!(count(&run), 1, "surviving shield reflects the actual hit");
        strike(&mut run);
        assert_eq!(count(&run), 1, "same-time hit is inside the 500ms cooldown");
        for _ in 0..12 {
            run.app.update();
        }
        strike(&mut run);
        assert_eq!(count(&run), 2, "reflection is available after cooldown");
        for _ in 0..12 {
            run.app.update();
        }
        let hit = server::test_support::healer_intake::Hit {
            tick: 0,
            source_guid: 0,
            pack: 0,
            target_slot: 0,
            school: 1,
            raw: 10_000.0,
            level_scaling_factor: 1.0,
            kind: server::test_support::healer_intake::HitKind::Melee {
                hand: 0,
                outcome: "Hit".into(),
                level_difference: 0,
                critical_multiplier: 1.0,
                weapon_speed: 2.0,
            },
        };
        crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit).unwrap();
        assert_eq!(count(&run), 3, "a fully absorbed white swing also reflects");
        for _ in 0..12 {
            run.app.update();
        }
        let parry = server::test_support::healer_intake::Hit {
            kind: server::test_support::healer_intake::HitKind::Melee {
                hand: 0,
                outcome: "Parry".into(),
                level_difference: 0,
                critical_multiplier: 1.0,
                weapon_speed: 2.0,
            },
            ..hit
        };
        crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &parry).unwrap();
        assert_eq!(
            count(&run),
            4,
            "a parry with surviving Blood Shield reflects"
        );
        for _ in 0..12 {
            run.app.update();
        }
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| matches!(aura.spell_id, 77535 | 207203),
        );
        strike(&mut run);
        assert_eq!(count(&run), 4, "no Blood Shield means no reflection");
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_inevitable_scales_plague_with_target_missing_health() {
        use shared::components::Health;
        use shared::protocol::CombatLogKind;
        fn tick(health_fraction: f32, talent: bool) -> i32 {
            let _seed = crate::random::seed_scope(70448);
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            assert!(crate::auras::has_aura(run.app.world(), actor, 1280658));
            if !talent {
                crate::auras::remove_auras(
                    run.app.world_mut(),
                    actor,
                    crate::auras::RemoveReason::Cancelled,
                    |aura| aura.spell_id == 1280658,
                );
            }
            let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
            health.current = health.max * health_fraction;
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: actor,
                    target,
                    spell_id: 55078,
                    effect_mask: 1,
                    source: crate::auras::AuraSource::Cast,
                },
            );
            for _ in 0..100 {
                run.app.update();
                let events = &run
                    .app
                    .world()
                    .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                    .0;
                if let Some(event) = events.iter().find(|event| {
                    event.spell_id == Some(55078)
                        && event.kind == CombatLogKind::Damage
                        && event.periodic
                }) {
                    return event.amount;
                }
            }
            panic!("Blood Plague did not tick");
        }
        let full = tick(1.0, true) as f32;
        let half = tick(0.55, true) as f32;
        let capped = tick(0.10, true) as f32;
        let unselected = tick(0.10, false) as f32;
        assert!((half / full - 1.15).abs() < 0.01, "full={full} half={half}");
        assert!(
            (capped / full - 1.30).abs() < 0.01,
            "full={full} capped={capped}"
        );
        assert!(
            (unselected / full - 1.0).abs() < 0.01,
            "full={full} without talent={unselected}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_plague_direct_heal_uses_tick_damage_and_independent_crit() {
        use shared::components::Health;
        use shared::protocol::CombatLogKind;
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        let mut health = run.app.world_mut().get_mut::<Health>(actor).unwrap();
        health.current = health.max * 0.1;
        for _ in 0..5 {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: actor,
                    target,
                    spell_id: 55078,
                    effect_mask: 1,
                    source: crate::auras::AuraSource::Cast,
                },
            );
            for _ in 0..250 {
                run.app.update();
            }
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let relevant: Vec<_> = events
            .iter()
            .filter(|event| {
                event.spell_id == Some(55078)
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Heal)
            })
            .collect();
        let mut pairs = 0;
        let mut independent = false;
        for pair in relevant.windows(2) {
            let (damage, heal) = (pair[0], pair[1]);
            if damage.kind != CombatLogKind::Damage || heal.kind != CombatLogKind::Heal {
                continue;
            }
            pairs += 1;
            assert!(!heal.periodic, "SimC emits a direct heal");
            if !heal.crit {
                assert!(
                    (heal.amount as f32 / damage.amount as f32 - 1.85).abs() < 0.01,
                    "damage={} heal={}",
                    damage.amount,
                    heal.amount
                );
            }
            independent |= damage.crit != heal.crit;
        }
        assert!(
            pairs >= 10,
            "expected real plague ticks and heals, got {pairs}"
        );
        assert!(
            independent,
            "heal crit must be rolled independently of the damage crit"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_death_strike_grants_voracious_leech() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        assert!(crate::auras::has_aura(run.app.world(), actor, 273953));
        let before = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .leech_pct;
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            shared::components::PowerType::RunicPower,
            1000.0,
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49998, Some(target), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 274009));
        let after = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .leech_pct;
        assert!(
            (after - before - 15.0).abs() < 0.01,
            "Death Strike grants the native 15% Leech: {before} -> {after}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_gift_duration_includes_everlasting_bond() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..190 {
            run.app.update();
        }
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 434153),
            "Everlasting Bond extends the Gift past eight seconds"
        );
        for _ in 0..70 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 434153),
            "Gift expires after the twelve-second weapon duration"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn vampiric_strike_heals_blood_player_from_native_health_percentage() {
        use shared::components::Health;
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        let max = run.app.world().get::<Health>(actor).unwrap().max;
        run.app
            .world_mut()
            .get_mut::<Health>(actor)
            .unwrap()
            .current = max * 0.5;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 433895, Some(target), false)
            .unwrap();
        for _ in 0..10 {
            run.app.update();
        }
        let heals = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            heals.iter().any(|event| event.spell_id == Some(434422)
                && event.kind == CombatLogKind::Heal
                && event.amount >= (max * 0.01) as i32
                && event.overflow < event.amount),
            "Vampiric Strike must heal from its native percentage"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn health_defenses_proc_only_below_threshold_and_obey_cooldowns() {
        use shared::components::Health;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let target = run.actor;
        let source = run.targets[0];
        let max = run.app.world().get::<Health>(target).unwrap().max;
        let strike = |run: &mut super::super::Run| {
            crate::auras::remove_auras(
                run.app.world_mut(),
                target,
                crate::auras::RemoveReason::Cancelled,
                |aura| matches!(aura.spell_id, 77535 | 207203),
            );
            crate::spell_cast::effects::replay_simc_dummy_strike(
                run.app.world_mut(),
                source,
                target,
                max * 0.05,
                false,
                false,
            );
            for _ in 0..4 {
                run.app.update();
            }
        };
        strike(&mut run);
        assert!(!crate::auras::has_aura(run.app.world(), target, 326808));
        assert!(!crate::auras::has_aura(run.app.world(), target, 313015));
        run.app
            .world_mut()
            .get_mut::<Health>(target)
            .unwrap()
            .current = max * 0.30;
        strike(&mut run);
        assert!(
            crate::auras::has_aura(run.app.world(), target, 326808),
            "damage below 35% grants Sanguination"
        );
        assert!(
            crate::auras::has_aura(run.app.world(), target, 326809),
            "Sanguination applies Satiated"
        );
        assert!(!crate::auras::has_aura(run.app.world(), target, 313015));
        run.app
            .world_mut()
            .get_mut::<Health>(target)
            .unwrap()
            .current = max * 0.15;
        strike(&mut run);
        assert!(
            crate::auras::has_aura(run.app.world(), target, 313015),
            "damage below 20% grants Recently Failed"
        );
        let heals = |run: &super::super::Run| {
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.spell_id == Some(313010)
                        && event.kind == shared::protocol::CombatLogKind::Heal
                })
                .count()
        };
        assert_eq!(heals(&run), 1);
        run.app
            .world_mut()
            .get_mut::<Health>(target)
            .unwrap()
            .current = max * 0.15;
        strike(&mut run);
        assert_eq!(
            heals(&run),
            1,
            "Recently Failed prevents another Failsafe heal"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn permafrost_white_hits_create_consumable_frost_shield() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        assert!(crate::auras::has_aura(run.app.world(), actor, 207200));
        super::super::engage(&mut run);
        for _ in 0..200 {
            run.app.update();
            if crate::auras::has_aura(run.app.world(), actor, 207203) {
                break;
            }
        }
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 207203),
            "a real white hit must create Frost Shield"
        );
        let (_, absorbed) =
            crate::auras::absorb_damage(run.app.world_mut(), actor, None, 1, 1_000_000.0);
        assert!(
            absorbed > 0.0,
            "incoming physical damage must consume the shield"
        );
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 207200 || aura.spell_id == 207203,
        );
        for _ in 0..200 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 207203),
            "white hits without the talent must not create a shield"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_rune_weapons_reduce_actual_incoming_strike() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let source = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 1264407));
        let max = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .max;
        crate::spell_cast::effects::replay_simc_dummy_strike(
            run.app.world_mut(),
            source,
            actor,
            100_000.0,
            false,
            false,
        );
        let with = max
            - run
                .app
                .world()
                .get::<shared::components::Health>(actor)
                .unwrap()
                .current;
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 1264407,
        );
        for _ in 0..3 {
            run.app.update();
        }
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(actor)
            .unwrap();
        health.current = health.max;
        let max = health.max;
        crate::spell_cast::effects::replay_simc_dummy_strike(
            run.app.world_mut(),
            source,
            actor,
            100_000.0,
            false,
            false,
        );
        let without = max
            - run
                .app
                .world()
                .get::<shared::components::Health>(actor)
                .unwrap()
                .current;
        assert!(
            with < without * 0.90,
            "two weapons reduce the actual incoming hit: with={with}, without={without}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_profile_potion_uses_native_item_and_strength_buff() {
        use super::super::literal_rotation::{
            equipment::Equipment,
            execution::{self, NativeAction, Prepared},
        };
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let world = run.app.world();
        let before = world
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .primary
            .strength;
        let potion = world
            .get::<super::super::profile_effects::Potion>(actor)
            .expect("configured potion");
        assert_eq!((potion.0.item_id, potion.0.item_level), (241292, 295));
        let mut bindings = crate::simc_apl_native::Bindings::default();
        let equipment = Equipment::load(world, actor, &mut bindings).unwrap();
        let row = server::simc_apl::Action {
            source_row: None,
            name: "potion".into(),
            target: None,
            options: Default::default(),
        };
        let prepared = Prepared::read(
            run.app.world_mut(),
            actor,
            &run.targets,
            &equipment,
            std::iter::empty::<NativeAction>(),
            false,
        )
        .unwrap();
        let item = prepared
            .execution(&row, &equipment)
            .unwrap()
            .expect("potion is ready");
        execution::execute(run.app.world_mut(), actor, item).unwrap();
        super::super::preparation::finish_pending_setup(&mut run).unwrap();
        let world = run.app.world();
        assert!(crate::auras::has_aura(world, actor, 1236998));
        let after = world
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .primary
            .strength;
        assert!(
            after > before,
            "native potion increases Strength: {before} -> {after}"
        );
        let prepared = Prepared::read(
            run.app.world_mut(),
            actor,
            &run.targets,
            &equipment,
            std::iter::empty::<NativeAction>(),
            false,
        )
        .unwrap();
        assert!(
            prepared.execution(&row, &equipment).unwrap().is_none(),
            "potion cannot be reused"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_dance_rolls_once_per_rune_payment() {
        fn first_spawn(runes: f32) -> u32 {
            let _seed = crate::random::seed_scope(70448);
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            super::super::engage(&mut run);
            for attempt in 1..=30 {
                crate::spell_cast::script_registry::after_payment(
                    run.app.world_mut(),
                    actor,
                    &[(
                        crate::spell_cast::CostPool::Power(shared::components::PowerType::Runes),
                        runes,
                    )],
                );
                let world = run.app.world_mut();
                if world
                    .query::<&crate::spell_cast::summon::SummonedCreature>()
                    .iter(world)
                    .any(|summon| summon.owner == actor && summon.spell_id == 1264353)
                {
                    return attempt;
                }
            }
            panic!("no Dance weapon after 30 payments")
        }
        let paid = first_spawn(1.0);
        assert_eq!(
            paid,
            first_spawn(2.0),
            "each payment gets one roll regardless of rune count"
        );
        assert_eq!(
            paid,
            first_spawn(0.0),
            "a fully discounted rune cost still gets a roll"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_free_heart_strikes_can_summon_dance_weapon() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            shared::components::PowerType::Runes,
            -6.0,
        );
        for _ in 0..30 {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id: 1264568,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                206930,
                Some(target),
                false,
            )
            .unwrap();
            let world = run.app.world_mut();
            if world
                .query::<&crate::spell_cast::summon::SummonedCreature>()
                .iter(world)
                .any(|summon| summon.owner == actor && summon.spell_id == 1264353)
            {
                return;
            }
            for _ in 0..30 {
                run.app.update();
            }
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                shared::components::PowerType::Runes,
                -6.0,
            );
        }
        panic!("free Heart Strikes must be able to summon a Dance weapon");
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_dance_weapon_buffs_follow_real_summon_lifetimes() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        super::super::engage(&mut run);
        let summon = |run: &mut super::super::Run| {
            assert!(crate::spell_cast::try_cast_triggered(
                run.app.world_mut(),
                actor,
                1264353,
                Some(actor),
                1
            ));
            for _ in 0..4 {
                run.app.update();
            }
            let world = run.app.world_mut();
            world
                .query::<&crate::spell_cast::summon::SummonedCreature>()
                .iter(world)
                .filter(|summon| summon.owner == actor && summon.spell_id == 1264353)
                .filter_map(|summon| summon.expires_at)
                .max_by(f64::total_cmp)
                .unwrap()
        };
        let first_end = summon(&mut run);
        let first_buff_end = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 81256)
            .unwrap()
            .expires_at
            .unwrap();
        assert!(
            (first_buff_end - first_end).abs() < 0.06,
            "weapon {first_end}, parry buff {first_buff_end}"
        );
        for _ in 0..40 {
            run.app.update();
        }
        let second_end = summon(&mut run);
        let stack_count = |run: &super::super::Run| {
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(actor)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 1264407)
                .map_or(0, |aura| aura.stacks)
        };
        assert_eq!(stack_count(&run), 2);
        while crate::spell_cast::now(run.app.world()) < first_end + 0.15 {
            run.app.update();
        }
        assert_eq!(
            stack_count(&run),
            1,
            "first weapon's Strength stack expires with it"
        );
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 81256),
            "second weapon keeps parry buff"
        );
        while crate::spell_cast::now(run.app.world()) < second_end + 0.15 {
            run.app.update();
        }
        assert_eq!(
            stack_count(&run),
            0,
            "last Strength stack expires with the last weapon"
        );
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 81256),
            "parry buff ends with the last weapon"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_rune_spend_can_summon_dance_weapon_and_enable_parry_buff() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        let mut found = false;
        for _ in 0..30 {
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                shared::components::PowerType::Runes,
                6.0,
            );
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                206930,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..30 {
                run.app.update();
            }
            let world = run.app.world_mut();
            found = world
                .query::<&crate::spell_cast::summon::SummonedCreature>()
                .iter(world)
                .any(|summon| summon.owner == actor && summon.spell_id == 1264353);
            if found {
                break;
            }
        }
        assert!(
            found,
            "rune spending summons a real Dance of Midnight weapon"
        );
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 81256),
            "real weapon enables the DRW parry buff"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_ossuary_tracks_bone_shield_and_death_strike_cost() {
        use server::test_support::healer_intake::{Hit, HitKind};
        use shared::components::PowerType;
        fn scenario(
            stacks: usize,
            lose_stack: bool,
            selected: bool,
            improved: bool,
        ) -> (bool, f32) {
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            if !selected {
                let known = run
                    .app
                    .world()
                    .get::<crate::class_progression::KnownSpells>(actor)
                    .unwrap()
                    .iter()
                    .filter(|&id| id != 219786)
                    .collect();
                run.app
                    .world_mut()
                    .entity_mut(actor)
                    .insert(crate::class_progression::KnownSpells::fixture(known));
            }
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Cancelled,
                |aura| {
                    matches!(aura.spell_id, 195181 | 219788)
                        || (!improved && aura.spell_id == 374277)
                },
            );
            for _ in 0..stacks {
                crate::auras::apply_aura(
                    run.app.world_mut(),
                    &crate::auras::AuraApplication {
                        caster: actor,
                        target: actor,
                        spell_id: 195181,
                        effect_mask: u32::MAX,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            if lose_stack {
                let hit = Hit {
                    tick: 0,
                    source_guid: 0,
                    pack: 0,
                    target_slot: 0,
                    school: 1,
                    raw: 1000.0,
                    level_scaling_factor: 1.0,
                    kind: HitKind::Melee {
                        hand: 0,
                        outcome: "Hit".into(),
                        level_difference: 0,
                        critical_multiplier: 1.0,
                        weapon_speed: 2.0,
                    },
                };
                crate::combat::replay_healer_hit(run.app.world_mut(), target, actor, &hit).unwrap();
                run.app.update();
            }
            let active = crate::auras::has_aura(run.app.world(), actor, 219788);
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                PowerType::RunicPower,
                1000.0,
            );
            let before = crate::spell_cast::powers::power_value(
                run.app.world(),
                actor,
                PowerType::RunicPower,
            )
            .unwrap()
            .0;
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                49998,
                Some(target),
                false,
            )
            .unwrap();
            let after = crate::spell_cast::powers::power_value(
                run.app.world(),
                actor,
                PowerType::RunicPower,
            )
            .unwrap()
            .0;
            (active, before - after)
        }
        assert_eq!(scenario(4, false, true, true), (false, 400.0));
        assert_eq!(scenario(5, false, true, true), (true, 350.0));
        assert_eq!(scenario(5, true, true, true), (false, 400.0));
        assert_eq!(scenario(5, false, false, true), (false, 400.0));
        assert_eq!(scenario(5, false, true, false), (true, 400.0));
        assert_eq!(scenario(4, false, true, false), (false, 450.0));
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_rune_spenders_earn_rune_mastery_strength() {
        use shared::components::PowerType;
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        assert!(crate::auras::has_aura(run.app.world(), actor, 374574));
        for _ in 0..30 {
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                PowerType::Runes,
                6.0,
            );
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                206930,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..4 {
                run.app.update();
            }
            if crate::auras::has_aura(run.app.world(), actor, 374585) {
                let aura = run
                    .app
                    .world()
                    .get::<crate::auras::ActiveAuras>(actor)
                    .unwrap()
                    .iter()
                    .find(|aura| aura.spell_id == 374585)
                    .unwrap();
                assert_eq!(
                    aura.effects[0].amount, 6.0,
                    "rank-two Rune Mastery Strength"
                );
                let buffed = run
                    .app
                    .world()
                    .get::<crate::stats::CharacterStats>(actor)
                    .unwrap()
                    .primary
                    .strength;
                crate::auras::remove_auras(
                    run.app.world_mut(),
                    actor,
                    crate::auras::RemoveReason::Cancelled,
                    |aura| aura.spell_id == 374585,
                );
                run.app.update();
                let unbuffed = run
                    .app
                    .world()
                    .get::<crate::stats::CharacterStats>(actor)
                    .unwrap()
                    .primary
                    .strength;
                assert!(buffed > unbuffed, "{unbuffed} -> {buffed}");
                return;
            }
            for _ in 0..40 {
                run.app.update();
            }
        }
        panic!("Blood rune spenders must earn Rune Mastery");
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_rune_mastery_rolls_on_free_and_costed_triggered_strikes() {
        use shared::components::PowerType;
        for triggered in [false, true] {
            let _seed = crate::random::seed_scope(70448);
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            let mut earned = false;
            for _ in 0..30 {
                crate::spell_cast::powers::change_power(
                    run.app.world_mut(),
                    actor,
                    PowerType::Runes,
                    6.0,
                );
                if triggered {
                    assert!(crate::spell_cast::cast_costed_trigger_with_context(
                        run.app.world_mut(),
                        actor,
                        433895,
                        Some(target),
                        1,
                        None,
                        None
                    ));
                } else {
                    crate::auras::apply_aura(
                        run.app.world_mut(),
                        &crate::auras::AuraApplication {
                            caster: actor,
                            target: actor,
                            spell_id: 1264568,
                            effect_mask: u32::MAX,
                            source: crate::auras::AuraSource::Cast,
                        },
                    );
                    let before = crate::spell_cast::powers::power_value(
                        run.app.world(),
                        actor,
                        PowerType::Runes,
                    )
                    .unwrap()
                    .0;
                    crate::spell_cast::creature_cast(
                        run.app.world_mut(),
                        actor,
                        433895,
                        Some(target),
                        false,
                    )
                    .unwrap();
                    assert_eq!(
                        crate::spell_cast::powers::power_value(
                            run.app.world(),
                            actor,
                            PowerType::Runes
                        )
                        .unwrap()
                        .0,
                        before
                    );
                }
                for _ in 0..4 {
                    run.app.update();
                }
                if crate::auras::has_aura(run.app.world(), actor, 374585) {
                    earned = true;
                    break;
                }
                for _ in 0..40 {
                    run.app.update();
                }
            }
            assert!(earned, "Rune Mastery must proc for triggered={triggered}");
        }
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn bloodied_blade_parries_grant_strength_then_strike() {
        use server::test_support::healer_intake::{Hit, HitKind};
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let source = run.targets[0];
        super::super::engage(&mut run);
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| matches!(aura.spell_id, 460499 | 460500 | 433899),
        );
        let hit = Hit {
            tick: 0,
            source_guid: 0,
            pack: 0,
            target_slot: 0,
            school: 1,
            raw: 100.0,
            level_scaling_factor: 1.0,
            kind: HitKind::Melee {
                hand: 0,
                outcome: "Parry".into(),
                level_difference: 0,
                critical_multiplier: 1.0,
                weapon_speed: 2.0,
            },
        };
        let before = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .primary
            .strength;
        for _ in 0..8 {
            crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &hit).unwrap();
            for _ in 0..12 {
                run.app.update();
            }
        }
        let aura = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 460499)
            .expect("eight parries grant stacks");
        assert_eq!(aura.stacks, 8);
        assert_eq!(aura.effects[0].amount, 0.5);
        let stacked = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .primary
            .strength;
        assert!(stacked > before * 1.03, "{before} -> {stacked}");
        crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &hit).unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(!crate::auras::has_aura(run.app.world(), actor, 460499));
        assert!(crate::auras::has_aura(run.app.world(), actor, 460500));
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events
                .iter()
                .any(|event| event.spell_id == Some(460501) && event.amount > 0),
            "final parry causes Heart Strike damage"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn bloodied_blade_vampiric_strike_requires_and_spends_a_rune() {
        use server::test_support::healer_intake::{Hit, HitKind};
        use shared::components::PowerType;
        for funded in [false, true] {
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let source = run.targets[0];
            super::super::engage(&mut run);
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Cancelled,
                |aura| matches!(aura.spell_id, 460499 | 460500 | 433899 | 434153),
            );
            for id in [
                460499, 460499, 460499, 460499, 460499, 460499, 460499, 460499, 433899,
            ] {
                crate::auras::apply_aura(
                    run.app.world_mut(),
                    &crate::auras::AuraApplication {
                        caster: actor,
                        target: actor,
                        spell_id: id,
                        effect_mask: u32::MAX,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            if !funded {
                crate::spell_cast::powers::change_power(
                    run.app.world_mut(),
                    actor,
                    PowerType::Runes,
                    -6.0,
                );
            }
            let before =
                crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Runes)
                    .unwrap()
                    .0;
            let hit = Hit {
                tick: 0,
                source_guid: 0,
                pack: 0,
                target_slot: 0,
                school: 1,
                raw: 100.0,
                level_scaling_factor: 1.0,
                kind: HitKind::Melee {
                    hand: 0,
                    outcome: "Parry".into(),
                    level_difference: 0,
                    critical_multiplier: 1.0,
                    weapon_speed: 2.0,
                },
            };
            crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &hit).unwrap();
            for _ in 0..4 {
                run.app.update();
            }
            assert!(crate::auras::has_aura(run.app.world(), actor, 460500));
            assert_eq!(
                crate::auras::has_aura(run.app.world(), actor, 433899),
                !funded
            );
            let after =
                crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Runes)
                    .unwrap()
                    .0;
            if funded {
                assert_eq!(after, before - 1.0);
            } else {
                assert_eq!(after, before);
            }
            let events = &run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0;
            assert_eq!(
                events
                    .iter()
                    .any(|event| event.spell_id == Some(433895) && event.amount > 0),
                funded
            );
        }
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_parry_during_rune_weapon_grants_free_vampiric_strike() {
        use server::test_support::healer_intake::{Hit, HitKind};
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let source = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        let hit = Hit {
            tick: 0,
            source_guid: 0,
            pack: 0,
            target_slot: 0,
            school: 1,
            raw: 100.0,
            level_scaling_factor: 1.0,
            kind: HitKind::Melee {
                hand: 0,
                outcome: "Parry".into(),
                level_difference: 0,
                critical_multiplier: 1.0,
                weapon_speed: 2.0,
            },
        };
        crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &hit).unwrap();
        run.app.update();
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 1264568),
            "actual parry earns Dance of Midnight"
        );
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            shared::components::PowerType::Runes,
            -6.0,
        );
        let strike = crate::spell_cast::knowledge::resolve_action(run.app.world(), actor, 206930)
            .unwrap()
            .unwrap();
        assert_eq!(strike, 433895);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, strike, Some(source), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 1264568),
            "Vampiric Strike consumes the free-cast proc"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_beast_attacks_and_eruptions_use_real_owned_summon() {
        use shared::components::{Npc, UnitSummonedBy};
        use shared::protocol::CombatLogKind;
        let _seed = crate::random::seed_scope(70448);
        for targets in [1, 3] {
            let mut run = super::super::prepare_registered_player(250, targets).unwrap();
            let owner = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            for &target in &run.targets {
                run.app
                    .world_mut()
                    .entity_mut(target)
                    .insert(shared::components::UnitLevel(93));
            }
            crate::spell_cast::creature_cast(run.app.world_mut(), owner, 49028, Some(owner), false)
                .unwrap();
            for _ in 0..4 {
                run.app.update();
            }
            let world = run.app.world_mut();
            let beast = world
                .query::<(bevy::prelude::Entity, &Npc, &UnitSummonedBy)>()
                .iter(world)
                .find(|(_, npc, summoned)| {
                    npc.template_id == 217228 && summoned.0 == owner.to_bits()
                })
                .expect("real owned Blood Beast")
                .0;
            assert_eq!(
                world.get::<shared::components::UnitLevel>(beast).unwrap().0,
                90
            );
            let owner_stats = world.get::<crate::stats::CharacterStats>(owner).unwrap();
            let owner_ap = owner_stats.total_attack_power(
                crate::stats::WeaponAttackType::MainHand,
                world.get::<crate::stats::StatModifiers>(owner),
            );
            let (_, beast_ap) = crate::spell_cast::effects::power_stats(world, beast, false);
            assert!((beast_ap - 3.0 * owner_ap).abs() < 0.01);
            for _ in 0..220 {
                run.app.update();
            }
            let events = &run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0;
            for spell in [None, Some(434574)] {
                assert!(
                    events
                        .iter()
                        .any(|event| event.source == Some(beast.to_bits())
                            && event.target == Some(target.to_bits())
                            && event.spell_id == spell
                            && event.kind == CombatLogKind::Damage
                            && event.amount > 0),
                    "Blood Beast damage {spell:?}"
                );
            }
            let casts = events
                .iter()
                .filter(|event| {
                    event.source == Some(beast.to_bits())
                        && event.spell_id == Some(434574)
                        && event.kind == CombatLogKind::CastSuccess
                })
                .count();
            assert_eq!(casts, 3, "Corrupted Blood begins on cooldown");
            let swings = events
                .iter()
                .filter(|event| {
                    event.source == Some(beast.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id.is_none()
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                })
                .count();
            assert!(
                swings >= 8,
                "Blood Beast should keep swinging for its ten-second lifetime, got {swings}"
            );
            assert!(
                events
                    .iter()
                    .any(|event| event.source == Some(owner.to_bits())
                        && event.spell_id == Some(434246)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0),
                "Blood Beast expiration must erupt"
            );
        }
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_sanguinary_burst_increases_with_real_runic_power_spending() {
        use shared::components::PowerType;
        use shared::protocol::CombatLogKind;
        fn burst(spell: Option<u32>, improved: bool) -> (f32, f32) {
            let _seed = crate::random::seed_scope(70448);
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            run.app
                .world_mut()
                .entity_mut(actor)
                .insert(crate::spell_cast::ObservedResourcePayments::default());
            if !improved {
                crate::auras::remove_auras(
                    run.app.world_mut(),
                    actor,
                    crate::auras::RemoveReason::Cancelled,
                    |aura| aura.spell_id == 374277 && aura.caster == actor,
                );
            }
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id: 77535,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                PowerType::RunicPower,
                1000.0,
            );
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
                .unwrap();
            for _ in 0..40 {
                run.app.update();
            }
            if let Some(spell) = spell {
                crate::spell_cast::creature_cast(
                    run.app.world_mut(),
                    actor,
                    spell,
                    Some(target),
                    false,
                )
                .unwrap();
            }
            for _ in 0..116 {
                run.app.update();
            }
            let _burst_seed = crate::random::seed_scope(500);
            for _ in 0..24 {
                run.app.update();
            }
            let event = run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .find(|event| {
                    event.source == Some(actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(1263786)
                        && event.kind == CombatLogKind::Damage
                })
                .expect("Sanguinary Burst damage");
            let paid = run
                .app
                .world()
                .get::<crate::spell_cast::ObservedResourcePayments>(actor)
                .unwrap()
                .payments
                .iter()
                .filter(|(id, pool, _)| {
                    *id == 49998
                        && *pool == crate::spell_cast::CostPool::Power(PowerType::RunicPower)
                })
                .map(|(_, _, amount)| *amount / 10.0)
                .sum();
            (
                event.amount as f32 / if event.crit { 2.0808 } else { 1.0 },
                paid,
            )
        }
        let (baseline, _) = burst(None, true);
        let (generated, _) = burst(Some(433895), true);
        assert!(
            (generated / baseline - 1.0).abs() < 0.10,
            "generated={generated} baseline={baseline}"
        );
        let (spent, paid) = burst(Some(49998), true);
        let (control, control_paid) = burst(Some(49998), false);
        assert!(paid > 0.0 && control_paid > paid);
        let expected = (1.0 + (paid + 5.0) * 0.02) / (1.0 + control_paid * 0.02);
        assert!(
            (spent / control - expected).abs() < 0.03,
            "spent={spent} control={control} paid={paid} control_paid={control_paid}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_mist_damages_generates_power_and_expires_into_burst() {
        use shared::components::PowerType;
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            PowerType::RunicPower,
            -1000.0,
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        assert!(crate::auras::has_aura(run.app.world(), actor, 1263729));
        for _ in 0..40 {
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
                .any(|event| event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1263752)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "Blood Mist deals periodic area damage"
        );
        let power =
            crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::RunicPower)
                .unwrap()
                .0;
        assert!(power > 0.0, "Blood Mist generates Runic Power");
        for _ in 0..140 {
            run.app.update();
        }
        assert!(!crate::auras::has_aura(run.app.world(), actor, 1263729));
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1263786)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "Blood Mist expiry fires Sanguinary Burst"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn equipped_prismatic_iris_proc_deals_real_periodic_damage() {
        use shared::protocol::CombatLogKind;
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        assert!(crate::auras::has_aura(run.app.world(), actor, 1251906));
        assert!(crate::auras::has_aura(run.app.world(), actor, 1252383));
        for _ in 0..2000 {
            super::super::drive_literal_tick(&mut run).unwrap();
            run.app.update();
            if run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| {
                    event.source == Some(actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(1252389)
                        && event.kind == CombatLogKind::Damage
                        && event.periodic
                        && event.amount > 0
                })
            {
                return;
            }
        }
        panic!("equipped Iris must proc and deal periodic damage");
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_rune_weapon_heart_strike_uses_owners_ground_cleave() {
        use shared::components::{Npc, UnitSummonedBy};
        use shared::protocol::CombatLogKind;
        for inside in [false, true] {
            let mut run = super::super::prepare_registered_player(250, 3).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
                .unwrap();
            for _ in 0..40 {
                run.app.update();
            }
            if inside {
                crate::spell_cast::creature_cast(
                    run.app.world_mut(),
                    actor,
                    43265,
                    Some(actor),
                    false,
                )
                .unwrap();
                for _ in 0..40 {
                    run.app.update();
                }
                assert!(crate::auras::has_aura(run.app.world(), actor, 188290));
            }
            let world = run.app.world_mut();
            let weapons: Vec<_> = world
                .query::<(bevy::prelude::Entity, &Npc, &UnitSummonedBy)>()
                .iter(world)
                .filter(|(_, npc, owner)| npc.template_id == 27893 && owner.0 == actor.to_bits())
                .map(|(entity, _, _)| entity)
                .collect();
            assert_eq!(weapons.len(), 2);
            crate::spell_cast::creature_cast(world, actor, 206930, Some(target), false).unwrap();
            for _ in 0..4 {
                run.app.update();
            }
            let events = &run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0;
            for weapon in weapons {
                let targets: std::collections::BTreeSet<_> = events
                    .iter()
                    .filter(|event| {
                        event.source == Some(weapon.to_bits())
                            && event.spell_id == Some(228645)
                            && event.kind == CombatLogKind::Damage
                            && event.amount > 0
                    })
                    .filter_map(|event| event.target)
                    .collect();
                assert_eq!(
                    targets.len(),
                    if inside { 3 } else { 2 },
                    "weapon cleave; inside={inside}"
                );
                assert!(targets.contains(&target.to_bits()));
            }
        }
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_strikes_cleave_more_targets_in_death_and_decay() {
        use shared::protocol::CombatLogKind;
        for (spell, inside) in [
            (206930, false),
            (433895, false),
            (206930, true),
            (433895, true),
        ] {
            let mut run = super::super::prepare_registered_player(250, 3).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            if inside {
                crate::spell_cast::creature_cast(
                    run.app.world_mut(),
                    actor,
                    43265,
                    Some(actor),
                    false,
                )
                .unwrap();
                for _ in 0..40 {
                    run.app.update();
                }
                assert!(crate::auras::has_aura(run.app.world(), actor, 188290));
            }
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                spell,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..4 {
                run.app.update();
            }
            let events = &run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0;
            let targets: std::collections::BTreeSet<_> = events
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.spell_id == Some(spell)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                })
                .filter_map(|event| event.target)
                .collect();
            assert_eq!(
                targets.len(),
                if inside { 3 } else { 2 },
                "spell {spell}, inside Death and Decay: {inside}"
            );
            assert!(
                targets.contains(&target.to_bits()),
                "primary target must be hit"
            );
        }
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_infliction_scales_with_remaining_plague_duration() {
        use shared::protocol::CombatLogKind;
        fn sample(duration: u32, blood_modifier: bool, health_fraction: f32) -> i32 {
            let _seed = crate::random::seed_scope(70448);
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            let mut health = run
                .app
                .world_mut()
                .get_mut::<shared::components::Health>(target)
                .unwrap();
            health.current = health.max * health_fraction;
            assert!(crate::auras::has_aura(run.app.world(), actor, 137008));
            if !blood_modifier {
                crate::auras::remove_auras(
                    run.app.world_mut(),
                    actor,
                    crate::auras::RemoveReason::Cancelled,
                    |aura| aura.spell_id == 137008 && aura.caster == actor,
                );
                crate::auras::apply_aura(
                    run.app.world_mut(),
                    &crate::auras::AuraApplication {
                        caster: actor,
                        target: actor,
                        spell_id: 137008,
                        effect_mask: u32::MAX & !(1 << 13),
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: actor,
                    target,
                    spell_id: 55078,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
            assert!(crate::auras::set_owned_duration(
                run.app.world_mut(),
                actor,
                target,
                55078,
                duration
            ));
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                433895,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..4 {
                run.app.update();
            }
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .find(|event| {
                    event.source == Some(actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(434144)
                        && event.kind == CombatLogKind::Damage
                })
                .expect("Infliction damage")
                .amount
        }
        let short = sample(6000, true, 1.0);
        let long = sample(12000, true, 1.0);
        assert!(
            long > short * 3 / 2 && long < short * 3,
            "remaining disease: {short} vs {long}"
        );
        let unmodified = sample(12000, false, 1.0);
        assert!(
            (long as f32 / unmodified as f32 - 2.0).abs() < 0.02,
            "Blood passive must add its native 10 percentage points: {long} vs {unmodified}"
        );
        let low_health = sample(12000, true, 0.10);
        assert!(
            (low_health as f32 / long as f32 - 1.30).abs() < 0.02,
            "Inevitable must scale Infliction's remaining disease: {long} vs {low_health}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_infliction_requires_owned_plague_and_extends_its_deadline() {
        use shared::protocol::CombatLogKind;
        for with_plague in [false, true] {
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            assert!(
                run.app
                    .world()
                    .get::<crate::class_progression::KnownSpells>(actor)
                    .unwrap()
                    .contains(434143)
            );
            crate::auras::remove_auras(
                run.app.world_mut(),
                target,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == 55078 && aura.caster == actor,
            );
            if with_plague {
                crate::auras::apply_aura(
                    run.app.world_mut(),
                    &crate::auras::AuraApplication {
                        caster: actor,
                        target,
                        spell_id: 55078,
                        effect_mask: u32::MAX,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            let expiry = |run: &super::super::Run| {
                run.app
                    .world()
                    .get::<crate::auras::ActiveAuras>(target)
                    .and_then(|auras| {
                        auras
                            .iter()
                            .find(|aura| aura.spell_id == 55078 && aura.caster == actor)
                    })
                    .and_then(|aura| aura.expires_at)
            };
            let before = expiry(&run);
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                433895,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..4 {
                run.app.update();
            }
            let hit = run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| {
                    event.source == Some(actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(434144)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                });
            assert_eq!(hit, with_plague, "Infliction requires owned Blood Plague");
            if let Some(before) = before {
                assert!((expiry(&run).unwrap() - before - 3.0).abs() < 0.001);
            }
        }
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_vampiric_strike_hits_primary_target_once_and_heals() {
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        let world = run.app.world_mut();
        let mut health = world.get_mut::<shared::components::Health>(actor).unwrap();
        health.current = health.max * 0.5;
        crate::spell_cast::creature_cast(world, actor, 433895, Some(target), false).unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let hits = events
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(433895)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        assert_eq!(
            hits, 1,
            "Blood uses one damage effect on the primary target"
        );
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(actor.to_bits())
                    && event.target == Some(actor.to_bits())
                    && event.spell_id == Some(434422)
                    && event.kind == CombatLogKind::Heal
                    && event.amount > 0),
            "Vampiric Strike still heals"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_vampiric_strike_earns_heartbreaker_per_target() {
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 3).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            shared::components::PowerType::RunicPower,
            -10000.0,
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 433895, Some(target), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let hits = events
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(433895)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        let gained: i32 = events
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(210738)
                    && event.kind == CombatLogKind::Energize
            })
            .map(|event| event.amount)
            .sum();
        assert!(hits >= 2, "Vampiric Strike must land on multiple enemies");
        assert_eq!(
            gained,
            hits as i32 * 20,
            "Heartbreaker grants two runic power per hit"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_transfusion_increases_real_weapon_damage_per_stack() {
        use shared::components::{Npc, UnitSummonedBy};
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .crit_pct = 0.0;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 433895, Some(target), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        let world = run.app.world_mut();
        let weapon = world
            .query::<(bevy::prelude::Entity, &Npc, &UnitSummonedBy)>()
            .iter(world)
            .find(|(_, npc, owner)| npc.template_id == 27893 && owner.0 == actor.to_bits())
            .unwrap()
            .0;
        let damage = |world: &mut bevy::prelude::World| {
            let _seed = crate::random::seed_scope(70448);
            let before = world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
            crate::spell_cast::cast_triggered_with_overrides(
                world,
                weapon,
                49998,
                Some(target),
                1,
                crate::spell_cast::TriggeredSpellOverrides {
                    effect_mask: Some(1),
                    ..Default::default()
                },
            );
            world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0[before..]
                .iter()
                .find(|event| {
                    event.source == Some(weapon.to_bits())
                        && event.spell_id == Some(49998)
                        && event.kind == CombatLogKind::Damage
                })
                .expect("weapon Death Strike damage")
                .amount
        };
        let boosted = damage(world);
        crate::auras::remove_auras(
            world,
            weapon,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 1265577,
        );
        for _ in 0..4 {
            run.app.update();
        }
        let unboosted = damage(run.app.world_mut());
        assert!(
            (boosted as f32 / unboosted as f32 - 1.1).abs() < 0.01,
            "one stack: {boosted} vs {unboosted}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn rune_weapon_death_strike_copies_before_new_blood_shield() {
        use shared::components::{Health, Npc, UnitSummonedBy};
        use shared::protocol::CombatLogKind;
        fn damage(shield_before: bool) -> i32 {
            let _seed = crate::random::seed_scope(70448);
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
                .unwrap();
            for _ in 0..30 {
                run.app.update();
            }
            let world = run.app.world_mut();
            world
                .get_mut::<crate::stats::CharacterStats>(actor)
                .unwrap()
                .crit_pct = 0.0;
            let weapon = world
                .query::<(
                    bevy::prelude::Entity,
                    &Npc,
                    &UnitSummonedBy,
                    &crate::spell_cast::summon::SummonedCreature,
                )>()
                .iter(world)
                .find(|(_, npc, owner, summon)| {
                    npc.template_id == 27893
                        && owner.0 == actor.to_bits()
                        && summon.spell_id == 49028
                })
                .unwrap()
                .0;
            world.get_mut::<Health>(actor).unwrap().current *= 0.5;
            if shield_before {
                crate::auras::apply_aura(
                    world,
                    &crate::auras::AuraApplication {
                        caster: actor,
                        target: actor,
                        spell_id: 77535,
                        effect_mask: u32::MAX,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            crate::spell_cast::powers::change_power(
                world,
                actor,
                shared::components::PowerType::RunicPower,
                1000.0,
            );
            let before = world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
            let _seed = crate::random::seed_scope(70448);
            crate::spell_cast::creature_cast(world, actor, 49998, Some(target), false).unwrap();
            for _ in 0..4 {
                run.app.update();
            }
            assert!(
                crate::auras::has_aura(run.app.world(), actor, 77535),
                "Death Strike must create Blood Shield after the weapon copy"
            );
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0[before..]
                .iter()
                .find(|event| {
                    event.source == Some(weapon.to_bits())
                        && event.spell_id == Some(49998)
                        && event.kind == CombatLogKind::Damage
                })
                .unwrap()
                .amount
        }
        let no_shield = damage(false);
        let existing_shield = damage(true);
        assert!(
            (existing_shield as f32 / no_shield as f32 - 1.08).abs() < 0.01,
            "only an existing Blood Shield increases weapon damage: {no_shield} -> {existing_shield}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn rune_weapon_death_strike_copies_before_hemostasis_is_consumed() {
        use shared::components::{Npc, UnitSummonedBy};
        use shared::protocol::CombatLogKind;
        fn damage(empowered: bool) -> i32 {
            let _seed = crate::random::seed_scope(70448);
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
                .unwrap();
            for _ in 0..30 {
                run.app.update();
            }
            let world = run.app.world_mut();
            world
                .get_mut::<crate::stats::CharacterStats>(actor)
                .unwrap()
                .crit_pct = 0.0;
            let weapon = world
                .query::<(
                    bevy::prelude::Entity,
                    &Npc,
                    &UnitSummonedBy,
                    &crate::spell_cast::summon::SummonedCreature,
                )>()
                .iter(world)
                .find(|(_, npc, owner, summon)| {
                    npc.template_id == 27893
                        && owner.0 == actor.to_bits()
                        && summon.spell_id == 49028
                })
                .unwrap()
                .0;
            if empowered {
                crate::auras::apply_aura(
                    world,
                    &crate::auras::AuraApplication {
                        caster: actor,
                        target: actor,
                        spell_id: 273947,
                        effect_mask: u32::MAX,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            crate::spell_cast::powers::change_power(
                world,
                actor,
                shared::components::PowerType::RunicPower,
                1000.0,
            );
            let before = world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
            let _seed = crate::random::seed_scope(70448);
            crate::spell_cast::creature_cast(world, actor, 49998, Some(target), false).unwrap();
            for _ in 0..4 {
                run.app.update();
            }
            assert!(!crate::auras::has_aura(run.app.world(), actor, 273947));
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0[before..]
                .iter()
                .find(|event| {
                    event.source == Some(weapon.to_bits())
                        && event.spell_id == Some(49998)
                        && event.kind == CombatLogKind::Damage
                })
                .unwrap()
                .amount
        }
        let normal = damage(false);
        let empowered = damage(true);
        assert!(
            (empowered as f32 / normal as f32 - 1.08).abs() < 0.01,
            "weapon copy must receive one Hemostasis stack: {normal} -> {empowered}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_rune_weapon_damage_uses_owner_blood_spell_modifiers() {
        use shared::components::{Npc, UnitSummonedBy};
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let world = run.app.world_mut();
        world
            .get_mut::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .crit_pct = 0.0;
        let weapon = world
            .query::<(bevy::prelude::Entity, &Npc, &UnitSummonedBy)>()
            .iter(world)
            .find(|(_, npc, owner)| npc.template_id == 27893 && owner.0 == actor.to_bits())
            .unwrap()
            .0;
        let damage = |world: &mut bevy::prelude::World| {
            let _seed = crate::random::seed_scope(70448);
            let before = world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
            crate::spell_cast::cast_triggered_with_overrides(
                world,
                weapon,
                49998,
                Some(target),
                1,
                crate::spell_cast::TriggeredSpellOverrides {
                    effect_mask: Some(1),
                    ..Default::default()
                },
            );
            world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0[before..]
                .iter()
                .find(|event| {
                    event.source == Some(weapon.to_bits())
                        && event.spell_id == Some(49998)
                        && event.kind == CombatLogKind::Damage
                })
                .expect("weapon Death Strike damage")
                .amount
        };
        let boosted = damage(world);
        crate::auras::remove_auras(
            world,
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 137008,
        );
        let unboosted = damage(world);
        assert!(
            boosted as f32 > unboosted as f32 * 2.5,
            "owner Blood modifiers: {boosted} vs {unboosted}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_dancing_rune_weapons_mirror_damage_with_owner_attack_power() {
        use shared::components::{Npc, UnitSummonedBy};
        use shared::protocol::CombatLogKind;
        for (owner_spell, weapon_spell) in [
            (49998, 49998),
            (206930, 228645),
            (433895, 433895),
            (50842, 50842),
        ] {
            let _seed = crate::random::seed_scope(70448);
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
                .unwrap();
            for _ in 0..40 {
                run.app.update();
            }
            let world = run.app.world_mut();
            let weapons: Vec<_> = world
                .query::<(bevy::prelude::Entity, &Npc, &UnitSummonedBy)>()
                .iter(world)
                .filter(|(_, npc, owner)| npc.template_id == 27893 && owner.0 == actor.to_bits())
                .map(|(entity, _, _)| entity)
                .collect();
            assert_eq!(weapons.len(), 2);
            let stats = world.get::<crate::stats::CharacterStats>(actor).unwrap();
            let owner_ap = stats.total_attack_power(
                crate::stats::WeaponAttackType::MainHand,
                world.get::<crate::stats::StatModifiers>(actor),
            );
            for &weapon in &weapons {
                assert_eq!(
                    crate::combat::read_pet_owner_critical_chance(world, weapon),
                    Some(stats.crit_pct),
                    "weapon must inherit owner critical chance"
                );
                let (_, pet_ap) = crate::spell_cast::effects::power_stats(world, weapon, false);
                assert!(
                    (pet_ap - owner_ap / 3.0).abs() < 0.01,
                    "weapon AP {pet_ap}, owner AP {owner_ap}"
                );
            }
            crate::spell_cast::powers::change_power(
                world,
                actor,
                shared::components::PowerType::RunicPower,
                1000.0,
            );
            crate::spell_cast::creature_cast(world, actor, owner_spell, Some(target), false)
                .unwrap();
            for _ in 0..4 {
                run.app.update();
            }
            let events = &run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0;
            for weapon in weapons {
                assert!(
                    events
                        .iter()
                        .any(|event| event.source == Some(weapon.to_bits())
                            && event.target == Some(target.to_bits())
                            && event.spell_id == Some(weapon_spell)
                            && event.kind == CombatLogKind::Damage
                            && event.amount > 0),
                    "missing weapon damage for {owner_spell}"
                );
                if owner_spell == 433895 {
                    assert!(
                        crate::auras::has_aura(run.app.world(), weapon, 1265577),
                        "Vampiric Strike grants each weapon Transfusion"
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_dancing_rune_weapon_spawns_owned_actor() {
        use shared::components::{Npc, UnitSummonedBy};
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        super::super::engage(&mut run);
        assert!(
            run.app
                .world()
                .get::<crate::class_progression::KnownSpells>(actor)
                .unwrap()
                .contains(377637)
        );
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 195181,
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        let world = run.app.world_mut();
        let count = world
            .query::<(&Npc, &UnitSummonedBy)>()
            .iter(world)
            .filter(|(npc, owner)| npc.template_id == 27893 && owner.0 == actor.to_bits())
            .count();
        assert_eq!(count, 2, "Everlasting Bond summons a second owned weapon");
        let dance = world
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1264407)
            .expect("weapons grant Dance of Midnight defense");
        assert_eq!(dance.stacks, 2);
        assert_eq!(
            dance
                .effects
                .iter()
                .find(|effect| effect.index == 3)
                .unwrap()
                .amount,
            -6.0
        );
        let bone = world
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 195181)
            .expect("Insatiable Blade grants Bone Shield");
        assert_eq!(bone.stacks, 5);
        let target = run.targets[0];
        for _ in 0..130 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 195292, Some(target), false)
            .unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        let bone = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 195181)
            .unwrap();
        assert_eq!(
            bone.stacks, 11,
            "owner and both weapon casts grant two stacks each"
        );
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 195181,
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 195182, Some(target), false)
            .unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        let world = run.app.world_mut();
        let weapons = world
            .query::<(&Npc, &UnitSummonedBy)>()
            .iter(world)
            .filter(|(npc, owner)| npc.template_id == 27893 && owner.0 == actor.to_bits())
            .count();
        let bone = world
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 195181)
            .expect("Marrowrend grants Bone Shield");
        assert_eq!(
            usize::from(bone.stacks),
            (3 * (1 + weapons)).min(12),
            "owner and active weapons grant three stacks each"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_bone_shield_stats_do_not_scale_with_stacks() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 195181,
        );
        let application = crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 195181,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        };
        crate::auras::apply_aura(run.app.world_mut(), &application);
        for _ in 0..4 {
            run.app.update();
        }
        let one = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .clone();
        for _ in 0..4 {
            crate::auras::apply_aura(run.app.world_mut(), &application);
        }
        for _ in 0..4 {
            run.app.update();
        }
        let five = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap();
        let auras = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap();
        assert_eq!(
            auras
                .iter()
                .find(|aura| aura.spell_id == 195181)
                .unwrap()
                .stacks,
            5
        );
        assert!((one.haste_pct - five.haste_pct).abs() < 0.01);
        assert!((one.armor - five.armor).abs() < 0.01);
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn flagged_fury_of_the_ages_haste_does_not_scale_with_stacks() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let application = crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 347490,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        };
        crate::auras::apply_aura(run.app.world_mut(), &application);
        for _ in 0..4 {
            run.app.update();
        }
        let one = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        crate::auras::apply_aura(run.app.world_mut(), &application);
        for _ in 0..4 {
            run.app.update();
        }
        let two = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        let auras = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap();
        assert_eq!(
            auras
                .iter()
                .find(|aura| aura.spell_id == 347490)
                .unwrap()
                .stacks,
            2
        );
        assert!((one - two).abs() < 0.01);
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_bone_shield_loses_one_stack_on_landed_swings_after_icd() {
        use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
        use server::test_support::healer_intake::{Hit, HitKind};
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let source = run.targets[0];
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |a| a.spell_id == 195181,
        );
        for _ in 0..3 {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id: 195181,
                    effect_mask: u32::MAX,
                    source: AuraSource::Cast,
                },
            );
        }
        let stacks = |run: &super::super::Run| {
            run.app
                .world()
                .get::<ActiveAuras>(actor)
                .unwrap()
                .iter()
                .find(|a| a.spell_id == 195181)
                .map_or(0, |a| a.stacks)
        };
        let hit = |outcome: &str| Hit {
            tick: 0,
            source_guid: 0,
            pack: 0,
            target_slot: 0,
            school: 1,
            raw: 100.0,
            level_scaling_factor: 1.0,
            kind: HitKind::Melee {
                hand: 0,
                outcome: outcome.into(),
                level_difference: 0,
                critical_multiplier: 1.0,
                weapon_speed: 2.0,
            },
        };
        assert_eq!(stacks(&run), 3);
        crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &hit("Miss")).unwrap();
        run.app.update();
        assert_eq!(stacks(&run), 3, "misses must not consume Bone Shield");
        crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &hit("Hit")).unwrap();
        run.app.update();
        assert_eq!(
            stacks(&run),
            2,
            "a landed swing removes one stack, not the whole aura"
        );
        crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &hit("Hit")).unwrap();
        run.app.update();
        assert_eq!(
            stacks(&run),
            2,
            "the native ICD blocks a second immediate stack loss"
        );
        for _ in 0..52 {
            run.app.update();
        }
        crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &hit("Hit")).unwrap();
        run.app.update();
        assert_eq!(
            stacks(&run),
            1,
            "a later landed swing consumes one more stack"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Unholy inputs"]
    fn unholy_essence_haste_uses_unholy_gift_multiplier() {
        let mut run = super::super::prepare_registered_player(252, 1).unwrap();
        let actor = run.actor;
        for _ in 0..4 {
            run.app.update();
        }
        let before = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        for spell_id in [433925, 434153] {
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
        for _ in 0..4 {
            run.app.update();
        }
        let after = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        let expected = ((1.0 + before / 100.0) * 1.018 - 1.0) * 100.0;
        assert!(
            (after - expected).abs() < 0.05,
            "Unholy Gift scales one Essence stack by 1.8: before={before} after={after}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_boiling_point_proc_consumes_and_echoes_blood_boil() {
        use shared::components::{Npc, UnitSummonedBy};
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 3).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        for _ in 0..30 {
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                206930,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..30 {
                run.app.update();
            }
            if crate::auras::has_aura(run.app.world(), actor, 1265968) {
                break;
            }
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 1265968));
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        let world = run.app.world_mut();
        let weapons: Vec<_> = world
            .query::<(
                bevy::prelude::Entity,
                &Npc,
                &UnitSummonedBy,
                &crate::spell_cast::summon::SummonedCreature,
            )>()
            .iter(world)
            .filter(|(_, npc, owner, summon)| {
                npc.template_id == 27893
                    && owner.0 == actor.to_bits()
                    && matches!(summon.spell_id, 49028 | 1237128)
            })
            .map(|(entity, _, _, _)| entity)
            .collect();
        assert_eq!(weapons.len(), 2);
        let start = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 50842, None, false).unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(!crate::auras::has_aura(run.app.world(), actor, 1265968));
        assert!(crate::auras::has_aura(run.app.world(), actor, 1265982));
        for _ in 0..70 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[start..];
        let hits = events
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(50842)
                    && event.kind == shared::protocol::CombatLogKind::Damage
            })
            .count();
        assert_eq!(hits, 2, "one Blood Boil hit and one delayed echo");
        for weapon in weapons {
            for target in &run.targets {
                let hits = events
                    .iter()
                    .filter(|event| {
                        event.source == Some(weapon.to_bits())
                            && event.target == Some(target.to_bits())
                            && event.spell_id == Some(50842)
                            && event.kind == shared::protocol::CombatLogKind::Damage
                    })
                    .count();
                assert_eq!(
                    hits, 2,
                    "each weapon must mirror both the original Blood Boil and its delayed echo on every target"
                );
            }
        }
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn boiling_point_empowers_existing_echo_when_a_new_cast_forces_it() {
        use shared::components::{Npc, UnitSummonedBy};
        use shared::protocol::CombatLogKind;
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        let world = run.app.world_mut();
        let weapons: Vec<_> = world
            .query::<(bevy::prelude::Entity, &Npc, &UnitSummonedBy)>()
            .iter(world)
            .filter(|(_, npc, owner)| npc.template_id == 27893 && owner.0 == actor.to_bits())
            .map(|(entity, _, _)| entity)
            .collect();
        assert_eq!(weapons.len(), 2);
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            1265968,
            Some(actor),
            1
        ));
        for _ in 0..4 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 50842, None, false).unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 1265982));
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            1265968,
            Some(actor),
            1
        ));
        for _ in 0..4 {
            run.app.update();
        }
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .crit_pct = 0.0;
        let start = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 50842, None, false).unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        let hits: Vec<_> = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[start..]
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(50842)
                    && event.kind == CombatLogKind::Damage
            })
            .collect();
        assert_eq!(hits.len(), 2, "the new cast forces the existing echo");
        assert!(!hits[0].crit && !hits[1].crit);
        let ratio = hits[1].amount as f32 / hits[0].amount as f32;
        assert!(
            (ratio - 1.0).abs() < 0.02,
            "both hits must use the same Boiling Point buff: {hits:?}"
        );
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[start..];
        for weapon in weapons {
            let hits: Vec<_> = events
                .iter()
                .filter(|event| {
                    event.source == Some(weapon.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(50842)
                        && event.kind == CombatLogKind::Damage
                })
                .collect();
            assert_eq!(hits.len(), 2, "both casts are mirrored");
            assert!(!hits[0].crit && !hits[1].crit);
            let ratio = hits[1].amount as f32 / hits[0].amount as f32;
            assert!(
                (ratio - 1.0).abs() < 0.05,
                "both weapon copies inherit Boiling Point: {hits:?}"
            );
        }
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn boiling_point_echo_consumes_intervening_proc_and_queues_another_echo() {
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            1265968,
            Some(actor),
            1
        ));
        for _ in 0..4 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 1265968));
        let start = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 50842, None, false).unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 1265982));
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            1265968,
            Some(actor),
            1
        ));
        for _ in 0..75 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 1265968),
            "first echo consumes the intervening proc"
        );
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 1265982),
            "first echo queues a second echo"
        );
        for _ in 0..75 {
            run.app.update();
        }
        let hits = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[start..]
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(50842)
                    && event.kind == CombatLogKind::Damage
            })
            .count();
        assert_eq!(hits, 3, "foreground Blood Boil and two delayed echoes");
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_plague_ticks_apply_selected_brittle() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 3).unwrap();
        let actor = run.actor;
        super::super::engage(&mut run);
        assert!(crate::auras::has_aura(run.app.world(), actor, 374504));
        let mut seen = [false; 3];
        for frame in 0..1800 {
            if frame % 300 == 0 {
                crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 50842, None, 1);
            }
            run.app.update();
            for (index, &target) in run.targets.iter().enumerate() {
                if let Some(aura) = run
                    .app
                    .world()
                    .get::<crate::auras::ActiveAuras>(target)
                    .and_then(|auras| {
                        auras
                            .iter()
                            .find(|aura| aura.spell_id == 374557 && aura.caster == actor)
                    })
                {
                    assert!(
                        aura.effects
                            .iter()
                            .any(|effect| effect.index == 0 && effect.amount == 6.0)
                    );
                    seen[index] = true;
                }
            }
            if seen.iter().all(|hit| *hit) {
                break;
            }
        }
        assert!(
            seen.into_iter().all(|hit| hit),
            "Blood Plague ticks must apply Brittle to each enemy"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn boiling_point_increases_foreground_blood_boil_damage() {
        use shared::protocol::CombatLogKind;
        fn damage(empowered: bool) -> i32 {
            let _seed = crate::random::seed_scope(70448);
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            run.app
                .world_mut()
                .get_mut::<crate::stats::CharacterStats>(actor)
                .unwrap()
                .crit_pct = 0.0;
            for (unit, spell) in [(target, 55078), (actor, 1265968)] {
                if spell == 1265968 && !empowered {
                    continue;
                }
                crate::auras::apply_aura(
                    run.app.world_mut(),
                    &crate::auras::AuraApplication {
                        caster: actor,
                        target: unit,
                        spell_id: spell,
                        effect_mask: u32::MAX,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            let before = run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
            let _seed = crate::random::seed_scope(70448);
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 50842, None, false)
                .unwrap();
            for _ in 0..4 {
                run.app.update();
            }
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0[before..]
                .iter()
                .find(|event| {
                    event.source == Some(actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(50842)
                        && event.kind == CombatLogKind::Damage
                })
                .unwrap()
                .amount
        }
        let normal = damage(false);
        let empowered = damage(true);
        assert!(
            (empowered as f32 / normal as f32 - 1.5).abs() < 0.02,
            "Boiling Point must multiply Blood Boil: {normal} -> {empowered}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn rune_weapon_plague_ticks_after_dismissal_without_mirroring_new_casts() {
        use shared::components::{Npc, UnitSummonedBy};
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        let weapon = run
            .app
            .world_mut()
            .query::<(bevy::prelude::Entity, &Npc, &UnitSummonedBy)>()
            .iter(run.app.world())
            .find(|(_, npc, owner)| npc.template_id == 27893 && owner.0 == actor.to_bits())
            .unwrap()
            .0;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 50842, None, false).unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(target)
                .unwrap()
                .iter()
                .any(|aura| aura.spell_id == 55078 && aura.caster == weapon)
        );
        let before = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        crate::spell_cast::summon::despawn_summon(run.app.world_mut(), weapon);
        for _ in 0..35 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 50842, None, false).unwrap();
        for _ in 0..100 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[before..];
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(weapon.to_bits())
                    && event.spell_id == Some(55078)
                    && event.kind == CombatLogKind::Damage
                    && event.periodic)
        );
        assert!(
            !events
                .iter()
                .any(|event| event.source == Some(weapon.to_bits())
                    && event.spell_id == Some(50842)
                    && event.kind == CombatLogKind::Damage)
        );
        for _ in 0..600 {
            run.app.update();
        }
        assert!(
            run.app.world().get_entity(weapon).is_err(),
            "retired weapon must be removed after its plague expires"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_death_strike_can_earn_and_consume_vampiric_strike() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        for _ in 0..20 {
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                shared::components::PowerType::RunicPower,
                1000.0,
            );
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                49998,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..30 {
                run.app.update();
            }
            if crate::auras::has_aura(run.app.world(), actor, 433899) {
                break;
            }
        }
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 433899),
            "Death Strike earns the proc"
        );
        let strike = crate::spell_cast::knowledge::resolve_action(run.app.world(), actor, 206930)
            .unwrap()
            .unwrap();
        assert_eq!(strike, 433895);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, strike, Some(target), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(!crate::auras::has_aura(run.app.world(), actor, 433899));
        assert!(crate::auras::has_aura(run.app.world(), actor, 433925));
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_sanlayn_gift_vampiric_strikes_raise_and_expire_mastery() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        for _ in 0..4 {
            run.app.update();
        }
        let before = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .mastery_points;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 55233, Some(actor), false)
            .unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 434153));
        let before_haste = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        let strike = crate::spell_cast::knowledge::resolve_action(run.app.world(), actor, 206930)
            .unwrap()
            .expect("Heart Strike resolves");
        assert_eq!(strike, 433895);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, strike, Some(target), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 433925),
            "Vampiric Strike earns Essence"
        );
        let during = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .mastery_points;
        let during_haste = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        assert!(
            (during_haste - ((1.0 + before_haste / 100.0) * 1.03 - 1.0) * 100.0).abs() < 0.05,
            "one Essence stack grants 3% haste during Blood Gift: before={before_haste}, during={during_haste}"
        );
        assert!(
            during >= before + 0.59,
            "Gift amplifies one Blood Thrill stack: before={before} during={during}"
        );
        for _ in 0..500 {
            run.app.update();
        }
        assert!(!crate::auras::has_aura(run.app.world(), actor, 433925));
        let after = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .mastery_points;
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 433925,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        for _ in 0..3 {
            run.app.update();
        }
        let renewed = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .mastery_points;
        assert!(
            (renewed - after - 0.2).abs() < 0.01,
            "without Gift, one Essence stack adds 0.2 mastery points"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_mid2_death_strikes_build_debt_and_marrowrend_spends_it() {
        use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
        use shared::components::PowerType;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        for spell_id in [1296650, 1296651] {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id,
                    effect_mask: u32::MAX,
                    source: AuraSource::ItemSet(2055),
                },
            );
        }
        for expected in 1..=10 {
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                PowerType::RunicPower,
                1000.0,
            );
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                49998,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..30 {
                run.app.update();
            }
            let auras = run.app.world().get::<ActiveAuras>(actor).unwrap();
            let debt = auras
                .iter()
                .find(|aura| aura.spell_id == 1310372)
                .expect("Death Strike builds Blood Debt");
            assert_eq!(debt.stacks, expected);
            assert_eq!(debt.effects[0].amount, 0.5);
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 195182, Some(target), false)
            .unwrap();
        for _ in 0..3 {
            run.app.update();
        }
        let auras = run.app.world().get::<ActiveAuras>(actor).unwrap();
        assert!(!auras.iter().any(|aura| aura.spell_id == 1310372));
        assert!(auras.iter().any(|aura| aura.spell_id == 1300369));
        assert!(
            auras
                .iter()
                .any(|aura| aura.spell_id == 195181 && aura.stacks >= 3)
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_death_strike_heals_and_creates_real_mastery_shield() {
        use server::test_support::healer_intake::{Hit, HitKind};
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        for _ in 0..4 {
            run.app.update();
        }
        let max = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .max;
        let hit = Hit {
            tick: 0,
            source_guid: 0,
            pack: 0,
            target_slot: 0,
            school: 1,
            raw: max * 0.7,
            level_scaling_factor: 1.0,
            kind: HitKind::Melee {
                hand: 0,
                outcome: "Hit".into(),
                level_difference: 0,
                critical_multiplier: 1.0,
                weapon_speed: 2.0,
            },
        };
        crate::combat::replay_healer_hit(run.app.world_mut(), target, actor, &hit).unwrap();
        let before = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .current;
        let current_max = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .max;
        assert!(
            before < current_max,
            "incoming hit must hurt: current={before} max={current_max}"
        );
        for _ in 0..3 {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id: 273947,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            shared::components::PowerType::RunicPower,
            1000.0,
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49998, Some(target), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        let after = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .current;
        assert!(after > before, "landed Death Strike must heal the caster");
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 273947),
            "Death Strike must consume Hemostasis after healing"
        );
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 77535),
            "Blood mastery must create a shield"
        );
        let (_, absorbed) = crate::auras::absorb_damage(run.app.world_mut(), actor, None, 1, max);
        assert!(
            absorbed > 0.0 && absorbed <= max * 0.5,
            "actual physical damage must consume capped Blood Shield: {absorbed}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_shield_cap_follows_bloody_reflection() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        assert!(crate::auras::has_aura(run.app.world(), actor, 1279633));
        let max = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .max;
        crate::spell_cast::cast_triggered_with_effect0_amount(
            run.app.world_mut(),
            actor,
            77535,
            Some(actor),
            1,
            max,
        );
        let (_, selected) = crate::auras::absorb_damage(run.app.world_mut(), actor, None, 1, max);
        assert!(
            (selected - max * 0.65).abs() < 2.0,
            "selected Bloody Reflection raises the actual cap: {selected}/{max}"
        );
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |a| a.spell_id == 1279633,
        );
        crate::spell_cast::cast_triggered_with_effect0_amount(
            run.app.world_mut(),
            actor,
            77535,
            Some(actor),
            1,
            max,
        );
        let (_, unselected) = crate::auras::absorb_damage(run.app.world_mut(), actor, None, 1, max);
        assert!(
            (unselected - max * 0.50).abs() < 2.0,
            "without Bloody Reflection the native cap is 50%: {unselected}/{max}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_death_and_decay_pulses_only_inside_its_real_field() {
        use shared::components::Position;
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 43265, Some(actor), false)
            .unwrap();
        let count = |run: &super::super::Run| {
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(52212)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                })
                .count()
        };
        for _ in 0..60 {
            run.app.update();
        }
        let inside = count(&run);
        assert!(inside >= 2, "expected periodic ground damage, got {inside}");
        let original = *run.app.world().get::<Position>(target).unwrap();
        run.app.world_mut().get_mut::<Position>(target).unwrap().x += 50.0;
        for _ in 0..40 {
            run.app.update();
        }
        assert_eq!(
            count(&run),
            inside,
            "outside target must not take field damage"
        );
        *run.app.world_mut().get_mut::<Position>(target).unwrap() = original;
        for _ in 0..40 {
            run.app.update();
        }
        assert!(
            count(&run) > inside,
            "returning target takes field damage again"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_relish_in_blood_heals_and_grants_power_on_free_decay() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        super::super::engage(&mut run);
        assert!(
            run.app
                .world()
                .get::<crate::class_progression::KnownSpells>(actor)
                .unwrap()
                .contains(317610)
        );
        for id in [195181, 195181, 195181, 81141] {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id: id,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(actor)
            .unwrap();
        health.current = health.max * 0.5;
        let before = health.current;
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            shared::components::PowerType::RunicPower,
            -1000.0,
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 43265, Some(actor), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        let health = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .current;
        let power = crate::spell_cast::powers::power_value(
            run.app.world(),
            actor,
            shared::components::PowerType::RunicPower,
        )
        .unwrap()
        .0;
        assert!(health > before, "Relish in Blood must heal");
        assert_eq!(
            power, 100.0,
            "Relish in Blood grants native ten Runic Power"
        );
        assert!(!crate::auras::has_aura(run.app.world(), actor, 81141));
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_visceral_strength_requires_free_decay() {
        for free in [false, true] {
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            super::super::engage(&mut run);
            assert!(
                run.app
                    .world()
                    .get::<crate::class_progression::KnownSpells>(actor)
                    .unwrap()
                    .contains(434157)
            );
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Cancelled,
                |aura| matches!(aura.spell_id, 81141 | 461130),
            );
            if free {
                crate::auras::apply_aura(
                    run.app.world_mut(),
                    &crate::auras::AuraApplication {
                        caster: actor,
                        target: actor,
                        spell_id: 81141,
                        effect_mask: u32::MAX,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            let before = run
                .app
                .world()
                .get::<crate::stats::CharacterStats>(actor)
                .unwrap()
                .primary
                .strength;
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 43265, Some(actor), false)
                .unwrap();
            for _ in 0..4 {
                run.app.update();
            }
            let world = run.app.world();
            assert_eq!(crate::auras::has_aura(world, actor, 461130), free);
            let after = world
                .get::<crate::stats::CharacterStats>(actor)
                .unwrap()
                .primary
                .strength;
            if free {
                assert!(after > before * 1.05, "strength {before} -> {after}");
            } else {
                assert!(
                    (after - before).abs() < 0.01,
                    "strength {before} -> {after}"
                );
            }
        }
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_soaked_ground_reduces_incoming_strike() {
        use shared::components::Health;
        fn strike(ground: bool) -> f32 {
            let _seed = crate::random::seed_scope(70448);
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let source = run.targets[0];
            if ground {
                crate::auras::apply_aura(
                    run.app.world_mut(),
                    &crate::auras::AuraApplication {
                        caster: actor,
                        target: actor,
                        spell_id: 434034,
                        effect_mask: u32::MAX,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            run.app.update();
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Cancelled,
                |aura| matches!(aura.spell_id, 77535 | 207203),
            );
            let before = run.app.world().get::<Health>(actor).unwrap().current;
            crate::spell_cast::effects::replay_simc_dummy_strike(
                run.app.world_mut(),
                source,
                actor,
                100_000.0,
                false,
                false,
            );
            before - run.app.world().get::<Health>(actor).unwrap().current
        }
        let unprotected = strike(false);
        let protected = strike(true);
        assert!(
            (protected / unprotected - 0.92).abs() < 0.001,
            "strike {unprotected} -> {protected}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_soaked_ground_increases_death_strike_vampiric_proc_chance() {
        use shared::components::{Health, PowerType};
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        let mut found = false;
        for draw_offset in 0..96 {
            let mut outcomes = [false; 2];
            for (index, ground) in [false, true].into_iter().enumerate() {
                crate::auras::remove_auras(
                    run.app.world_mut(),
                    actor,
                    crate::auras::RemoveReason::Cancelled,
                    |aura| matches!(aura.spell_id, 433899 | 434153 | 434034),
                );
                if ground {
                    crate::auras::apply_aura(
                        run.app.world_mut(),
                        &crate::auras::AuraApplication {
                            caster: actor,
                            target: actor,
                            spell_id: 434034,
                            effect_mask: u32::MAX,
                            source: crate::auras::AuraSource::Cast,
                        },
                    );
                }
                let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
                health.current = health.max;
                crate::spell_cast::powers::change_power(
                    run.app.world_mut(),
                    actor,
                    PowerType::RunicPower,
                    1000.0,
                );
                let seed = crate::random::seed_scope(70448);
                for _ in 0..draw_offset {
                    let _: f32 = crate::random::random();
                }
                crate::spell_cast::creature_cast(
                    run.app.world_mut(),
                    actor,
                    49998,
                    Some(target),
                    false,
                )
                .unwrap();
                for _ in 0..4 {
                    run.app.update();
                }
                outcomes[index] = crate::auras::has_aura(run.app.world(), actor, 433899);
                drop(seed);
                for _ in 0..40 {
                    run.app.update();
                }
            }
            if outcomes == [false, true] {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "Blood-soaked Ground must turn a qualifying Death Strike roll into a proc"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_crimson_scourge_builds_real_ground_and_sanguine_follows_position() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        super::super::engage(&mut run);
        for _ in 0..2000 {
            super::super::drive_literal_tick(&mut run).unwrap();
            run.app.update();
            if crate::auras::has_aura(run.app.world(), actor, 391459) {
                break;
            }
        }
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 188290),
            "real Death and Decay must include its caster"
        );
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 391459),
            "selected Sanguine Ground must follow actual ground"
        );
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 434034),
            "selected Blood-soaked Ground must follow actual ground"
        );
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 81141),
            "Death and Decay consumes its earned Crimson Scourge"
        );
        let original = *run
            .app
            .world()
            .get::<shared::components::Position>(actor)
            .unwrap();
        run.app
            .world_mut()
            .get_mut::<shared::components::Position>(actor)
            .unwrap()
            .x += 50.0;
        run.app.update();
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 391459),
            "leaving the ground removes the healing buff"
        );
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 434034),
            "leaving the ground removes Blood-soaked Ground"
        );
        *run.app
            .world_mut()
            .get_mut::<shared::components::Position>(actor)
            .unwrap() = original;
        run.app.update();
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 391459),
            "returning to the active field restores the buff"
        );
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 434034),
            "returning restores Blood-soaked Ground"
        );
        let expiry = run
            .app
            .world_mut()
            .query::<&crate::spell_cast::area_trigger::SpellAreaTrigger>()
            .iter(run.app.world())
            .find(|field| field.spell_id == 43265 && field.caster == actor)
            .unwrap()
            .expires_at
            .unwrap();
        while crate::spell_cast::now(run.app.world()) < expiry + 3.0 {
            run.app.update();
        }
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 391459),
            "Sanguine Ground persists during the four-second expiration grace"
        );
        while crate::spell_cast::now(run.app.world()) < expiry + 4.2 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 391459),
            "Sanguine Ground expires after its grace"
        );
    }

    #[test]
    #[ignore = "requires canonical world database, MID2 Blood inputs and SimC tank trace"]
    fn blood_replays_mid2_tank_dummy_with_literal_rotation() {
        let trace =
            std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
        let report = super::super::super::tank_replay::replay_simc(
            250,
            std::path::Path::new(&trace),
            300,
            70448,
        )
        .unwrap();
        assert_eq!(report["spec"], 250);
        assert_eq!(report["level"], 90);
        assert!(report["incoming_events"].as_u64().unwrap() > 0, "{report}");
        assert!(report["raw_damage"].as_f64().unwrap() > 0.0, "{report}");
        assert!(
            report["active_mitigation_uptime"].as_f64().unwrap() > 0.0,
            "{report}"
        );
        assert!(
            report["tank_casts"]["49998"].as_u64().unwrap() > 0,
            "{report}"
        );
        assert!(
            report["tank_casts"]["195292"].as_u64().unwrap() > 0,
            "{report}"
        );
        println!("BLOOD_TANK_REPORT={report}");
    }
}
