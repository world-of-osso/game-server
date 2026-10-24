//! Native Feral bindings under qualification; literal controller not wired yet.
mod bindings;
mod facts;
mod geometry;

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
    }

    fn rake_initial_damage(prowl: bool) -> (f64, f64) {
        let mut run = prepare(103, 1).unwrap();
        let actor = run.actor;
        for legacy in [231052, 405834] {
            assert!(!crate::auras::has_aura(run.app.world(), actor, legacy));
        }
        if prowl {
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
            run.app.update();
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
            facts
                .value("dot.rip.refreshable")
                .unwrap_err()
                .contains("NATIVE_DOT_REFRESH_UNAVAILABLE")
        );
    }
}
