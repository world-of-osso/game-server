//! Native Feral bindings under qualification; literal controller not wired yet.
mod bindings;

#[cfg(test)]
mod tests {
    use super::super::*;
    use server::simc_apl::Facts;

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
