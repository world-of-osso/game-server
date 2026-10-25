//! Native aura54 caster penalties must reach physical spell hit selection too.
use super::test_harness::*;
use shared::components::PowerType;
use shared::protocol::{CombatLogKind, MissKind};

fn assert_dark_luck_misses(spell: u32, class: u8) {
    let mut h = harness(Vec::new(), &[194356, spell, SUNWELL_RADIANCE], class).unwrap();
    if class == 1 {
        h.set_power(PowerType::Rage, 1000.0);
    } else {
        charge_range(&mut h);
    }
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 194356,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.cast_and_deliver(spell, Some(h.npc));
    h.advance(50);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox.log.iter().any(|event| event.spell_id == Some(spell)
            && event.kind == CombatLogKind::Miss(MissKind::Miss)),
        "{:?}",
        h.inbox.log
    );
    assert_eq!(h.health(h.npc), 5000.0);
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 194356,
    );
    // Only the post-removal positive control requires a hit. Preserve the miss case.
    apply_native_enemy_dodge_reduction(&mut h);
    h.advance(130); // Native Mortal Strike cooldown has elapsed; no history reset.
    h.cast_and_deliver(spell, Some(h.npc));
    h.advance(50);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.log.iter().any(|event| event.spell_id == Some(spell)
        && event.kind == CombatLogKind::Damage
        && event.amount > 0));
    assert!(h.health(h.npc) < 5000.0);
}

#[test]
fn class_completeness_native_dark_luck_blocks_mortal_strike_hit() {
    assert_dark_luck_misses(12294, 1);
}

#[test]
fn class_completeness_native_dark_luck_blocks_auto_shot_hit() {
    assert_dark_luck_misses(75, 3);
}
