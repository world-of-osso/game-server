//! Incoming damage is exercised through the real spell pipeline, not aura fields.
use super::test_harness::*;
use shared::components::Health;

#[test]
fn dummy8_breath_burn_reduces_only_damage_to_the_owning_monk_and_expires() {
    // This coefficient/ownership fixture requires Keg Smash to land; native
    // Sunwell Radiance effect1 isolates that setup dodge, not Fire Blast's roll.
    let mut h = harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[115181, 123725, 121253, 108853],
        10,
    )
    .unwrap();
    let caster = h.caster;
    let npc = h.npc;
    h.app
        .world_mut()
        .entity_mut(npc)
        .insert(crate::stats::CharacterStats {
            spell_power: 100.0,
            ..Default::default()
        });
    let foreign = spawn_player(&mut h.app, 10, 0.0);
    h.cast_and_deliver(121253, Some(npc));
    assert!(
        crate::auras::has_aura(h.app.world(), npc, 121253),
        "Keg Smash must land before Breath of Fire: {:?}",
        h.inbox.log
    );
    h.advance(35);
    h.cast_and_deliver(115181, Some(npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), npc, 123725));
    let before = h.health(caster);
    super::cast_triggered(h.app.world_mut(), npc, 108853, Some(caster), 1);
    h.advance(5);
    assert!(
        varied(187.3875).contains(&(before - h.health(caster))),
        "damage {}, burn {}, combat {:?}",
        before - h.health(caster),
        crate::auras::has_aura(h.app.world(), npc, 123725),
        h.inbox.log
    );
    let before = h.health(foreign);
    super::cast_triggered(h.app.world_mut(), npc, 108853, Some(foreign), 1);
    h.advance(5);
    assert!(varied(197.25).contains(&(before - h.health(foreign))));
    h.advance(320);
    assert!(!crate::auras::has_aura(h.app.world(), npc, 123725));
    h.app.world_mut().get_mut::<Health>(caster).unwrap().current = 1000.0;
    let before = h.health(caster);
    super::cast_triggered(h.app.world_mut(), npc, 108853, Some(caster), 1);
    h.advance(5);
    assert!(varied(197.25).contains(&(before - h.health(caster))));
}
