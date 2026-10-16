//! Aura241: TC HandleForceMoveForward retains FORCE_MOVEMENT until the last aura leaves.
use super::*;
use shared::components::UnitFlags2;

#[test]
fn toyfx62_force_forward_preserves_other_flags_until_last_source_aura_is_removed() {
    use crate::auras::{AuraApplication, AuraSource, RemoveReason};
    let mut h = harness(Vec::new(), &[148626, 194807], 8).unwrap();
    let unit = h.caster;
    h.app.world_mut().entity_mut(unit).insert(UnitFlags2(1));
    for spell_id in [148626, 194807] {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster: unit,
                target: unit,
                spell_id,
                effect_mask: 1, // Both real source spells carry241 at EffectIndex0.
                source: AuraSource::Cast,
            },
        );
        assert_eq!(
            h.app.world().get::<UnitFlags2>(unit),
            Some(&UnitFlags2(0x41))
        );
    }
    crate::auras::remove_auras(h.app.world_mut(), unit, RemoveReason::Interrupted, |aura| {
        aura.spell_id == 148626
    });
    assert_eq!(
        h.app.world().get::<UnitFlags2>(unit),
        Some(&UnitFlags2(0x41))
    );
    crate::auras::remove_auras(h.app.world_mut(), unit, RemoveReason::Expired, |aura| {
        aura.spell_id == 194807
    });
    assert_eq!(h.app.world().get::<UnitFlags2>(unit), Some(&UnitFlags2(1)));
}
