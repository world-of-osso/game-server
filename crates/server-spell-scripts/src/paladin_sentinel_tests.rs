use super::*;
use combat::auras::{AuraApplication, AuraSource};

#[test]
fn sentinel_removal_cleans_linked_aura_and_decay() {
    let mut harness =
        crate::test_support::harness(vec![], &[SENTINEL, 1241410], 11).expect("native spell data");
    let caster = harness.caster;
    let world = harness.app.world_mut();
    world
        .entity_mut(caster)
        .insert(shared::components::ActiveSpec(66));
    let mut application = AuraApplication {
        caster,
        target: caster,
        spell_id: SENTINEL,
        effect_mask: u32::MAX,
        source: AuraSource::Cast,
    };
    auras::apply_aura(world, &application);
    application.spell_id = 1241410;
    auras::apply_aura(world, &application);
    assert!(auras::has_aura(world, caster, SENTINEL));
    assert!(auras::has_aura(world, caster, 1241410));
    assert!(world.get::<SentinelDecay>(caster).is_some());
    auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == SENTINEL
    });
    assert!(!auras::has_aura(world, caster, 1241410));
    assert!(world.get::<SentinelDecay>(caster).is_none());
}
