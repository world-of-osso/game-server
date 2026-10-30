//! Brewmaster fixture adapter over the production APL.
use bevy::prelude::*;
pub(super) use server_follower_apl::brewmaster_rotation::*;
// The Brewmaster tests below read these through `super::`.
use super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Encounter, NativeFacts};

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter, &run.profile)?;
    super::literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    while rotation.step(run.app.world_mut(), run.actor, &run.targets, true)? {
        super::preparation::finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("brewmaster_rotation");

impl super::SpecDriver for Rotation {
    fn tank_active_mitigation_auras(&self) -> Option<&'static [u32]> {
        Some(&[119582, 322507, 1241059, 215479, 124255])
    }
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_BREWMASTER_76f8118"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.step(world, actor, targets, false).map(|_| ())
    }

    fn report(&self) -> serde_json::Value {
        self.report()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires canonical Brewmaster profile and world database"]
    fn harmony_coalescence_spends_real_keg_damage_from_pool() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_literal(268, 1).unwrap();
        super::super::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        assert!(
            run.app
                .world()
                .get::<crate::class_progression::TraitLoadout>(actor)
                .unwrap()
                .active_talent_spells()
                .any(|talent| talent.spell_id == 450529)
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
            .unwrap();
        let event = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.spell_id == Some(100780)
                    && event.kind == shared::protocol::CombatLogKind::Damage
            })
            .expect("actual Tiger Palm damage");
        let actual_damage = (event.amount - event.overflow) as f32;
        let deposited = run
            .app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .unwrap()
            .accumulator;
        let world = run.app.world();
        let value = |spell, effect| {
            crate::spell_cast::scripts::script_effect_value(world, actor, spell, effect)
        };
        // SimC reads Way's native effect directly; the selected MID2 row is 30%.
        let expected =
            actual_damage * value(450508, 0) / 100.0 * (1.0 + value(450529, 2) / 100.0) * 1.3;
        assert!(
            (deposited - expected).abs() < 1.0,
            "pool={deposited}, hit={actual_damage}, expected={expected}"
        );
        for _ in 0..40 {
            run.app.update();
        }
        let before = run
            .app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .unwrap()
            .accumulator;
        assert!(before > 0.0);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
            .unwrap();
        for _ in 0..5 {
            run.app.update();
        }
        let after = run
            .app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .unwrap()
            .accumulator;
        assert!(
            after < before,
            "Coalescence spends Keg Smash damage: {before} -> {after}"
        );
    }

    #[test]
    #[ignore = "requires canonical Brewmaster profile and world database"]
    fn harmony_real_damage_banks_and_celestial_infusion_spends() {
        let mut run = super::super::prepare_literal(268, 1).unwrap();
        super::super::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
            .unwrap();
        let bank = run
            .app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .expect("damage banks Harmony")
            .accumulator;
        assert!(bank > 0.0, "real Tiger Palm must bank damage");
        for _ in 0..40 {
            run.app.update();
        }
        {
            let mut health = run
                .app
                .world_mut()
                .get_mut::<shared::components::Health>(actor)
                .unwrap();
            health.current = health.max * 0.5;
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 322101, Some(actor), false)
            .unwrap();
        let healed_bank = run
            .app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .unwrap()
            .accumulator;
        assert!(
            healed_bank > bank,
            "effective Expel Harm must add to Harmony"
        );
        let mut bindings = super::bindings();
        let equipment = super::Equipment::load(run.app.world(), actor, &mut bindings).unwrap();
        let world = run.app.world();
        let facts = super::BrewmasterFacts {
            world,
            actor,
            native: super::NativeFacts::new(
                world,
                actor,
                &bindings,
                super::Encounter::Fixture {
                    started_at: 0.0,
                    duration: 300.0,
                    style: crate::simc_apl_native::FightStyle::Patchwerk,
                },
                1,
            ),
            equipment: &equipment,
        };
        assert_eq!(
            server::simc_apl::Facts::value(&facts, "buff.aspect_of_harmony_accumulator.value")
                .unwrap(),
            f64::from(healed_bank)
        );
        assert_eq!(
            server::simc_apl::Facts::value(&facts, "health.max").unwrap(),
            f64::from(world.get::<shared::components::Health>(actor).unwrap().max)
        );
        for _ in 0..40 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241059, Some(actor), false)
            .unwrap();
        let pools = run
            .app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .unwrap();
        assert_eq!(pools.accumulator, 0.0);
        let before = pools.spender;
        assert!(before > 0.0);
        assert!(crate::auras::has_aura(run.app.world(), actor, 450711));
        for _ in 0..40 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
            .unwrap();
        let after = run
            .app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .unwrap()
            .spender;
        assert!(after < before, "real damage spends the Harmony pool");
    }

    #[test]
    #[ignore = "requires canonical Brewmaster profile and world database"]
    fn brew_keg_smash_grants_and_extends_shuffle() {
        let mut run = super::super::prepare_literal(268, 1).unwrap();
        super::super::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        assert_eq!(
            crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), actor, 100784)
                .unwrap(),
            205523
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
            .unwrap();
        let now = crate::spell_cast::now(run.app.world());
        let remaining = |world: &bevy::prelude::World| {
            world
                .get::<crate::auras::ActiveAuras>(actor)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 215479 && aura.caster == actor)
                .and_then(|aura| aura.expires_at)
                .unwrap()
                - now
        };
        let first = remaining(run.app.world());
        assert!(first > 4.9 && first <= 5.0, "first Shuffle: {first}");
        for _ in 0..40 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 205523, Some(target), false)
            .unwrap();
        let second = remaining(run.app.world());
        assert!(
            second > first + 2.9,
            "Blackout Kick extends Shuffle: {first} -> {second}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and SimC tank trace"]
    fn replay_counts_stagger_self_damage_as_damage_taken() {
        let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("SimC trace required");
        let report = super::super::super::tank_replay::replay_simc_with_healing(
            268,
            std::path::Path::new(&trace),
            300,
            70448,
            true,
        )
        .expect("Brewmaster replay");
        let self_damage = report["self_periodic_damage_taken"].as_f64().unwrap();
        let taken = report["damage_taken"].as_f64().unwrap();
        assert!(
            self_damage > 0.0,
            "the real Stagger ticks must be reported: {report}"
        );
        assert!(
            taken > self_damage,
            "damage taken includes dummy hits and self ticks: {report}"
        );
        assert!(report["largest_spike"].as_f64().unwrap() > 0.0);
    }

    #[test]
    #[ignore = "requires canonical world database and SimC tank trace"]
    fn replays_brewmaster_dummy_pressure() {
        let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("SimC trace required");
        let report = super::super::super::tank_replay::replay_simc_with_healing(
            268,
            std::path::Path::new(&trace),
            300,
            70448,
            true,
        )
        .expect("Brewmaster replay");
        assert_eq!(report["spec"], 268);
        assert_eq!(report["rotation_status"], "registered_literal_simc");
        assert!(report["raw_damage"].as_f64().unwrap() > 0.0);
        assert!(report["active_mitigation_uptime"].as_f64().unwrap() > 0.0);
        assert_eq!(
            report["tank_casts"]["132578"], 3,
            "the literal APL must cast Niuzao on its valid native target"
        );
        println!("HEALER_TANK_REPORT={report}");
    }
}
