//! Fixture-only pinned Feral consumer; missing native capabilities remain explicit.
mod bindings;
mod context;
mod driver;
mod facts;
pub(super) use driver::Rotation;
mod geometry;
mod source;

#[cfg(test)]
mod tests {
    use super::super::*;
    use server::simc_apl::Facts;

    #[test]
    #[ignore = "requires exact spec103 profile/traits and native private DB"]
    fn feral_literal_native_spell_targets_follow_actual_area_recipients() {
        let mut run = prepare(103, 3).unwrap();
        engage(&mut run);
        let bindings = super::bindings::build(run.app.world(), run.actor).unwrap();
        let encounter = crate::simc_apl_native::Encounter::Fixture {
            started_at: crate::spell_cast::now(run.app.world()),
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        };
        {
            let facts = super::facts::FeralFacts::read(
                run.app.world_mut(),
                run.actor,
                &bindings,
                encounter,
                &run.targets,
            )
            .unwrap();
            assert_eq!(facts.value("spell_targets.swipe_cat").unwrap(), 3.0);
            assert_eq!(facts.value("spell_targets.primal_wrath").unwrap(), 3.0);
            assert_eq!(
                facts.value("spell_targets").unwrap(),
                3.0,
                "pinned distance-disabled bare operand is the actual encounter enemy count"
            );
        }
        run.app
            .world_mut()
            .get_mut::<shared::components::Position>(run.targets[2])
            .unwrap()
            .x = 1000.0;
        let facts = super::facts::FeralFacts::read(
            run.app.world_mut(),
            run.actor,
            &bindings,
            encounter,
            &run.targets,
        )
        .unwrap();
        assert_eq!(facts.value("spell_targets.swipe_cat").unwrap(), 2.0);
        assert_eq!(facts.value("spell_targets.primal_wrath").unwrap(), 2.0);
        assert_eq!(
            facts.value("spell_targets").unwrap(),
            3.0,
            "distance-disabled encounter count is distinct from native reachable recipients"
        );
    }

    fn wait_native_gcd(run: &mut Run, actor: Entity, spell_id: u32) {
        for _ in 0..100 {
            let world = run.app.world();
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(spell_id)
                .unwrap();
            if world
                .get::<crate::spell_cast::SpellHistory>(actor)
                .is_none_or(|history| !history.on_gcd(spell, crate::spell_cast::now(world)))
            {
                return;
            }
            run.app.update();
        }
        panic!("native GCD for{spell_id} did not expire in100frames");
    }

