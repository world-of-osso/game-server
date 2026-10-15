//! Authored charge formulas and shared resource snapshot boundaries.
use super::test_harness::*;
use shared::components::{Mana, PowerType};
use shared::protocol::CombatLogKind;

#[test]
fn spellmech_barrage_charge_damage_and_talented_mana_refund_survive_flight() {
    for talent in [false, true] {
        let mut h = harness(Vec::new(), &[44425, 36032, 321526], 8).unwrap();
        if talent {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: h.caster,
                    target: h.caster,
                    spell_id: 321526,
                    effect_mask: 1,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        } else {
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
fn spellmech_missing_required_combo_points_refuses_before_any_cost() {
    let mut h = harness(Vec::new(), &[32645], 4).unwrap();
    h.set_power(PowerType::ComboPoints, 0.0);
    h.set_power(PowerType::Energy, 100.0);
    let before = h.health(h.npc);
    let result = super::execute_cast(
        h.app.world_mut(),
        h.caster,
        world_db().unwrap().store.get(32645).unwrap(),
        Some(h.npc),
    );
    assert!(matches!(
        result,
        Err(super::CastFailure::NotEnoughPower(PowerType::ComboPoints))
    ));
    assert_eq!(h.power(PowerType::Energy), 100.0);
    assert_eq!(h.health(h.npc), before);
}

#[test]
fn spellmech_rip_native_duration_and_optional_cost_use_same_snapshot() {
    for points in [1.0, 5.0] {
        let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[1079], 11).unwrap();
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

#[test]
fn spellmech_black_powder_scales_each_native_area_hit_by_spent_combo() {
    for points in [1.0, 3.0, 5.0] {
        let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[319175], 4).unwrap();
        let near = spawn_npc(&mut h.app, 4.0);
        let far = spawn_npc(&mut h.app, 30.0);
        let ally = spawn_player(&mut h.app, 4, 2.0);
        h.set_power(PowerType::ComboPoints, points);
        h.set_power(PowerType::Energy, 100.0);
        let spell = world_db().unwrap().store.get(319175).unwrap();
        let base = super::effects::scaled_amount(h.app.world(), h.caster, spell, &spell.effects[0]);
        let primary_before = h.health(h.npc);
        let near_before = h.health(near);
        let far_before = h.health(far);
        let ally_before = h.health(ally);
        super::execute_cast(h.app.world_mut(), h.caster, spell, None).unwrap();
        assert_eq!(h.power(PowerType::ComboPoints), 0.0);
        assert!(varied(base * points).contains(&(primary_before - h.health(h.npc))));
        assert!(varied(base * points).contains(&(near_before - h.health(near))));
        assert_eq!(h.health(far), far_before);
        assert_eq!(h.health(ally), ally_before);
    }
}
