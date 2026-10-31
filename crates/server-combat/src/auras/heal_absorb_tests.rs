//! Native Shroud of Purgatory absorbs healing before health can increase.
use crate::spell_cast::{
    self,
    effects::EffectCtx,
    test_harness::{harness, world_db},
};
use shared::components::Health;

#[test]
fn native_shroud_consumes_healing_then_allows_the_remainder() {
    let mut h = harness(Vec::new(), &[116888, 431483], 2).unwrap();
    let unit = h.caster;
    h.app.world_mut().entity_mut(unit).insert(Health {
        current: 100.0,
        max: 1000.0,
    });
    spell_cast::cast_triggered_with_effect0_amount(
        h.app.world_mut(),
        unit,
        116888,
        Some(unit),
        1,
        150.0,
    );
    h.advance(1);
    assert!(super::has_aura(h.app.world(), unit, 116888));
    let spell = world_db().unwrap().store.get(431483).unwrap().clone();
    let ctx = EffectCtx {
        caster: unit,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    spell_cast::effects::apply_fixed_heal(h.app.world_mut(), &ctx, unit, 100.0);
    assert_eq!(h.health(unit), 100.0);
    assert!(super::has_aura(h.app.world(), unit, 116888));
    spell_cast::effects::apply_fixed_heal(h.app.world_mut(), &ctx, unit, 100.0);
    assert_eq!(h.health(unit), 150.0);
    assert!(!super::has_aura(h.app.world(), unit, 116888));
}
