//! Native118, authenticated follower profiles and authored31935/176017 selectors.
use bevy::prelude::*;
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

use super::profile_behavior_tests::spawn_profile_fixture;
use super::update_ai;
use crate::auras::{ActiveAuras, RemoveReason};
use crate::networking::CombatTarget;
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use crate::spell_info::SpellStore;

const POLYMORPH: u32 = 118;
const GARRICK: u32 = 209057;
const MEREDY: u32 = 209059;
const SHUJA: u32 = 209053;
const AVENGERS_SHIELD: u32 = 31935;
const SHADOW_BOLT_VOLLEY: u32 = 176017;
const ARCANE_EXPLOSION: u32 = 1449;

fn polymorph(h: &mut Harness, target: Entity) {
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[6, 16372]);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, POLYMORPH, Some(target), true)
        .unwrap();
    h.advance(1);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == POLYMORPH)
        .unwrap();
    assert!(aura.harmful);
    assert_eq!(aura.interrupt_flags[0], 0x0808_0000);
}

fn remove_polymorph(h: &mut Harness, target: Entity) {
    crate::auras::remove_auras(h.app.world_mut(), target, RemoveReason::Expired, |aura| {
        aura.spell_id == POLYMORPH
    });
}

fn assert_holds_cc(entry: u32) {
    let (mut h, unit, _) = spawn_profile_fixture(entry);
    let enemy = h.npc;
    polymorph(&mut h, enemy);
    let log_start = h.inbox.log.len();
    for _ in 0..80 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    let started: Vec<_> = h.inbox.log[log_start..]
        .iter()
        .filter(|event| {
            event.source == Some(unit.to_bits()) && event.kind == CombatLogKind::CastStart
        })
        .collect();
    assert!(
        started.is_empty(),
        "profile{entry} started casts into native118: {started:?}"
    );
    assert!(crate::auras::has_aura(h.app.world(), enemy, POLYMORPH));
    assert_eq!(
        h.app
            .world()
            .get::<CombatTarget>(unit)
            .and_then(|target| target.0),
        None
    );
    remove_polymorph(&mut h, enemy);
    update_ai(h.app.world_mut());
    h.advance(80);
    assert!(
        h.inbox.log.iter().any(|event| {
            event.source == Some(unit.to_bits()) && event.kind == CombatLogKind::CastSuccess
        }),
        "profile{entry} did not resume after CC expiry"
    );
}

#[test]
fn follower_cc_meredy_holds_then_resumes_native_profile() {
    assert_holds_cc(MEREDY);
}
#[test]
fn follower_cc_shuja_holds_then_resumes_native_profile() {
    assert_holds_cc(SHUJA);
}
#[test]
fn follower_cc_garrick_holds_then_resumes_native_profile() {
    assert_holds_cc(GARRICK);
}

#[test]
fn follower_cc_stops_started_fireball_but_not_an_already_launched_missile() {
    let (mut h, unit, _) = spawn_profile_fixture(MEREDY);
    let enemy = h.npc;
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 133, Some(enemy), false).unwrap();
    assert!(h.app.world().get::<CastState>(unit).is_some());
    polymorph(&mut h, enemy);
    update_ai(h.app.world_mut());
    assert!(
        h.app.world().get::<CastState>(unit).is_none(),
        "unfinished cast into sheep continued"
    );
    remove_polymorph(&mut h, enemy);
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 133, Some(enemy), true).unwrap();
    polymorph(&mut h, enemy);
    update_ai(h.app.world_mut());
    h.advance(40);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(unit.to_bits())
                && event.target == Some(enemy.to_bits())
                && event.kind == CombatLogKind::Damage),
        "CC protection must not cancel an already launched missile"
    );
}

#[test]
fn follower_cc_retargets_living_attacker_not_owner_selected_sheep() {
    let (mut h, unit, _) = spawn_profile_fixture(MEREDY);
    let sheep = h.npc;
    let attacker = spawn_npc(&mut h.app, 4.0);
    h.app
        .world_mut()
        .entity_mut(attacker)
        .insert(CombatTarget(Some(h.caster)));
    polymorph(&mut h, sheep);
    update_ai(h.app.world_mut());
    assert_eq!(
        h.app.world().get::<CombatTarget>(unit).unwrap().0,
        Some(attacker)
    );
}

