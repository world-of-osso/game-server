//! Gift's authored Blood and other-spec modifier variants are alternatives.
use super::{AuraApplication, AuraSource, SpellModOp, apply_aura, spell_mods};
use crate::spell_cast::test_harness::harness;
use crate::spell_info::SpellStore;
use shared::components::ActiveSpec;

#[test]
fn gift_modifies_essence_once_with_the_selected_spec_value() {
    for (spec, expected) in [(250, [30.0, 15.0, 30.0]), (252, [18.0, 9.0, 18.0])] {
        let mut h = harness(Vec::new(), &[434153, 433925], 6).unwrap();
        let caster = h.caster;
        h.app
            .world_mut()
            .entity_mut(caster)
            .insert(ActiveSpec(spec));
        apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster,
                target: caster,
                spell_id: 434153,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
        let world = h.app.world();
        let essence = world.resource::<SpellStore>().get(433925).unwrap();
        let actual = [
            spell_mods::modify(world, caster, essence, SpellModOp::PointsIndex0, 10.0),
            spell_mods::modify(world, caster, essence, SpellModOp::PointsIndex1, 5.0),
            spell_mods::modify(world, caster, essence, SpellModOp::PointsIndex2, 10.0),
        ];
        for (value, expected) in actual.into_iter().zip(expected) {
            assert!(
                (value - expected).abs() < 0.001,
                "spec {spec}: {value} != {expected}"
            );
        }
    }
}
