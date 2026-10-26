//! Auto-attack starts only from `CMSG_ATTACK_SWING` (TrinityCore
//! `HandleAttackSwingOpcode` → `Unit::Attack`), never from a selection
//! (`HandleSetSelectionOpcode` only sets the selection, MiscHandler.cpp:396-399).

use super::*;
use crate::spell_cast::test_harness::{
    Harness, harness, harness_with_plugins, load_native_creature_world,
};
use shared::protocol::{AttackStart, AttackStopped, CombatLogEvent};

/// Warrior caster out of combat, hostile kobold at 3 yd.
fn attack_harness() -> Option<Harness> {
    let mut h = harness(Vec::new(), &[], 1)?;
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(false));
    Some(h)
}

fn swings(h: &Harness) -> Vec<&CombatLogEvent> {
    h.inbox
        .log
        .iter()
        .filter(|event| event.spell_id.is_none() && event.source == Some(h.caster.to_bits()))
        .collect()
}

fn victim(h: &Harness) -> Option<Entity> {
    h.app
        .world()
        .get::<AutoAttack>(h.caster)
        .map(|auto_attack| auto_attack.victim)
}

fn in_combat(h: &Harness, unit: Entity) -> bool {
    h.app
        .world()
        .get::<CombatStatus>(unit)
        .is_some_and(|status| status.0)
}

fn stopped(h: &Harness, victim: Option<Entity>) -> AttackStopped {
    AttackStopped {
        attacker: h.caster.to_bits(),
        victim: victim.map(Entity::to_bits),
        now_dead: false,
    }
}

#[test]
fn selecting_a_hostile_unit_neither_swings_nor_enters_combat() {
    let Some(mut h) = attack_harness() else {
        return;
    };
    let npc = h.npc;
    h.select(Some(npc));
    // 3 s: a swing would land at once and again after the 2 s unarmed speed.
    h.advance(60);

    let world = h.app.world();
    assert_eq!(world.get::<CombatTarget>(h.caster).unwrap().0, Some(npc));
    assert_eq!(victim(&h), None);
    assert!(swings(&h).is_empty(), "{:?}", swings(&h));
    assert!(h.inbox.attack_starts.is_empty());
    assert_eq!(h.health(npc), 5000.0);
    assert!(!in_combat(&h, h.caster));
    assert!(!in_combat(&h, npc));
}

#[test]
fn attack_swing_starts_auto_attack_announces_it_and_the_first_swing_enters_combat() {
    let Some(mut h) = attack_harness() else {
        return;
    };
    let npc = h.npc;
    h.attack(npc);
    h.advance(3);

    assert_eq!(victim(&h), Some(npc));
    assert_eq!(
        h.inbox.attack_starts,
        vec![AttackStart {
            attacker: h.caster.to_bits(),
            victim: npc.to_bits(),
        }]
    );
    let swings = swings(&h);
    assert_eq!(swings.len(), 1, "{swings:?}");
    assert_eq!(swings[0].target, Some(npc.to_bits()));
    assert!(in_combat(&h, h.caster));
    assert!(in_combat(&h, npc));
    // The selection is untouched (`Player::SetTarget` is a no-op, Player.h:1920).
    assert_eq!(h.app.world().get::<CombatTarget>(h.caster).unwrap().0, None);
}

#[test]
fn immediate_attack_start_restores_native_timers_and_publishes_one_packet() {
    let mut h = attack_harness().expect("native combat fixture for immediate attack start");
    let npc = h.npc;
    h.attack(npc);
    h.advance(3);
    let saved = h.app.world().get::<AutoAttack>(h.caster).unwrap().clone();
    assert!(
        saved.timer > Duration::ZERO,
        "the native first swing must have reset its timer"
    );
    h.stop_attack();
    h.advance(3);
    assert_eq!(victim(&h), None);
    let swings_before = swings(&h).len();
    let caster = h.caster;
    crate::combat::start_attack_now(h.app.world_mut(), caster, saved.clone());
    let restored = h.app.world().get::<AutoAttack>(caster).unwrap();
    assert_eq!(restored.victim, saved.victim);
    assert_eq!(restored.timer, saved.timer);
    assert_eq!(restored.off_hand_timer, saved.off_hand_timer);
    h.advance(3);
    let packet = AttackStart {
        attacker: caster.to_bits(),
        victim: npc.to_bits(),
    };
    assert_eq!(h.inbox.attack_starts, vec![packet.clone(), packet]);
    assert_eq!(h.inbox.attack_stops, vec![stopped(&h, Some(npc))]);
    assert_eq!(
        swings(&h).len(),
        swings_before,
        "restoring a live timer must not grant an immediate swing"
    );
}

#[test]
fn attack_stop_ends_swinging_and_announces_it() {
    let Some(mut h) = attack_harness() else {
        return;
    };
    let npc = h.npc;
    h.attack(npc);
    h.advance(3);
    h.stop_attack();
    h.advance(3);
    let swings_before = swings(&h).len();
    h.advance(60);

    assert_eq!(victim(&h), None);
    assert_eq!(h.inbox.attack_stops, vec![stopped(&h, Some(npc))]);
    assert_eq!(swings(&h).len(), swings_before, "no swing after the stop");
}

/// The client stops its auto-attack when its selection turns friendly or
/// empty; the selection alone leaves the server's victim in place.
#[test]
fn a_new_selection_keeps_the_victim_until_the_client_stops() {
    let Some(mut h) = attack_harness() else {
        return;
    };
    let npc = h.npc;
    h.attack(npc);
    h.advance(3);
    h.select(None);
    h.advance(3);
    assert_eq!(victim(&h), Some(npc));

    h.stop_attack();
    h.advance(3);
    assert_eq!(victim(&h), None);
}

