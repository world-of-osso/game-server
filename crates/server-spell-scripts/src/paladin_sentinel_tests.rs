use crate::{combat, test_support};
use combat::auras::{self, AuraApplication, AuraSource, RemoveReason};

const SENTINEL: u32 = 389539;
const WRATH_HAMMER_OVERRIDE: u32 = 1241410;

// Sentinel registers its own on_remove; the linked-aura policy's on_remove
// must still run for the same spell.
#[test]
fn sentinel_removal_removes_linked_hammer_override() {
    let mut harness = test_support::harness(vec![], &[SENTINEL, WRATH_HAMMER_OVERRIDE], 11)
        .expect("native spell data");
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
    application.spell_id = WRATH_HAMMER_OVERRIDE;
    auras::apply_aura(world, &application);
    assert!(auras::has_aura(world, caster, SENTINEL));
    assert!(auras::has_aura(world, caster, WRATH_HAMMER_OVERRIDE));
    auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == SENTINEL
    });
    assert!(!auras::has_aura(world, caster, WRATH_HAMMER_OVERRIDE));
}
