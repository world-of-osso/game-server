//! Native70448 charm/possess lifecycle at the real aura and combat boundary.
use bevy::prelude::*;
use shared::components::{CombatStatus, MovementControl, ThreatTable, UnitFactionTemplate};

use crate::auras::{
    ActiveAuras, AuraApplication, AuraSource, RemoveReason, apply_aura, has_aura, remove_auras,
};
use crate::combat::AutoAttack;
use crate::networking::CombatTarget;
use crate::spell_cast::test_harness::{Harness, harness, spawn_npc};

fn fixture() -> Harness {
    harness(Vec::new(), &[605, 205364, 1098, 30923], 5).unwrap()
}

fn apply(h: &mut Harness, spell_id: u32) {
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.npc,
            spell_id,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

#[test]
#[ignore = "RED: generic aura2/6 charm state absent"]
fn charm_native_605_takes_faction_stops_combat_and_restores_on_cancel() {
    let mut h = fixture();
    let enemy = spawn_npc(&mut h.app, 4.0);
    let mut threat = ThreatTable::default();
    threat.add_threat(enemy.to_bits(), 90.0);
    h.app.world_mut().entity_mut(h.npc).insert((
        AutoAttack::new(h.caster),
        CombatTarget(Some(h.caster)),
        threat,
        CombatStatus(true),
    ));
    apply(&mut h, 605);
    assert!(has_aura(h.app.world(), h.npc, 605));
    assert_eq!(
        h.app.world().get::<UnitFactionTemplate>(h.npc).unwrap().0,
        1,
        "possessed Kobold must fight for Human charmer, not faction25"
    );
    assert!(h.app.world().get::<AutoAttack>(h.npc).is_none());
    assert_eq!(h.app.world().get::<CombatTarget>(h.npc).unwrap().0, None);
    assert!(
        h.app
            .world()
            .get::<ThreatTable>(h.npc)
            .unwrap()
            .entries()
            .is_empty()
    );
    assert!(
        h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled,
        "possessing priest's body may not consume movement input"
    );
    remove_auras(h.app.world_mut(), h.npc, RemoveReason::Cancelled, |a| {
        a.spell_id == 605
    });
    assert_eq!(
        h.app.world().get::<UnitFactionTemplate>(h.npc).unwrap().0,
        25
    );
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled
    );
}

#[test]
#[ignore = "RED: generic aura6 owner/follow absent"]
fn charm_native_205364_changes_faction_follows_charmer_and_expires() {
    let mut h = fixture();
    apply(&mut h, 205364);
    assert_eq!(
        h.app.world().get::<UnitFactionTemplate>(h.npc).unwrap().0,
        1
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::creature_ai::FollowOwner>(h.npc)
            .unwrap()
            .owner,
        h.caster
    );
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled,
        "Dominate Mind does not park the priest's body"
    );
    let duration = h
        .app
        .world()
        .get::<ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 205364)
        .unwrap()
        .duration_ms;
    assert!(duration > 0, "native duration must be finite");
    h.advance(duration as usize / 50 + 3);
    assert!(!has_aura(h.app.world(), h.npc, 205364));
    assert_eq!(
        h.app.world().get::<UnitFactionTemplate>(h.npc).unwrap().0,
        25
    );
    assert!(
        h.app
            .world()
            .get::<crate::creature_ai::FollowOwner>(h.npc)
            .is_none()
    );
}

#[test]
#[ignore = "RED: generic aura6 controller replacement absent"]
fn charm_native_1098_30923_uses_same_lifecycle_and_rejects_second_charmer() {
    for spell in [1098, 30923] {
        let mut h = fixture();
        apply(&mut h, spell);
        assert_eq!(
            h.app.world().get::<UnitFactionTemplate>(h.npc).unwrap().0,
            1,
            "native aura6 spell{spell}"
        );
        let second = spawn_npc(&mut h.app, 6.0);
        apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster: second,
                target: h.npc,
                spell_id: 205364,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
        assert!(
            !has_aura(h.app.world(), h.npc, 205364),
            "already charmed unit refuses another charmer"
        );
        remove_auras(h.app.world_mut(), h.npc, RemoveReason::Cancelled, |a| {
            a.spell_id == spell
        });
        assert_eq!(
            h.app.world().get::<UnitFactionTemplate>(h.npc).unwrap().0,
            25
        );
    }
}
