//! Native Liquid Luster payload, shared by every configured specialization.
use super::literal_rotation::{
    equipment::ItemUse,
    execution::{self, Execution},
};

fn prepare() -> super::Run {
    let (profile, traits) = super::read_reference(70).unwrap();
    let identity = super::inputs::read_actor(&profile).unwrap();
    super::prepare_literal_reference(profile, &traits, 1, identity).unwrap()
}

fn gleam(run: &super::Run) -> &crate::auras::AuraInstance {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1295147)
        .expect("native Lustrous Gleam must exist")
}

#[test]
#[ignore = "native70448 item and spell catalogs"]
fn shared_liquid_luster_native_use_stacks_rating_and_expires_with_driver() {
    let mut run = prepare();
    let actor = run.actor;
    let before = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .ratings
        .versatility;
    let item = ItemUse::consumable(run.app.world(), 271887).unwrap();
    assert_eq!(item.item_level, 295);
    execution::execute(run.app.world_mut(), actor, Execution::Item(item, actor)).unwrap();
    run.app.update();
    let aura = gleam(&run);
    assert_eq!(
        aura.stacks, 1,
        "first Gleam is immediate, before the six-second tick"
    );
    let amount = aura
        .effects
        .iter()
        .find(|effect| effect.aura_type == 189)
        .unwrap()
        .amount;
    assert!(
        amount > 0.0,
        "driver scaling supplies the otherwise zero-valued payload"
    );
    let expires = aura.expires_at.unwrap();
    let driver = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1295132)
        .unwrap();
    assert_eq!(expires, driver.expires_at.unwrap());
    let tick = driver.effects[0].periodic_started_at + 6.0;
    while crate::spell_cast::now(run.app.world()) < tick + 0.1 {
        run.app.update();
    }
    assert_eq!(gleam(&run).stacks, 2);
    assert_eq!(
        gleam(&run).expires_at.unwrap(),
        expires,
        "new stacks do not extend the potion"
    );
    let after = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .ratings
        .versatility;
    assert_eq!(
        after - before,
        amount * 2.0,
        "real stacks grant derived versatility"
    );
    while crate::spell_cast::now(run.app.world()) < expires - 0.1 {
        run.app.update();
    }
    assert_eq!(
        gleam(&run).stacks,
        5,
        "five native stacks before the final expiration"
    );
    for _ in 0..5 {
        run.app.update();
    }
    assert!(!crate::auras::has_aura(run.app.world(), actor, 1295132));
    assert!(!crate::auras::has_aura(run.app.world(), actor, 1295147));
}

#[test]
#[ignore = "native70448 item and spell catalogs"]
fn shared_liquid_luster_profile_prepot_preserves_native_stacks_and_cooldown() {
    let mut run = prepare();
    let actor = run.actor;
    run.profile
        .consumables
        .insert("potion".into(), "liquid_luster_2".into());
    super::profile_effects::install(run.app.world_mut(), actor, &run.profile).unwrap();
    let item = run
        .app
        .world()
        .get::<super::profile_effects::Potion>(actor)
        .unwrap()
        .0;
    assert_eq!((item.item_id, item.item_level), (271887, 295));
    let row = server::simc_apl::Action {
        source_row: None,
        name: "potion".into(),
        target: None,
        options: [("pre_pot_time".into(), "12".into())].into(),
    };
    execution::execute_row(
        run.app.world_mut(),
        actor,
        &row,
        Execution::Item(item, actor),
    )
    .unwrap();
    run.app.update();
    assert_eq!(
        gleam(&run).stacks,
        3,
        "literal twelve-second prepot pays two native six-second ticks"
    );
    let now = crate::spell_cast::now(run.app.world());
    let remaining = gleam(&run).expires_at.unwrap() - now;
    assert!(
        remaining > 17.8 && remaining <= 18.0,
        "prepot remaining duration: {remaining}"
    );
    let bindings = crate::simc_apl_native::Bindings::default();
    let facts = crate::simc_apl_native::NativeFacts::new(
        run.app.world(),
        actor,
        &bindings,
        crate::simc_apl_native::Encounter::Fixture {
            started_at: now,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        },
        1,
    );
    use server::simc_apl::Facts;
    assert_eq!(facts.value("potion.liquid_luster").unwrap(), 1.0);
    let cooldown = facts.value("cooldown.potion.remains").unwrap();
    assert!(
        cooldown > 287.8 && cooldown <= 288.0,
        "native five-minute cooldown aged twelve seconds: {cooldown}"
    );
}
