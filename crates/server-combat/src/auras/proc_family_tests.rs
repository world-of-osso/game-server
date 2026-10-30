//! Explicit native proc families constrain dispatch, including an empty aura mask.
use crate::auras::procs::{self, AuraEvent, PROC_DEAL_MELEE_ABILITY};
use crate::spell_cast::test_harness::{ObservedCombatEvents, harness};
use shared::{components::PowerType, protocol::CombatLogKind};

fn festering_wound_grants(spell: u32, hit_mask: u32) -> usize {
    let mut h = harness(vec![], &[197147, 195757, spell], 6).unwrap();
    // Finish ordinary KnownSpell passive installation before pinning event candidates.
    // A manual Cast aura would be replaced before the deferred event is processed.
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 197147));
    h.set_power(PowerType::RunicPower, 0.0);
    h.app.init_resource::<ObservedCombatEvents>();
    procs::push_event(
        h.app.world_mut(),
        AuraEvent {
            actor: h.caster,
            target: Some(h.npc),
            action_target: None,
            actor_mask: [PROC_DEAL_MELEE_ABILITY, 0],
            target_mask: [0; 2],
            spell_id: Some(spell),
            can_proc: true,
            cast_used_proc_modifier: false,
            periodic: false,
            damage: if hit_mask == procs::PROC_HIT_ABSORB {
                0.0
            } else {
                1.0
            },
            heal: None,
            hit_mask,
            reflected: false,
        },
    );
    h.advance(3);
    h.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.spell_id == Some(195757)
                && event.kind == CombatLogKind::Energize
                && event.amount > event.overflow
        })
        .count()
}

#[test]
fn explicit_proc_family_rejects_frost_strike_without_festering_mask() {
    assert_eq!(festering_wound_grants(49143, procs::PROC_HIT_NORMAL), 0);
}

#[test]
fn explicit_proc_family_rejects_another_melee_spell_family() {
    assert_eq!(festering_wound_grants(1464, procs::PROC_HIT_NORMAL), 0);
}

#[test]
fn explicit_proc_family_accepts_native_festering_strike_third_mask_word() {
    assert_eq!(festering_wound_grants(85948, procs::PROC_HIT_NORMAL), 1);
}

#[test]
fn explicit_landed_proc_accepts_crit_and_absorb_but_not_parry() {
    assert_eq!(festering_wound_grants(85948, procs::PROC_HIT_CRITICAL), 1);
    assert_eq!(festering_wound_grants(85948, procs::PROC_HIT_ABSORB), 1);
    assert_eq!(festering_wound_grants(85948, procs::PROC_HIT_PARRY), 0);
}
