//! Critical Contrition does not create Divine Aegis in retail.
use super::test_harness::*;
use shared::components::{ActiveSpec, Health};
use shared::protocol::CombatLogKind;

#[test]
fn critical_contrition_does_not_create_divine_aegis() {
    let mut h = harness(Vec::new(), &[281469, 47515, 47753], 5).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 100.0;
    let ally = spawn_player(&mut h.app, 5, 1.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 1_000.0,
        max: 100_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 47515,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 281469, Some(ally), 1);
    h.advance(5);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(281469)
                && event.target == Some(ally.to_bits())
                && event.crit),
        "the actual Contrition heal must critically land: {:?}",
        h.inbox.log
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), ally, 47753),
        "critical Contrition must not create Divine Aegis"
    );
}
