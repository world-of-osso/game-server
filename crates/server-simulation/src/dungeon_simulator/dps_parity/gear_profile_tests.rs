//! Full native reference setup: profile configuration must not disappear at the follower seam.
#[test]
#[ignore = "requires prepared Ret profile and private native database"]
fn ret_gear_profile_native_race_applies() {
    let run = super::prepare_literal(70, 1).unwrap();
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    assert!(
        known.contains(59224),
        "Dwarf Might of the Mountain must come from native racial spell knowledge"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 59224),
        "native racial knowledge must apply its real crit-damage aura"
    );
}

#[test]
#[ignore = "requires prepared Ret profile and private native database"]
fn ret_gear_profile_native_consumables_apply() {
    let run = super::prepare_literal(70, 1).unwrap();
    for id in [1235108, 1264426, 1294727] {
        let aura = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == id)
            .unwrap_or_else(|| panic!("configured native aura{id} missing"));
        assert!(
            aura.effects.iter().any(|e| e.amount > 0.0),
            "consumable{id} must grant real native stats"
        );
    }
}

#[test]
#[ignore = "requires prepared Ret profile and private native database"]
fn ret_gear_profile_native_oil_coexists_with_permanent_enchant() {
    let mut run = super::prepare_literal(70, 1).unwrap();
    let before = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .ratings;
    run.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItemModifiers>(run.actor)
        .unwrap()
        .temporary_enchants
        .clear();
    run.app.update();
    let after = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .ratings;
    assert!(
        before.crit > after.crit && before.haste > after.haste,
        "oil must contribute native crit and haste independently of permanent enchant"
    );
}

#[test]
#[ignore = "requires prepared Ret profile and private native database"]
fn ret_gear_profile_native_potion_selects_extrema_and_is_consumed_once() {
    use super::literal_rotation::{
        equipment::Equipment,
        execution::{self, NativeAction, Prepared},
    };
    let mut run = super::prepare_literal(70, 1).unwrap();
    let mut bindings = crate::simc_apl_native::Bindings::default();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let action = server::simc_apl::Action {
        name: "potion".into(),
        target: None,
        options: Default::default(),
    };
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        std::iter::empty::<NativeAction>(),
        false,
    )
    .unwrap();
    let potion = prepared
        .execution(&action, &equipment)
        .unwrap()
        .expect("configured potion must be admitted through native item readiness");
    execution::execute(run.app.world_mut(), run.actor, potion).unwrap();
    super::preparation::finish_pending_setup(&mut run).unwrap();
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 1236994)
        .expect("native potion buff");
    assert_eq!(
        aura.effects
            .iter()
            .filter(|e| e.aura_type == 189 && e.amount > 0.0)
            .count(),
        1,
        "only highest secondary gains rating"
    );
    assert_eq!(
        aura.effects
            .iter()
            .filter(|e| e.aura_type == 189 && e.amount < 0.0)
            .count(),
        1,
        "only lowest secondary loses rating"
    );
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        std::iter::empty::<NativeAction>(),
        false,
    )
    .unwrap();
    assert!(
        prepared.execution(&action, &equipment).unwrap().is_none(),
        "one potion per encounter"
    );
}

#[test]
#[ignore = "requires prepared Ret profile and private native database"]
fn ret_gear_profile_native_expansion_traits_apply() {
    let run = super::prepare_literal(70, 1).unwrap();
    for id in [1279599, 1287555, 1279610, 1279614] {
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, id),
            "native Omnium trait{id}"
        );
    }
}
