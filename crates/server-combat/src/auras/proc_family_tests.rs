//! Explicit native proc families constrain dispatch, including an empty aura mask.
use super::procs::{self, AuraEvent, PROC_DEAL_MELEE_ABILITY};
use crate::spell_cast::test_harness::harness;
use shared::{components::PowerType, protocol::CombatLogKind};

fn festering_wound_grants(spell: u32) -> usize {
    let mut h = harness(vec![], &[197147, spell], 6).unwrap();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 197147,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.set_power(PowerType::RunicPower, 0.0);
    procs::push_event(
        h.app.world_mut(),
        AuraEvent {
            actor: h.caster,
            target: Some(h.npc),
            actor_mask: [PROC_DEAL_MELEE_ABILITY, 0],
            target_mask: [0; 2],
            spell_id: Some(spell),
            can_proc: true,
            cast_used_proc_modifier: false,
            periodic: false,
            damage: 1.0,
            heal: None,
            reflected: false,
        },
    );
    h.advance(3);
    h.inbox
        .log
        .iter()
        .filter(|event| event.spell_id == Some(195757) && event.kind == CombatLogKind::Energize)
        .count()
}

#[test]
fn explicit_proc_family_rejects_frost_strike_without_festering_mask() {
    assert_eq!(festering_wound_grants(49143), 0);
}

#[test]
fn explicit_proc_family_rejects_another_melee_spell_family() {
    assert_eq!(festering_wound_grants(1464), 0);
}

#[test]
fn explicit_proc_family_accepts_native_festering_strike_third_mask_word() {
    assert_eq!(festering_wound_grants(85948), 1);
}
