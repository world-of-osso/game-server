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
fn ret_gear_profile_native_flask_changes_derived_mastery_rating() {
    let mut run = super::prepare_literal(70, 1).unwrap();
    let before = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .ratings
        .mastery;
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1235108,
    );
    run.app.update();
    let after = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .ratings
        .mastery;
    assert!(
        before > after,
        "native Magisters flask must raise derived mastery, not just store aura points: before={before}, after={after}"
    );
}

#[test]
#[ignore = "requires prepared Ret profile and private native database"]
fn ret_gear_profile_native_flask_uses_item_rating_multiplier() {
    let run = super::prepare_literal(70, 1).unwrap();
    let world = run.app.world();
    let item = world
        .resource::<crate::item_templates::ItemTemplates>()
        .get(shared::item_data::ItemDefinitionSource::Retail, 241322)
        .unwrap();
    let level = item.scaling.item_level;
    let tables = world.resource::<crate::item_stats::ItemScalingTables>();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(1235108)
        .unwrap();
    let coefficient = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .coefficient;
    let expected = (tables.spell_budgets[&level][2]
        * coefficient
        * tables.rating_multiplier[usize::from(level) - 1][2])
        .round();
    let aura = world
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1235108)
        .unwrap();
    let actual = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert_eq!(
        actual, expected,
        "native scaling class -7 includes its rating multiplier"
    );
}

#[test]
#[ignore = "requires prepared Ret profile and private native database"]
fn ret_gear_profile_native_primary_consumables_do_not_grant_stamina() {
    let mut run = super::prepare_literal(70, 1).unwrap();
    let before = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .primary;
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| matches!(aura.spell_id, 1264426 | 1294727),
    );
    run.app.update();
    let after = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .primary;
    assert!(
        before.strength > after.strength,
        "primary consumables must still grant real Strength"
    );
    assert_eq!(
        before.stamina, after.stamina,
        "hybrid primary stats do not include Stamina"
    );
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
        source_row: None,
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
#[ignore = "requires exact258 profile/player and private native database"]
fn native_expansion_named_shadow_profile_and_numeric_entries_apply_real_auras() {
    const NAMED: &str = "rune_of_unleashed_fire/rune_of_selfmending/rune_of_lingering/rune_of_critical_power/rune_of_overload";
    const NUMERIC: &str = "136822/136819:1/136817/136815/136814";
    for tokens in [NAMED, NUMERIC] {
        let (mut profile, traits) = super::read_reference(258).unwrap();
        assert_eq!(profile.omnium_talents.as_deref(), Some(NAMED));
        profile.omnium_talents = Some(tokens.into());
        let identity = super::inputs::read_actor(&profile).unwrap();
        let run = super::prepare_literal_reference(profile, &traits, 1, identity).unwrap();
        for spell in [1279599, 1279603, 1287555, 1279609, 1279614] {
            assert!(
                crate::auras::has_aura(run.app.world(), run.actor, spell),
                "{tokens}: native Omnium aura{spell} missing"
            );
        }
    }
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

#[test]
#[ignore = "real269 profile and native consumable catalogs"]
fn ww_gear_profile_blood_knights_grants_native_haste() {
    let mut run = super::prepare_measurement(269, 1).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1235110),
        "configured Blood Knights2 flask must apply its real native aura"
    );
    assert!(
        !run.gaps
            .iter()
            .any(|gap| gap == "CONSUMABLE_UNMODELED:flask=flask_of_the_blood_knights_2")
    );
    let before = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .ratings
        .haste;
    let actor = run.actor;
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1235110 && aura.caster == actor,
    );
    run.app.update();
    let after = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .ratings
        .haste;
    assert!(
        before > after,
        "native flask must grant derived haste: before={before}, after={after}"
    );
}

#[test]
#[ignore = "real269 profile and native consumable catalogs"]
fn ww_gear_profile_harandar_grants_native_primary_stat() {
    let mut run = super::prepare_measurement(269, 1).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1232585),
        "configured Harandar Celebration must apply its real native food aura"
    );
    assert!(
        !run.gaps
            .iter()
            .any(|gap| gap == "CONSUMABLE_UNMODELED:food=harandar_celebration")
    );
    let before = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .primary
        .agility;
    let actor = run.actor;
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1232585 && aura.caster == actor,
    );
    run.app.update();
    let after = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .primary
        .agility;
    assert!(
        before > after,
        "native food must grant derived primary stat: before={before}, after={after}"
    );
}

#[test]
#[ignore = "real profile equipment/traits and native food catalog"]
fn ww_gear_profile_silvermoon_parade_applies_same_native_food() {
    let (mut profile, traits) = super::read_reference(269).unwrap();
    profile
        .consumables
        .insert("food".into(), "silvermoon_parade".into());
    let identity = super::inputs::read_actor(&profile).unwrap();
    let run = super::prepare_literal_reference(profile, &traits, 1, identity).unwrap();
    let food = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1232585 && aura.caster == run.actor)
        .expect("Silvermoon Parade must apply the shared native primary-food buff");
    assert!(
        food.effects.iter().any(|effect| effect.aura_type == 29
            && effect.misc_values[0] == -2
            && effect.amount > 0.0),
        "shared food must contain a real native primary-stat amount"
    );
    assert!(
        !run.gaps
            .iter()
            .any(|gap| gap == "CONSUMABLE_UNMODELED:food=silvermoon_parade")
    );
}