#[test]
fn follower_cc_hold_keeps_combat_reference_and_stops_melee() {
    let (mut h, unit, _) = spawn_profile_fixture(GARRICK);
    let enemy = h.npc;
    crate::combat_refs::set_in_combat_with(h.app.world_mut(), unit, enemy);
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(crate::combat::AutoAttack::new(enemy));
    polymorph(&mut h, enemy);
    update_ai(h.app.world_mut());
    assert!(
        h.app
            .world()
            .get::<crate::combat::AutoAttack>(unit)
            .is_none()
    );
    assert!(
        h.app
            .world()
            .get::<shared::components::CombatStatus>(unit)
            .unwrap()
            .0
    );
    assert!(
        h.app
            .world()
            .get::<crate::creature_ai::FollowOwner>(unit)
            .is_none()
    );
}

fn selected(h: &mut Harness, spell_id: u32, explicit: Option<Entity>) -> Vec<Entity> {
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(spell_id)
        .unwrap()
        .clone();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.effect == 2)
        .unwrap();
    crate::spell_cast::targets::select_targets(
        h.app.world_mut(),
        h.caster,
        explicit,
        &spell,
        effect,
        0,
        false,
    )
    .unwrap()
}

#[test]
fn follower_cc_authored_shield_excludes_sheep_secondary_but_allows_explicit_sheep() {
    let (mut h, unit, _) = spawn_profile_fixture(GARRICK);
    let primary = h.npc;
    let sheep = spawn_npc(&mut h.app, 4.0);
    let other = spawn_npc(&mut h.app, 5.0);
    polymorph(&mut h, sheep);
    let targets = selected(&mut h, AVENGERS_SHIELD, Some(primary));
    assert!(
        !targets.contains(&sheep),
        "native31935 chained onto protected sheep: {targets:?}"
    );
    assert!(targets.contains(&primary));
    assert!(
        targets.contains(&other),
        "chain should still reach an unprotected secondary"
    );
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        unit,
        AVENGERS_SHIELD,
        Some(primary),
        true,
    )
    .unwrap();
    h.advance(40);
    let damage: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| event.source == Some(unit.to_bits()) && event.kind == CombatLogKind::Damage)
        .collect();
    assert!(
        damage
            .iter()
            .any(|event| event.target == Some(primary.to_bits()))
    );
    assert!(
        damage
            .iter()
            .any(|event| event.target == Some(other.to_bits()))
    );
    assert!(
        !damage
            .iter()
            .any(|event| event.target == Some(sheep.to_bits())),
        "{damage:?}"
    );
    let explicit = selected(&mut h, AVENGERS_SHIELD, Some(sheep));
    assert!(
        explicit.contains(&sheep),
        "explicit player target must remain legal"
    );
    remove_polymorph(&mut h, sheep);
    assert!(selected(&mut h, AVENGERS_SHIELD, Some(primary)).contains(&sheep));
}

#[test]
fn follower_cc_authored_area_excludes_sheep_but_unflagged_area_keeps_it() {
    let (mut h, _, _) = spawn_profile_fixture(GARRICK);
    let sheep = h.npc;
    let other = spawn_npc(&mut h.app, 4.0);
    polymorph(&mut h, sheep);
    let targets = selected(&mut h, SHADOW_BOLT_VOLLEY, None);
    assert!(
        !targets.contains(&sheep),
        "native176017 area selected protected sheep: {targets:?}"
    );
    assert!(targets.contains(&other));
    assert!(
        selected(&mut h, ARCANE_EXPLOSION, None).contains(&sheep),
        "native1449 has no Attr6 exclusion"
    );
    remove_polymorph(&mut h, sheep);
    assert!(selected(&mut h, SHADOW_BOLT_VOLLEY, None).contains(&sheep));
}

#[test]
fn follower_cc_explicit_player_shield_still_damages_sheep() {
    let (mut h, _, _) = spawn_profile_fixture(GARRICK);
    let sheep = h.npc;
    polymorph(&mut h, sheep);
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        h.caster,
        AVENGERS_SHIELD,
        Some(sheep),
        true,
    )
    .unwrap();
    h.advance(40);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(h.caster.to_bits())
                && event.target == Some(sheep.to_bits())
                && event.kind == CombatLogKind::Damage),
        "explicit player damage was blocked: {:?}",
        h.inbox.log
    );
}
