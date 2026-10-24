//! TC a352b1fa Spell.cpp8538: misses other than EVADE still engage the caster.
use super::test_harness::{Harness, harness};
use crate::spell_info::SpellStore;
use shared::components::CombatStatus;
use shared::protocol::{CombatLogKind, MissKind};

const JUDGMENT: u32 = 20271;
const FIREBALL: u32 = 133;
const DARK_LUCK: u32 = 194356;
const DIVINE_SHIELD: u32 = 642;

fn native_fixture(spell: u32, class: u8) -> Harness {
    let mut h = harness(Vec::new(), &[spell, DARK_LUCK, DIVINE_SHIELD], class).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![spell]));
    let mode = crate::test_data::world_data().templates[&6].aggro_mode;
    let target = h.npc;
    crate::creature_aggro::insert_aggro_components(
        &mut h.app.world_mut().commands().entity(target),
        mode,
    );
    h.app.world_mut().flush();
    h
}

fn complete_native_cast(h: &mut Harness, id: u32) {
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(id)
        .unwrap()
        .clone();
    let caster = h.caster;
    let target = h.npc;
    super::execute_cast(h.app.world_mut(), caster, &spell, Some(target)).unwrap();
    h.advance(3);
    assert!(
        h.inbox
            .go
            .iter()
            .any(|cast| cast.caster == caster.to_bits() && cast.spell_id == id)
    );
}

fn assert_miss(h: &Harness, spell: u32, kind: MissKind) {
    assert!(
        h.inbox.log.iter().any(|event| event.spell_id == Some(spell)
            && event.target == Some(h.npc.to_bits())
            && event.kind == CombatLogKind::Miss(kind)),
        "native miss receipt: {:?}",
        h.inbox.log
    );
    assert_eq!(h.health(h.npc), 5000.0);
}

fn assert_launch_engages_caster(h: &Harness) {
    assert!(
        h.app
            .world()
            .get::<CombatStatus>(h.caster)
            .is_some_and(|status| status.0),
        "a hostile cast need not land to enter combat"
    );
    assert!(crate::combat_refs::has_active_reference_to(
        h.app.world(),
        h.caster,
        h.npc
    ));
}

#[test]
fn native_missed_hostile_spell_creates_launch_combat_reference() {
    let mut h = native_fixture(JUDGMENT, 2);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: DARK_LUCK,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    complete_native_cast(&mut h, JUDGMENT);
    assert_miss(&h, JUDGMENT, MissKind::Miss);
    assert_launch_engages_caster(&h);
}

#[test]
fn native_immune_hostile_spell_creates_launch_combat_reference() {
    let mut h = native_fixture(FIREBALL, 8);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.npc,
            target: h.npc,
            spell_id: DIVINE_SHIELD,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    complete_native_cast(&mut h, FIREBALL);
    assert_miss(&h, FIREBALL, MissKind::Immune);
    assert_launch_engages_caster(&h);
}

#[test]
fn native_evading_hostile_target_has_no_launch_combat_reference() {
    let mut h = native_fixture(JUDGMENT, 2);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::creature_ai::CreatureState::Returning);
    complete_native_cast(&mut h, JUDGMENT);
    assert!(!crate::combat_refs::has_active_reference_to(
        h.app.world(),
        h.caster,
        h.npc
    ));
    assert!(
        !h.app
            .world()
            .get::<CombatStatus>(h.caster)
            .is_some_and(|status| status.0)
    );
}
