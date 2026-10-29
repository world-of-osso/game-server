//! Purifying Brew recharge follows native haste and charge-category data.
use super::*;
use crate::auras::{AuraApplication, AuraSource};

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn purifying_recharge_uses_haste() {
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 2825,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    for _ in 0..10 {
        run.app.update();
    }
    let world = run.app.world();
    let stats = world.get::<crate::stats::CharacterStats>(actor).unwrap();
    let info = world
        .resource::<crate::spell_info::SpellStore>()
        .get(119582)
        .unwrap();
    let cat = info.categories.as_ref().unwrap();
    assert_eq!(cat.max_charges, 2);
    let category = cat.charge_category;
    let expected =
        f64::from(cat.charge_recovery_ms) / 1000.0 / (1.0 + f64::from(stats.haste_pct) / 100.0);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 119582, Some(target), false)
        .unwrap();
    let history = run
        .app
        .world()
        .get::<crate::spell_cast::SpellHistory>(actor)
        .unwrap();
    let (used, next) = crate::spell_cast::test_harness::charge_recovery(history, category);
    assert_eq!(used, 1);
    let elapsed = next.unwrap() - crate::spell_cast::now(run.app.world());
    assert!(
        (elapsed - expected).abs() < 0.01,
        "actual recharge {elapsed}, expected {expected}"
    );
}
