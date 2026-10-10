//! Mana formulas consume authored talent values, not fixed percentages.
use super::test_harness::*;
use shared::components::{Mana, PowerType};
use shared::protocol::CombatLogKind;

#[test]
fn spellmech_barrage_charge_damage_and_talented_mana_refund_survive_flight() {
    for talent in [false, true] {
        let mut h = harness(Vec::new(), &[44425, 36032, 321526], 8).unwrap();
        if !talent {
            h.app.world_mut().entity_mut(h.caster).insert(
                crate::class_progression::KnownSpells::fixture(vec![44425, 36032]),
            );
            crate::auras::remove_auras(
                h.app.world_mut(),
                h.caster,
                crate::auras::RemoveReason::Cancelled,
                |a| a.spell_id == 321526,
            );
        }
        h.set_power(PowerType::ArcaneCharges, 3.0);
        h.app.world_mut().get_mut::<Mana>(h.caster).unwrap().current = 5000.0;
        let spell = world_db().unwrap().store.get(44425).unwrap();
        let base = super::effects::scaled_amount(h.app.world(), h.caster, spell, &spell.effects[0]);
        super::execute_cast(h.app.world_mut(), h.caster, spell, Some(h.npc)).unwrap();
        assert_eq!(h.power(PowerType::ArcaneCharges), 0.0);
        assert_eq!(
            h.app.world().get::<Mana>(h.caster).unwrap().current,
            if talent { 5600.0 } else { 5000.0 }
        );
        h.advance(25);
        let damage: i32 = h
            .inbox
            .log
            .iter()
            .filter(|e| e.spell_id == Some(44425) && e.kind == CombatLogKind::Damage)
            .map(|e| e.amount)
            .sum();
        assert!(
            varied(base * (1.0 + 3.0 * 0.9)).contains(&(damage as f32)),
            "base={base}, damage={damage}"
        );
    }
}

#[test]
fn spellmech_surge_optional_talent_uses_catalog_percentage() {
    let mut h = harness(Vec::new(), &[365350, 365362, 457521], 8).unwrap();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 457521,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let spell = world_db().unwrap().store.get(365350).unwrap();
    let base = super::effects::scaled_amount(h.app.world(), h.caster, spell, &spell.effects[0]);
    let bonus = world_db().unwrap().store.get(457521).unwrap().effects[0].base_points;
    let before = h.health(h.npc);
    super::execute_cast(h.app.world_mut(), h.caster, spell, Some(h.npc)).unwrap();
    assert!(varied(base * 2.0 * (1.0 + bonus / 100.0)).contains(&(before - h.health(h.npc))));
}

#[test]
fn spellmech_rip_native_duration_and_optional_cost_use_same_snapshot() {
    for points in [1.0, 5.0] {
        let mut h = harness(Vec::new(), &[1079], 11).unwrap();
        h.set_power(PowerType::ComboPoints, points);
        h.set_power(PowerType::Energy, 100.0);
        super::execute_cast(
            h.app.world_mut(),
            h.caster,
            world_db().unwrap().store.get(1079).unwrap(),
            Some(h.npc),
        )
        .unwrap();
        assert_eq!(h.power(PowerType::ComboPoints), 0.0);
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 1079)
            .unwrap();
        assert_eq!(aura.duration_ms, (4000.0 * (1.0 + points)) as u32);
    }
}

#[test]
fn spellmech_cancelled_surge_neither_spends_nor_hits() {
    let mut h = harness(Vec::new(), &[365350, 365362], 8).unwrap();
    let before = h.health(h.npc);
    h.cast_and_deliver(365350, Some(h.npc));
    super::cancel_cast(h.app.world_mut(), h.caster);
    h.advance(70);
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().current,
        10000.0
    );
    assert_eq!(h.health(h.npc), before);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 365362));
}
