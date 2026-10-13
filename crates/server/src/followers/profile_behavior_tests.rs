//! Every authenticated local class/spec profile uses the production initializer and
//! real spell/aura/cost executor. No invented follower identities or spell rows.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness, spawn_npc, world_db};
use shared::components::{Health, Npc, UnitPowers};

pub(super) fn spawn_profile_fixture(entry: u32) -> (Harness, Entity, Profile) {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let profile = catalog::load_catalog(&db)
        .unwrap()
        .profiles
        .into_iter()
        .find(|profile| profile.entry == entry)
        .expect("authenticated imported follower profile");
    let mut h = harness(vec![], &[], 8).unwrap();
    // Full native metadata includes actual form/passive/trigger dependencies.
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&db).unwrap());
    let unit = spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(unit).insert((
        Npc {
            template_id: profile.entry,
            name: profile.name.clone(),
        },
        UnitPowers::default(),
    ));
    spawning::initialize(h.app.world_mut(), h.caster, unit, profile.clone()).unwrap();
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = Some(h.npc);
    (h, unit, profile)
}

fn assert_profile_combat(entry: u32) {
    let (mut h, unit, profile) = spawn_profile_fixture(entry);
    let enemy = h.npc;
    if profile.role == GroupRole::Healer {
        h.app
            .world_mut()
            .get_mut::<Health>(h.caster)
            .unwrap()
            .current = 100.0;
    }
    if profile.role == GroupRole::Tank {
        // Taunts retain native creature-melee miss rolls. This success fixture
        // fixes only the hit modifier; the real miss boundary is tested below.
        h.app
            .world_mut()
            .get_mut::<crate::auras::AuraStatMods>(unit)
            .unwrap()
            .melee_hit_chance_pct = 100.0;
        h.app.world_mut().entity_mut(enemy).insert((
            shared::components::ThreatTable::default(),
            crate::creature_ai::CreatureState::Pursuing {
                target: h.caster.to_bits(),
            },
            crate::networking::CombatTarget(Some(h.caster)),
        ));
        crate::creature_aggro::add_threat(h.app.world_mut(), enemy, h.caster, 100.0);
        update_ai(h.app.world_mut());
        h.advance(4);
        assert_eq!(
            h.app
                .world()
                .get::<crate::networking::CombatTarget>(enemy)
                .unwrap()
                .0,
            Some(unit),
            "{} must taunt the ally's attacker with spell{}; casts {:?}; combat {:?}",
            profile.name,
            profile.taunt_spell,
            h.inbox.go,
            h.inbox.log
        );
    }
    let before = h.health(enemy);
    for _ in 0..200 {
        update_ai(h.app.world_mut());
        h.advance(1);
        if profile.role == GroupRole::Healer && h.health(h.caster) > 100.0 {
            break;
        }
    }
    if profile.role == GroupRole::Healer {
        assert!(
            h.health(h.caster) > 100.0,
            "{} must heal the injured owner through spell{}",
            profile.name,
            profile.heal_spell
        );
        assert!(
            !h.inbox
                .log
                .iter()
                .any(|event| event.source == Some(unit.to_bits())
                    && event.kind == shared::protocol::CombatLogKind::Damage),
            "{} must triage before damage",
            profile.name
        );
    } else {
        assert!(
            h.health(enemy) < before,
            "{} class{} spec{} must damage with its native priority spells{}/{}; actual casts: {:?}",
            profile.name,
            profile.class_id,
            profile.spec_id,
            profile.attack_spell,
            profile.cooldown_spell,
            h.inbox
                .go
                .iter()
                .filter(|event| event.caster == unit.to_bits())
                .map(|event| event.spell_id)
                .collect::<Vec<_>>()
        );
    }
    assert!(
        h.app
            .world()
            .get::<UnitPowers>(unit)
            .is_some_and(|powers| !powers.entries.is_empty()),
        "{} native class resources must be replicated",
        profile.name
    );
}

#[test]
fn follower_real_taunt_miss_keeps_the_attacked_ally_as_victim() {
    let (mut h, unit, _) = spawn_profile_fixture(209057);
    let enemy = h.npc;
    h.app
        .world_mut()
        .get_mut::<crate::auras::AuraStatMods>(unit)
        .unwrap()
        .melee_hit_chance_pct = -100.0;
    h.app.world_mut().entity_mut(enemy).insert((
        shared::components::ThreatTable::default(),
        crate::creature_ai::CreatureState::Pursuing {
            target: h.caster.to_bits(),
        },
        crate::networking::CombatTarget(Some(h.caster)),
    ));
    crate::creature_aggro::add_threat(h.app.world_mut(), enemy, h.caster, 100.0);
    update_ai(h.app.world_mut());
    h.advance(4);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(enemy)
            .unwrap()
            .0,
        Some(h.caster)
    );
    assert!(
        h.inbox.log.iter().any(|event| event.spell_id == Some(62124)
            && event.kind
                == shared::protocol::CombatLogKind::Miss(shared::protocol::MissKind::Miss)),
        "actual taunt miss log missing: {:?}",
        h.inbox.log
    );
}

macro_rules! profile_case {
    ($test:ident, $entry:literal) => {
        #[test]
        fn $test() {
            assert_profile_combat($entry);
        }
    };
}
profile_case!(follower_labelled_garrick_protection_paladin, 209057);
profile_case!(follower_labelled_breka_protection_warrior, 215517);
profile_case!(follower_labelled_crenna_restoration_druid, 209072);
profile_case!(follower_labelled_henry_holy_priest, 215011);
profile_case!(follower_labelled_meredy_fire_mage, 209059);
profile_case!(follower_labelled_shuja_elemental_shaman, 209053);
profile_case!(follower_labelled_austin_beast_mastery_hunter, 209065);
profile_case!(follower_labelled_kybel_devastation_evoker, 215240);
profile_case!(follower_labelled_bo_windwalker_monk, 227734);
profile_case!(follower_labelled_lantha_demonology_warlock, 230861);
profile_case!(follower_labelled_ralia_feral_druid, 232800);
