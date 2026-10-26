//! Shared consumables use the same native item payload for every specialization.
use super::literal_rotation::{
    equipment::Equipment,
    execution::{self, NativeAction, Prepared},
};

fn prepare(kind: &str, token: &str) -> super::Run {
    let (mut profile, traits) = super::read_reference(70).unwrap();
    profile.consumables.insert(kind.into(), token.into());
    let identity = super::inputs::read_actor(&profile).unwrap();
    super::prepare_literal_reference(profile, &traits, 1, identity).unwrap()
}

#[test]
#[ignore = "native70448 profile and consumable catalogs"]
fn shared_consumables_void_touched_augment_rune_grants_native_primary() {
    let mut run = prepare("augmentation", "void_touched_augment_rune");
    let actor = run.actor;
    let before = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .primary;
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1264426)
        .expect("full rune token must apply native Void-Touched");
    let amount = aura
        .effects
        .iter()
        .find(|e| e.aura_type == 29)
        .unwrap()
        .amount;
    assert!(amount > 0.0);
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1264426,
    );
    run.app.update();
    let after = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .primary;
    assert!(
        before.strength > after.strength,
        "rune must raise derived primary stat"
    );
    assert_eq!(before.stamina, after.stamina);
    assert!(
        !run.gaps
            .iter()
            .any(|gap| gap == "CONSUMABLE_UNMODELED:augmentation=void_touched_augment_rune")
    );
}

#[test]
#[ignore = "native70448 profile and consumable catalogs"]
fn shared_consumables_thalassian_resistance_quality2_grants_native_versatility() {
    let mut run = prepare("flask", "flask_of_thalassian_resistance_2");
    let actor = run.actor;
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(1235057)
        .unwrap();
    let item = world
        .resource::<crate::item_templates::ItemTemplates>()
        .get(shared::item_data::ItemDefinitionSource::Retail, 241320)
        .unwrap();
    assert_eq!(item.scaling.item_level, 295, "native quality2 identity");
    let tables = world.resource::<crate::item_stats::ItemScalingTables>();
    let expected = (tables.spell_budgets[&295][2]
        * spell.effects[0].coefficient
        * tables.rating_multiplier[294][2])
        .round();
    let aura = world
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1235057)
        .expect("Thalassian Resistance must apply native flask");
    assert_eq!(aura.effects[0].amount, expected);
    let before = world
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .ratings
        .versatility;
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1235057,
    );
    run.app.update();
    let after = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .ratings
        .versatility;
    assert_eq!(
        before - after,
        expected,
        "native scaled rating must reach derived stats"
    );
    assert!(
        !run.gaps
            .iter()
            .any(|gap| gap == "CONSUMABLE_UNMODELED:flask=flask_of_thalassian_resistance_2")
    );
}

#[test]
#[ignore = "native70448 profile and consumable catalogs"]
fn shared_consumables_lights_potential_quality2_cast_grants_native_primary() {
    let mut run = prepare("potion", "lights_potential_2");
    let actor = run.actor;
    let world = run.app.world();
    let before = world
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .primary;
    let potion = world
        .get::<super::profile_effects::Potion>(actor)
        .expect("Light's Potential must configure its native use item");
    assert_eq!((potion.0.item_id, potion.0.item_level), (241308, 295));
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
    let potion = prepared
        .execution(&row, &equipment)
        .unwrap()
        .expect("Light's Potential must be ready");
    execution::execute(run.app.world_mut(), actor, potion).unwrap();
    super::preparation::finish_pending_setup(&mut run).unwrap();
    let world = run.app.world();
    let aura = world
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1236616)
        .expect("native potion primary buff");
    assert!(
        aura.effects
            .iter()
            .any(|e| e.aura_type == 29 && e.misc_values[0] == -2 && e.amount > 0.0)
    );
    let after = world
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .primary;
    assert!(after.strength > before.strength);
    assert_eq!(after.stamina, before.stamina);
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
        "combat potion cannot be reused"
    );
    assert!(
        !run.gaps
            .iter()
            .any(|gap| gap == "CONSUMABLE_UNMODELED:potion=lights_potential_2")
    );
}
