//! Shared mechanics exercised with native Mainline rows and actual impacts.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Health, PowerType};

#[test]
fn spellmech_optional_combo_cost_snapshots_rupture_duration() {
    for points in [1.0, 3.0, 5.0] {
        let mut h = harness(Vec::new(), &[1943], 4).unwrap();
        h.set_power(PowerType::ComboPoints, points);
        h.set_power(PowerType::Energy, 100.0);
        super::cast_granted(h.app.world_mut(), h.caster, 1943, Some(h.npc)).unwrap();
        assert_eq!(h.power(PowerType::ComboPoints), 0.0);
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 1943)
            .unwrap();
        assert_eq!(aura.duration_ms, (4000.0 * (points + 1.0)) as u32);
        h.set_power(PowerType::ComboPoints, 5.0 - points);
        h.advance(20);
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 1943)
            .unwrap();
        assert_eq!(aura.duration_ms, (4000.0 * (points + 1.0)) as u32);
    }
}

#[test]
fn spellmech_surge_spends_current_mana_and_scales_before_post_cast_buff() {
    for mana in [2500.0, 10000.0] {
        let mut h = harness(Vec::new(), &[365350, 365362, 457521], 8).unwrap();
        // No optional Arcing Cleave talent in this base-formula case.
        crate::auras::remove_auras(
            h.app.world_mut(),
            h.caster,
            crate::auras::RemoveReason::Cancelled,
            |a| a.spell_id == 457521,
        );
        h.app.world_mut().entity_mut(h.caster).insert(
            crate::class_progression::KnownSpells::fixture(vec![365350, 365362]),
        );
        h.set_power(PowerType::Mana, mana);
        let spell = world_db().unwrap().store.get(365350).unwrap();
        let base = super::effects::scaled_amount(h.app.world(), h.caster, spell, &spell.effects[0]);
        let before = h.health(h.npc);
        super::execute_cast(h.app.world_mut(), h.caster, spell, Some(h.npc)).unwrap();
        assert_eq!(h.power(PowerType::Mana), 0.0);
        assert!(
            varied(base * 2.0 * mana / 10000.0).contains(&(before - h.health(h.npc))),
            "mana={mana}, base={base}, loss={}",
            before - h.health(h.npc)
        );
    }
}

#[test]
fn spellmech_envenom_spends_all_combo_points_and_scales_damage() {
    for points in [1.0, 3.0, 5.0] {
        let mut h = harness(Vec::new(), &[32645], 4).unwrap();
        h.set_power(PowerType::ComboPoints, points);
        h.set_power(PowerType::Energy, 100.0);
        let spell = world_db().unwrap().store.get(32645).unwrap();
        let base = super::effects::scaled_amount(h.app.world(), h.caster, spell, &spell.effects[0]);
        let before = h.health(h.npc);
        super::execute_cast(h.app.world_mut(), h.caster, spell, Some(h.npc)).unwrap();
        assert_eq!(h.power(PowerType::ComboPoints), 0.0);
        assert!(varied(base * points).contains(&(before - h.health(h.npc))));
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 32645)
            .unwrap();
        assert_eq!(aura.duration_ms, (1000.0 * points) as u32);
    }
}

fn sacrifice(h: &mut Harness, recipient: Entity) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: recipient,
            target: h.npc,
            spell_id: 6940,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

#[test]
fn spellmech_sacrifice_splits_real_incoming_spell_damage() {
    let mut h = harness(Vec::new(), &[133, 6940], 8).unwrap();
    let recipient = spawn_player(&mut h.app, 2, 1.0);
    h.app.world_mut().entity_mut(recipient).insert(Health {
        current: 10000.0,
        max: 10000.0,
    });
    sacrifice(&mut h, recipient);
    let before = h.health(h.npc);
    super::cast_triggered(h.app.world_mut(), h.caster, 133, Some(h.npc), 1);
    h.advance(25);
    let protected_loss = before - h.health(h.npc);
    let recipient_loss = 10000.0 - h.health(recipient);
    assert!(recipient_loss > 0.0);
    assert!((recipient_loss / (recipient_loss + protected_loss) - 0.3).abs() < 0.01);
}

#[test]
fn spellmech_sacrifice_health_floor_removes_link_without_redirecting_hit() {
    let mut h = harness(Vec::new(), &[133, 6940], 8).unwrap();
    let recipient = spawn_player(&mut h.app, 2, 1.0);
    h.app.world_mut().entity_mut(recipient).insert(Health {
        current: 200.0,
        max: 1000.0,
    });
    sacrifice(&mut h, recipient);
    super::cast_triggered(h.app.world_mut(), h.caster, 133, Some(h.npc), 1);
    h.advance(25);
    assert_eq!(h.health(recipient), 200.0);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, 6940));
}
