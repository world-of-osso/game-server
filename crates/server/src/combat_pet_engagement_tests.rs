//! Shared attack/hit ownership semantics with native133 and ordinary combat execution.
use crate::combat_refs::{active_opponents, end_all_combat};
use crate::spell_cast::test_harness::{Harness, harness, spawn_npc};
use bevy::prelude::*;
use shared::components::{CombatStatus, ThreatTable, UnitIsPet, UnitSummonedBy};

fn fixture() -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[133], 8).unwrap();
    let owner = spawn_npc(&mut h.app, 2.0);
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(ThreatTable::default());
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert((UnitSummonedBy(owner.to_bits()), UnitIsPet(true)));
    end_all_combat(h.app.world_mut(), h.caster);
    (h, owner)
}

fn assert_owner_engaged(h: &Harness, owner: Entity) {
    let world = h.app.world();
    assert!(active_opponents(world, h.npc).any(|enemy| enemy == h.caster));
    assert!(
        active_opponents(world, owner).any(|enemy| enemy == h.caster),
        "TC AtTargetAttacked engages the attacked controlled unit's owner too"
    );
    assert_eq!(world.get::<CombatStatus>(owner), Some(&CombatStatus(true)));
    let threat = world.get::<ThreatTable>(owner).unwrap();
    assert!(
        threat
            .entries()
            .iter()
            .any(|entry| entry.entity == h.caster.to_bits()),
        "threat-capable owner receives native zero-amount engagement"
    );
    assert_eq!(
        threat.threat_for(h.caster.to_bits()),
        0.0,
        "pet damage threat must not transfer to its owner"
    );
}

#[test]
#[ignore = "RED: melee attack on controlled unit omits owner engagement"]
fn pet_combat_owner_native_melee_hit_engages_owner_without_damage_threat() {
    let _seed = crate::random::seed_scope(1);
    let (mut h, owner) = fixture();
    h.attack(h.npc);
    for _ in 0..20 {
        h.advance(1);
        if h.inbox.log.iter().any(|event| {
            event.source == Some(h.caster.to_bits())
                && event.target == Some(h.npc.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage
        }) {
            assert_owner_engaged(&h, owner);
            return;
        }
    }
    panic!("ordinary melee must actually resolve before owner assertion");
}

#[test]
#[ignore = "RED: direct damage on controlled unit omits owner engagement"]
fn pet_combat_owner_direct_damage_engages_owner_without_damage_threat() {
    let (mut h, owner) = fixture();
    crate::combat::deal_direct_damage(h.app.world_mut(), h.caster, h.npc, 1.0);
    assert_owner_engaged(&h, owner);
}

#[test]
#[ignore = "RED: lethal direct hit omits owner engagement before pet death"]
fn pet_combat_owner_lethal_hit_engages_owner_before_pet_combat_ends() {
    let (mut h, owner) = fixture();
    let damage = h
        .app
        .world()
        .get::<shared::components::Health>(h.npc)
        .unwrap()
        .current;
    crate::combat::deal_direct_damage(h.app.world_mut(), h.caster, h.npc, damage);
    assert_eq!(h.health(h.npc), 0.0, "ordinary hit actually kills the pet");
    assert!(
        active_opponents(h.app.world(), owner).any(|enemy| enemy == h.caster),
        "owner engagement survives the attacked pet's native death cleanup"
    );
    assert_eq!(
        h.app
            .world()
            .get::<ThreatTable>(owner)
            .unwrap()
            .threat_for(h.caster.to_bits()),
        0.0
    );
}

#[test]
#[ignore = "RED: native missile impact on controlled unit omits owner engagement"]
fn pet_combat_owner_native_missile_engages_owner_only_at_impact() {
    let _seed = crate::random::seed_scope(1);
    let (mut h, owner) = fixture();
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.npc)
        .unwrap()
        .x = 30.0;
    h.cast_and_deliver(133, Some(h.npc));
    for _ in 0..100 {
        h.advance(1);
        if h.app
            .world()
            .get::<crate::combat_refs::CombatRefs>(h.npc)
            .is_some()
        {
            break;
        }
    }
    assert!(
        h.app
            .world()
            .get::<crate::combat_refs::CombatRefs>(h.npc)
            .is_some(),
        "real native Fireball launch reference"
    );
    assert!(
        !active_opponents(h.app.world(), owner).any(|enemy| enemy == h.caster),
        "suppressed projectile launch does not engage the target's owner"
    );
    for _ in 0..100 {
        h.advance(1);
        if h.inbox.log.iter().any(|event| {
            event.source == Some(h.caster.to_bits())
                && event.target == Some(h.npc.to_bits())
                && event.spell_id == Some(133)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
        }) {
            assert_owner_engaged(&h, owner);
            return;
        }
    }
    panic!("native Fireball must actually land before owner assertion");
}