#[test]
fn attack_swing_at_a_new_victim_switches_and_keeps_the_swing_timer() {
    let Some(mut h) = attack_harness() else {
        return;
    };
    let first = h.npc;
    let second = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 2.0);
    h.attack(first);
    h.advance(3);
    h.attack(second);
    h.advance(3);

    assert_eq!(victim(&h), Some(second));
    assert_eq!(h.inbox.attack_starts.len(), 2);
    assert_eq!(h.inbox.attack_starts[1].victim, second.to_bits());
    // The first victim's swing reset the timer; the switch does not swing again.
    let swings = swings(&h);
    assert_eq!(swings.len(), 1, "{swings:?}");
    assert_eq!(swings[0].target, Some(first.to_bits()));
}

/// `HandleAttackSwingOpcode`: `!IsValidAttackTarget` answers `SMSG_ATTACK_STOP`.
fn assert_refused(mut h: Harness) {
    let npc = h.npc;
    h.attack(npc);
    h.advance(60);

    assert_eq!(victim(&h), None);
    assert!(h.inbox.attack_starts.is_empty());
    assert_eq!(h.inbox.attack_stops, vec![stopped(&h, Some(npc))]);
    assert!(swings(&h).is_empty());
    assert!(!in_combat(&h, h.caster));
}

#[test]
fn attack_swing_at_a_friendly_guard_is_refused() {
    let Some(h) = attack_harness() else {
        return;
    };
    // FactionTemplate 11: Stormwind City Guard.
    let npc = h.npc;
    let mut h = h;
    h.app
        .world_mut()
        .entity_mut(npc)
        .insert(UnitFactionTemplate(11));
    assert_refused(h);
}

/// `IsValidAttackTarget` (Object.cpp:2367): no dead victim.
#[test]
fn attack_swing_at_a_corpse_is_refused() {
    let Some(mut h) = attack_harness() else {
        return;
    };
    let npc = h.npc;
    h.app.world_mut().entity_mut(npc).insert(Health {
        current: 0.0,
        max: 5000.0,
    });
    assert_refused(h);
}

/// `IsValidAttackTarget` (Object.cpp:2374): no UNIT_FLAG_UNINTERACTIBLE victim.
#[test]
fn attack_swing_at_an_uninteractible_hostile_is_refused() {
    let Some(mut h) = attack_harness() else {
        return;
    };
    let npc = h.npc;
    h.app
        .world_mut()
        .entity_mut(npc)
        .insert(shared::components::UnitFlags(
            shared::components::UnitFlags::NOT_SELECTABLE,
        ));
    assert_refused(h);
}

#[test]
fn faction_reaction_attack_swing_honors_authored_non_attackable_and_pc_immune_flags() {
    // TC UnitFlags: NON_ATTACKABLE0x2, IMMUNE_TO_PC0x100, NOT_SELECTABLE0x02000000.
    for flag in [0x2, 0x100, shared::components::UnitFlags::NOT_SELECTABLE] {
        let mut h = attack_harness().expect("authentic hostile kobold fixture");
        let npc = h.npc;
        h.app
            .world_mut()
            .entity_mut(npc)
            .insert(shared::components::UnitFlags(flag));
        assert_refused(h);
    }
}

#[test]
fn attack_swing_at_an_unknown_unit_answers_a_stop_without_victim() {
    let Some(mut h) = attack_harness() else {
        return;
    };
    let gone = h.app.world_mut().spawn_empty().id();
    h.app.world_mut().despawn(gone);
    h.attack(gone);
    h.advance(3);

    assert_eq!(victim(&h), None);
    assert_eq!(h.inbox.attack_stops, vec![stopped(&h, None)]);
}

/// The victim's death runs its `CombatStop` → `RemoveAllAttackers`, and each
/// attacker's `AttackStop` announces it.
#[test]
fn the_victims_death_stops_the_attack_and_announces_it() {
    let Some(mut h) = attack_harness() else {
        return;
    };
    let npc = h.npc;
    h.app.world_mut().entity_mut(npc).insert(Health {
        current: 1.0,
        max: 5000.0,
    });
    h.attack(npc);
    // Swings until one lands (misses, dodges and parries do not kill).
    for _ in 0..20 {
        h.advance(40);
        if h.health(npc) <= 0.0 {
            break;
        }
    }

    assert_eq!(h.health(npc), 0.0);
    assert_eq!(victim(&h), None);
    assert_eq!(h.inbox.attack_stops, vec![stopped(&h, Some(npc))]);
}

/// `Spell::finish` (Spell.cpp:4448-4450): Polymorph (118) has
/// SPELL_ATTR0_CANCELS_AUTO_ATTACK_COMBAT, so its cast ends the caster's auto-attack.
#[test]
fn a_cast_that_cancels_auto_attack_stops_it() {
    const POLYMORPH: u32 = 118;
    let Some(mut h) = harness_with_plugins(Vec::new(), &[POLYMORPH], 8, false, |app| {
        app.insert_resource(load_native_creature_world(&[6, 16372]));
    }) else {
        return;
    };
    let npc = h.npc;
    h.attack(npc);
    h.advance(3);
    assert_eq!(victim(&h), Some(npc));
    h.cast_and_deliver(POLYMORPH, Some(npc));
    // The 1.5 s cast.
    h.advance(35);

    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(victim(&h), None);
    assert_eq!(h.inbox.attack_stops, vec![stopped(&h, Some(npc))]);
}
