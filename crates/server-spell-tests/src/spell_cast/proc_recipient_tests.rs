//! TC AuraEffect.cpp6126–6155: actor-side trigger target is nullable actionTarget.
//! Native273685/273702 cover absent enemy target;270233/270232 keep implicit self.
use super::test_harness::{Harness, harness};
use crate::auras::has_aura;
use shared::protocol::CombatLogKind;

const FIREBALL: u32 = 133;

fn proc_fixture(aura: u32, child: u32) -> Harness {
    let mut h = harness(vec![], &[FIREBALL, aura, child], 8).unwrap();
    let caster = h.caster;
    super::cast_triggered(h.app.world_mut(), caster, aura, Some(caster), 0);
    h.advance(3);
    assert!(has_aura(h.app.world(), caster, aura));
    h
}

fn cast_fireball(h: &mut Harness) {
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    h.advance(100);
    assert!(h.inbox.go.iter().any(|cast| cast.spell_id == FIREBALL));
}

#[test]
#[ignore = "native default proc nullable target must never manufacture an explicit caster recipient"]
fn native_proc_recipient_absent_cast_action_target_does_not_apply_enemy_child_to_self() {
    let _seed = crate::random::seed_scope(70448);
    // Native273685 has Aura42/Trigger273702,101%, mask[0,4], no family mask.
    //273702's damage and aura effects explicitly require enemy6; its third
    //effect's implicit caster child remains independent of that missing unit.
    let mut h = proc_fixture(273685, 273702);
    cast_fireball(&mut h);
    assert!(
        !has_aura(h.app.world(), h.caster, 273702),
        "nullable cast actionTarget must not become the aura owner's own enemy target"
    );
}

#[test]
#[ignore = "native nullable actionTarget preserves a child whose own selector is caster1"]
fn native_proc_recipient_absent_action_target_keeps_native_implicit_self_buff() {
    let _seed = crate::random::seed_scope(70448);
    // Native270233 Aura42/Trigger270232,101%, mask[0,4], no family mask.
    let mut h = proc_fixture(270233, 270232);
    cast_fireball(&mut h);
    assert!(has_aura(h.app.world(), h.caster, 270232));
    assert!(!has_aura(h.app.world(), h.npc, 270232));
}

#[test]
#[ignore = "native ordinary hit proc must retain its actual enemy recipient"]
fn native_proc_recipient_actual_hit_target_receives_native_child_damage() {
    let _seed = crate::random::seed_scope(70448);
    // Native44480 Seal of Wrath Aura42/Trigger44481,100%, harmful spell mask.
    let mut h = proc_fixture(44480, 44481);
    cast_fireball(&mut h);
    let damage = h
        .inbox
        .log
        .iter()
        .filter(|event| event.spell_id == Some(44481) && event.kind == CombatLogKind::Damage)
        .collect::<Vec<_>>();
    assert!(
        damage
            .iter()
            .any(|event| event.target == Some(h.npc.to_bits()) && event.amount > 0)
    );
    assert!(
        !damage
            .iter()
            .any(|event| event.target == Some(h.caster.to_bits()))
    );
}
