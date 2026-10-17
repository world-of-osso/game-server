//! TC a352b1fa boss_telash_greywing.cpp: independent initial Energy boundary.
use super::*;
use crate::spell_cast::powers::{PowerTypeTable, power_value};
use shared::components::PowerType;

#[test]
#[ignore = "RED: Telash native initial Energy60 not implemented"]
fn azure_telash_appears_with_sixty_native_energy() {
    let (mut h, telash, _) = bound_boss_at_difficulty(186737, 2515, 2, 205, None);
    let energy = h
        .app
        .world()
        .resource::<PowerTypeTable>()
        .get(PowerType::Energy)
        .expect("native Energy data prerequisite");
    assert_eq!(energy.max_base_power, 100);
    // No synthetic powers, runes or alternate actor: observe the spawned native boss.
    h.advance(0.1);
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((60.0, 100.0)),
        "source JustAppeared initializes native primary Energy to60"
    );
}

#[test]
#[ignore = "RED: native creature power regeneration flag not yet honored"]
fn azure_telash_native_flag_stops_regen_without_changing_controlled_pools() {
    let (mut h, telash, _) = bound_boss_at_difficulty(186737, 2515, 2, 205, None);
    h.advance(1.0);
    assert_eq!(
        power_value(h.app.world(), telash, PowerType::Energy),
        Some((60.0, 100.0)),
        "authored unit_flags2=0 prevents unauthored baseline regeneration"
    );
    // Control the existing companion resource path, not actual Telash gameplay.
    h.app
        .world_mut()
        .get_mut::<shared::components::UnitFlags>(telash)
        .unwrap()
        .0 |= crate::faction::UNIT_FLAG_PLAYER_CONTROLLED;
    h.advance(0.2);
    let (current, maximum) = power_value(h.app.world(), telash, PowerType::Energy).unwrap();
    assert!(
        current > 60.0,
        "controlled pools retain native player-rate regeneration"
    );
    assert_eq!(
        maximum, 100.0,
        "control does not manufacture a new capacity"
    );
}
