//! Native catalog shields through the production retaliation/damage boundary.
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::{CombatLogKind, MissKind};

use super::{AuraApplication, AuraSource, AuraStatMods};
use crate::spell_cast::test_harness::{Harness, harness};
use crate::spell_info::SpellStore;

const THORNS: u32 = 8148;
const DIVINE_SHIELD: u32 = 642;
const POWER_WORD_SHIELD: u32 = 17;

fn apply(h: &mut Harness, spell_id: u32, caster: Entity, target: Entity) {
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(spell_id)
        .unwrap();
    let effect_mask = super::aura_effect_mask(spell);
    super::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask,
            source: AuraSource::Cast,
        },
    );
}

#[test]
fn damage_shield_uses_original_caster_bonuses_but_owner_attribution() {
    let mut h = harness(Vec::new(), &[THORNS], 11).unwrap();
    let (caster, owner) = (h.caster, h.npc);
    apply(&mut h, THORNS, caster, owner);
    h.app.world_mut().entity_mut(caster).insert(AuraStatMods {
        damage_done_mult: [2.0; 7],
        ..Default::default()
    });
    h.app.world_mut().entity_mut(owner).insert(AuraStatMods {
        damage_done_mult: [5.0; 7],
        ..Default::default()
    });
    let before = h.app.world().get::<Health>(caster).unwrap().current;
    super::damage_shield::retaliate(h.app.world_mut(), owner, caster);
    assert_eq!(
        h.app.world().get::<Health>(caster).unwrap().current,
        before - 4.0
    );
    h.advance(2);
    assert!(h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::Damage
            && event.spell_id == Some(THORNS)
            && event.source == Some(owner.to_bits())
            && event.target == Some(caster.to_bits())
            && event.amount == 4
            && !event.crit
    }));
}

/// Authored20570 Thorns: three base points plus1.0 spell-power coefficient.
#[test]
fn damage_shield_uses_native_power_coefficient_at_retaliation() {
    const POWERED_THORNS: u32 = 20570;
    let mut h = harness(Vec::new(), &[POWERED_THORNS], 11).unwrap();
    let (caster, owner) = (h.caster, h.npc);
    apply(&mut h, POWERED_THORNS, caster, owner);
    let before = h.app.world().get::<Health>(caster).unwrap().current;
    super::damage_shield::retaliate(h.app.world_mut(), owner, caster);
    assert_eq!(
        h.app.world().get::<Health>(caster).unwrap().current,
        before - 103.0
    );
}

#[test]
fn damage_shield_honors_school_immunity_and_native_absorption() {
    for protection in [DIVINE_SHIELD, POWER_WORD_SHIELD] {
        let mut h = harness(Vec::new(), &[THORNS, protection], 2).unwrap();
        let (attacker, owner) = (h.caster, h.npc);
        apply(&mut h, THORNS, owner, owner);
        apply(&mut h, protection, attacker, attacker);
        let before = h.app.world().get::<Health>(attacker).unwrap().current;
        super::damage_shield::retaliate(h.app.world_mut(), owner, attacker);
        assert_eq!(
            h.app.world().get::<Health>(attacker).unwrap().current,
            before
        );
        h.advance(2);
        let defended = h.inbox.log.iter().any(|event| {
            if event.spell_id != Some(THORNS) {
                return false;
            }
            if protection == DIVINE_SHIELD {
                event.kind == CombatLogKind::Miss(MissKind::Immune)
            } else {
                event.kind == CombatLogKind::Damage && event.amount == 0 && event.absorbed == 2
            }
        });
        assert!(defended, "protection {protection}: {:?}", h.inbox.log);
    }
}

#[test]
fn damage_shield_does_not_refresh_creature_leash_damage_window() {
    let mut h = harness(Vec::new(), &[THORNS], 11).unwrap();
    let (owner, attacker) = (h.caster, h.npc);
    apply(&mut h, THORNS, owner, owner);
    let before = h.app.world().get::<Health>(attacker).unwrap().current;
    super::damage_shield::retaliate(h.app.world_mut(), owner, attacker);
    assert!(h.app.world().get::<Health>(attacker).unwrap().current < before);
    assert!(
        h.app
            .world()
            .get::<crate::creature_aggro::LastDamagedTime>(attacker)
            .is_none()
    );
}
