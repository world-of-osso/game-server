//! Temporary oil conditions select actual native enchant contributions.
#[test]
#[ignore = "native70448 exact Elemental profile and talent catalogs"]
fn shared_oil_elemental_condition_uses_native_selected_talent() {
    let mut run = super::prepare_literal(262, 1).unwrap();
    let actor = run.actor;
    let selected = run
        .app
        .world()
        .get::<crate::class_progression::TraitLoadout>(actor)
        .unwrap()
        .active_talent_spells()
        .any(|spell| spell.spell_id == 318038);
    for (condition, expected) in [
        ("!talent.flametongue_weapon", !selected),
        ("talent.flametongue_weapon", selected),
    ] {
        run.app
            .world_mut()
            .get_mut::<crate::stats::EquippedItemModifiers>(actor)
            .unwrap()
            .temporary_enchants
            .clear();
        run.app.update();
        let before = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .ratings;
        let token = format!("main_hand:thalassian_phoenix_oil_2,if={condition}");
        run.profile
            .consumables
            .insert("temporary_enchant".into(), token.clone());
        super::profile_effects::install(run.app.world_mut(), actor, &run.profile).unwrap();
        run.app.update();
        let after = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .ratings;
        assert_eq!(
            after.crit > before.crit,
            expected,
            "{token}: real crit contribution"
        );
        assert_eq!(
            after.haste > before.haste,
            expected,
            "{token}: real haste contribution"
        );
        let gaps = super::report::gaps(&run.profile, true);
        assert!(
            !gaps
                .iter()
                .any(|gap| gap == &format!("CONSUMABLE_UNMODELED:temporary_enchant={token}")),
            "a valid false condition is modeled but inactive"
        );
    }
}
