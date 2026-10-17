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