    fn rake_initial_damage(prowl: bool) -> (f64, f64) {
        let mut run = prepare(103, 1).unwrap();
        let actor = run.actor;
        let encounter = crate::simc_apl_native::Encounter::Fixture {
            started_at: crate::spell_cast::now(run.app.world()),
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        };
        literal_rotation::capture_fixture(run.app.world_mut(), actor, encounter).unwrap();
        for legacy in [231052, 405834] {
            assert!(!crate::auras::has_aura(run.app.world(), actor, legacy));
        }
        for target in [actor, fixture_player(&run)] {
            if !crate::auras::has_aura(run.app.world(), target, 1126) {
                wait_native_gcd(&mut run, actor, 1126);
                crate::spell_cast::creature_cast(
                    run.app.world_mut(),
                    actor,
                    1126,
                    Some(target),
                    false,
                )
                .expect("ordinary native Mark of the Wild before Prowl");
            }
        }
        wait_native_gcd(&mut run, actor, 1822);
        if prowl {
            crate::auras::observations::start_removal_probe(run.app.world_mut(), actor, 5215);
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5215, Some(actor), false)
                .expect("ordinary native Prowl cast");
            assert!(crate::auras::has_aura(run.app.world(), actor, 5215));
        }
        for _ in 0..100 {
            let world = run.app.world();
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(1822)
                .unwrap();
            let waiting = world
                .get::<crate::spell_cast::SpellHistory>(actor)
                .is_some_and(|history| history.on_gcd(spell, crate::spell_cast::now(world)));
            if !waiting {
                break;
            }
            let observed_before = run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
            let prowl_before = prowl && crate::auras::has_aura(run.app.world(), actor, 5215);
            run.app.update();
            if prowl_before && !crate::auras::has_aura(run.app.world(), actor, 5215) {
                let removal = crate::auras::observations::removal_probe_snapshot(run.app.world());
                let events = &run
                    .app
                    .world()
                    .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                    .0[observed_before..];
                eprintln!(
                    "RAKE_PROWL_REMOVAL {}",
                    serde_json::json!({"removal": removal, "frame_events": events})
                );
            }
        }
        run.app
            .world_mut()
            .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .clear();
        let source = run.app.world().resource::<crate::spell_info::SpellStore>();
        let bonus = source
            .get(1822)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == 3)
            .unwrap()
            .base_points;
        if prowl {
            assert!(
                crate::auras::has_aura(run.app.world(), actor, 5215),
                "native Prowl survives the ordinary GCD wait"
            );
        }
        let actor_position = *run
            .app
            .world()
            .get::<shared::components::Position>(actor)
            .unwrap();
        {
            let mut target_position = run
                .app
                .world_mut()
                .get_mut::<shared::components::Position>(run.targets[0])
                .unwrap();
            target_position.x = actor_position.x + 1.5;
            target_position.y = actor_position.y;
            target_position.z = actor_position.z;
        }
        let _seed = crate::random::seed_scope(70448);
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            actor,
            1822,
            Some(run.targets[0]),
            false,
        )
        .expect("ordinary native Rake cast");
        let combat = report::combat(run.app.world(), &run.targets, 1.0);
        let damage = combat["actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|action| action["spell_id"] == 1822)
            .map(|action| action["damage"].as_f64().unwrap())
            .sum();
        (damage, 1.0 + f64::from(bonus) / 100.0)
    }

    #[test]
    #[ignore = "requires exact spec103 profile/traits and native private DB"]
    fn feral_literal_native_selected_profile_prowl_rake_uses_authored_stealth_bonus() {
        let (normal, multiplier) = rake_initial_damage(false);
        let (stealth, _) = rake_initial_damage(true);
        assert!(normal > 0.0 && stealth > 0.0, "both real casts must land");
        assert!(
            (stealth - normal * multiplier).abs() <= 2.0,
            "native selected-profile Rake: normal{normal}, Prowl{stealth}, authored factor{multiplier}"
        );
    }

    #[test]
    #[ignore = "requires exact spec103 profile/traits and native private DB"]
    fn feral_literal_native_maximum_energy_reads_modified_bite_cost_without_spending() {
        use crate::spell_cast::powers::PowerPools;
        use server::simc_apl::{Action, ActionFacts};
        let mut run = prepare(103, 1).unwrap();
        let actor = run.actor;
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 102543),
            "Incarnation policy remains a separate gap"
        );
        assert!(!crate::auras::has_aura(run.app.world(), actor, 391882));
        let bindings = super::bindings::build(run.app.world(), actor).unwrap();
        let encounter = crate::simc_apl_native::Encounter::Fixture {
            started_at: crate::spell_cast::now(run.app.world()),
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        };
        let action = Action {
            name: "ferocious_bite".into(),
            target: None,
            options: Default::default(),
        };
        // Native70448:22568 costs25 Energy and has a25 extra cap; actual391882
        // effect0 is a -100% Energy-cost modifier. No proc eligibility is asserted.
        for (modifier, expected) in [(None, 50.0), (Some(391882), 25.0)] {
            if let Some(spell_id) = modifier {
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
                assert!(crate::auras::has_aura(run.app.world(), actor, spell_id));
            }
            let before = run.app.world().get::<PowerPools>(actor).unwrap().clone();
            let last_cast = run
                .app
                .world()
                .get::<crate::spell_cast::SpellHistory>(actor)
                .and_then(|history| history.last_successful_cast());
            for _ in 0..3 {
                let facts = super::facts::FeralFacts::read(
                    run.app.world_mut(),
                    actor,
                    &bindings,
                    encounter,
                    &run.targets,
                )
                .unwrap();
                assert_eq!(
                    facts.value_for(&action, "maximum_energy").unwrap(),
                    expected
                );
            }
            assert_eq!(
                run.app.world().get::<PowerPools>(actor).unwrap(),
                &before,
                "reading a threshold must neither fill nor spend any real pool"
            );
            assert_eq!(
                run.app
                    .world()
                    .get::<crate::spell_cast::SpellHistory>(actor)
                    .and_then(|history| history.last_successful_cast()),
                last_cast
            );
        }
    }

    #[test]
    #[ignore = "requires exact spec103 profile/traits and native private DB"]
    fn feral_literal_native_prowl_first_uses_enemy_context_but_casts_on_actor() {
        use super::super::literal_rotation::{Driver, equipment::Equipment, execution::Prepared};
        use server::simc_apl::{Action, ActionFacts};
        let mut run = prepare_literal(103, 3).unwrap();
        let actor = run.actor;
        if !crate::auras::has_aura(run.app.world(), actor, 768) {
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false)
                .expect("ordinary native Cat Form setup");
            preparation::finish_pending_setup(&mut run).unwrap();
        }
        assert!(!crate::auras::has_aura(run.app.world(), actor, 5215));
        // Observe a native owned Rake aura, not a synthetic refreshable number.
        // This fixture qualifies context/dispatch, not Rake's producer or OOC exit.
        apply_native_dot(&mut run, 155722);
        let encounter = crate::simc_apl_native::Encounter::Fixture {
            started_at: crate::spell_cast::now(run.app.world()),
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        };
        let mut bindings = super::bindings::build(run.app.world(), actor).unwrap();
        let equipment = Equipment::load(run.app.world(), actor, &mut bindings).unwrap();
        let potion = super::context::read_configured_potion(&run.profile).unwrap();
        let mut driver = Driver::new(super::source::load().unwrap());
        let before_target = run
            .app
            .world()
            .get::<crate::networking::CombatTarget>(actor)
            .and_then(|target| target.0);
        let selected = {
            let prepared = Prepared::read(
                run.app.world_mut(),
                actor,
                &run.targets,
                &equipment,
                super::bindings::execution_actions(&bindings),
                false,
            )
            .unwrap();
            let facts = super::context::Context::read(
                run.app.world_mut(),
                actor,
                &bindings,
                &equipment,
                &prepared,
                &potion,
                encounter,
                &run.targets,
            )
            .unwrap();
            let prowl = Action {
                name: "prowl".into(),
                target: None,
                options: Default::default(),
            };
            assert_eq!(
                facts.target_ids(&prowl).unwrap(),
                run.targets
                    .iter()
                    .map(|target| target.to_bits())
                    .collect::<Vec<_>>(),
                "self-cast source target_if observes the ordered live enemies, not its cast recipient"
            );
            driver
                .select("aoe_builder", &facts, &prepared, &equipment)
                .unwrap()
                .expect("actual source's first refreshable Prowl context")
        };
        assert_eq!(selected.0.name, "prowl");
        assert_eq!(
            selected.0.target,
            Some(run.targets[1].to_bits()),
            "first target's fresh native Rake must lose FIRST to the second target"
        );
        driver
            .execute(run.app.world_mut(), actor, selected.0, selected.1)
            .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        assert!(crate::auras::has_aura(run.app.world(), actor, 5215));
        for target in &run.targets {
            assert!(!crate::auras::has_aura(run.app.world(), *target, 5215));
        }
        assert_eq!(
            run.app
                .world()
                .get::<crate::networking::CombatTarget>(actor)
                .and_then(|target| target.0),
            before_target,
            "observing an enemy context must not retarget a real self-cast"
        );
    }

    fn apply_native_dot(run: &mut Run, spell_id: u32) {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.targets[0],
                spell_id,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }

    #[test]
    #[ignore = "requires exact spec103 profile/traits and native private DB"]
    fn feral_literal_native_lunar_inspiration_applies_and_removes_override() {
        let mut run = prepare(103, 1).unwrap();
        let world = run.app.world_mut();
        let actor = run.actor;
        assert!(crate::auras::has_aura(world, actor, 155580));
        assert!(
            crate::auras::has_aura(world, actor, 155627),
            "TC Lunar Inspiration bootstrap must apply its native override aura"
        );
        assert_eq!(
            server::spell_cast::knowledge::resolve_action(world, actor, 8921).unwrap(),
            Some(155625),
            "ordinary learned Moonfire resolves through native override while selected"
        );
        let known: Vec<_> = world
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap()
            .iter()
            .collect();
        crate::auras::remove_auras(
            world,
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 155580,
        );
        assert!(!crate::auras::has_aura(world, actor, 155627));
        assert_eq!(
            world
                .get::<crate::class_progression::KnownSpells>(actor)
                .unwrap()
                .iter()
                .collect::<Vec<_>>(),
            known,
            "the script changes an aura, not the learned book"
        );
        assert_eq!(
            server::spell_cast::knowledge::resolve_action(world, actor, 8921).unwrap(),
            Some(8921)
        );
        crate::auras::apply_aura(
            world,
            &crate::auras::AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 155580,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
        assert!(crate::auras::has_aura(world, actor, 155627));
        assert_eq!(
            server::spell_cast::knowledge::resolve_action(world, actor, 8921).unwrap(),
            Some(155625)
        );
    }

    #[test]
    #[ignore = "requires exact spec103 profile/traits and native private DB"]
    fn feral_literal_native_bindings_read_owned_rake_and_moonfire_refresh() {
        let mut run = prepare(103, 1).unwrap();
        let bindings = super::bindings::build(run.app.world(), run.actor).unwrap();
        let encounter = crate::simc_apl_native::Encounter::Fixture {
            started_at: crate::spell_cast::now(run.app.world()),
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        };
        for (alias, spell_id) in [("rake", 155722), ("moonfire_cat", 155625)] {
            let operand = format!("dot.{alias}.refreshable");
            let before = crate::simc_apl_native::NativeFacts::new(
                run.app.world(),
                run.actor,
                &bindings,
                encounter,
                1,
            )
            .with_targets(Some(run.targets[0]), &run.targets);
            assert_eq!(before.value(&operand).unwrap(), 1.0);
            apply_native_dot(&mut run, spell_id);
            let fresh = crate::simc_apl_native::NativeFacts::new(
                run.app.world(),
                run.actor,
                &bindings,
                encounter,
                1,
            )
            .with_targets(Some(run.targets[0]), &run.targets);
            assert_eq!(fresh.value(&operand).unwrap(), 0.0);
            assert_eq!(fresh.value(&format!("active_dot.{alias}")).unwrap(), 1.0);
        }
        apply_native_dot(&mut run, 1079);
        let facts = crate::simc_apl_native::NativeFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            encounter,
            1,
        )
        .with_targets(Some(run.targets[0]), &run.targets);
        assert!(
            facts.value("dot.primal_wrath.remains").unwrap() > 0.0,
            "Primal Wrath observes the actual owned Rip aura, not a synthetic aura285381"
        );
        assert!(
            facts
                .value("dot.primal_wrath.refreshable")
                .unwrap_err()
                .contains("NATIVE_DOT_REFRESH_UNAVAILABLE")
        );
        assert!(
            facts
                .value("dot.rip.refreshable")
                .unwrap_err()
                .contains("NATIVE_DOT_REFRESH_UNAVAILABLE")
        );
    }
}
