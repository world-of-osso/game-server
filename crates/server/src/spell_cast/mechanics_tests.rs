//! Shared mechanics exercised with native Mainline rows and actual impacts.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Mana, PowerType};

#[test]
fn spellmech_optional_combo_cost_snapshots_rupture_duration() {
    for points in [1.0, 3.0, 5.0] {
        let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[1943], 4).unwrap();
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
fn spellmech_surge_spends_native_optional_current_mana() {
    for mana in [2500.0, 10000.0] {
        let mut h = harness(Vec::new(), &[365350, 365362], 8).unwrap();
        h.app.world_mut().get_mut::<Mana>(h.caster).unwrap().current = mana;
        let spell = world_db().unwrap().store.get(365350).unwrap();
        super::execute_cast(h.app.world_mut(), h.caster, spell, Some(h.npc)).unwrap();
        assert_eq!(h.app.world().get::<Mana>(h.caster).unwrap().current, 0.0);
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
