use super::*;
use shared::components::{Health, Mana, Npc, UnitFactionTemplate};

fn profile(entry: u32, role: GroupRole, attack: u32, heal: u32) -> Profile {
    Profile {
        entry,
        name: format!("Follower {entry}"),
        class_id: 8,
        spec_id: 63,
        role,
        attack_spell: attack,
        cooldown_spell: 0,
        heal_spell: heal,
        taunt_spell: 0,
        form_spell: 0,
        melee: false,
    }
}

#[test]
fn every_player_role_forms_exactly_one_tank_one_healer_three_damage() {
    let profiles = vec![
        profile(209057, GroupRole::Tank, 275779, 0),
        profile(209072, GroupRole::Healer, 5176, 8936),
        profile(209059, GroupRole::Dps, 133, 0),
        profile(209053, GroupRole::Dps, 188196, 0),
        profile(209065, GroupRole::Dps, 193455, 0),
    ];
    for role in [GroupRole::Tank, GroupRole::Healer, GroupRole::Dps] {
        let party = select_party(&profiles, role).unwrap();
        assert_eq!(party.len(), 4);
        for (required, count) in [
            (GroupRole::Tank, 1),
            (GroupRole::Healer, 1),
            (GroupRole::Dps, 3),
        ] {
            assert_eq!(
                party.iter().filter(|p| p.role == required).count() + usize::from(role == required),
                count
            );
        }
    }
    assert!(select_party(&profiles[..2], GroupRole::Dps).is_err());
}

fn spawn_follower(h: &mut crate::spell_cast::test_harness::Harness, profile: Profile) -> Entity {
    let owner = h.caster;
    let unit = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(unit).insert((
        Npc {
            template_id: profile.entry,
            name: profile.name.clone(),
        },
        UnitFactionTemplate(1),
        Follower { owner, profile },
        Mana {
            current: 10000.0,
            max: 10000.0,
        },
        crate::stats::CharacterStats {
            spell_power: 100.0,
            healing_power: 100.0,
            base_mana: 10000.0,
            ..default()
        },
    ));
    unit
}

#[test]
fn damage_follower_casts_real_fireball_and_stops_after_enemy_death() {
    let mut h = crate::spell_cast::test_harness::harness(vec![], &[133], 8).unwrap();
    let follower = spawn_follower(&mut h, profile(209059, GroupRole::Dps, 133, 0));
    let enemy = h.npc;
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = Some(enemy);
    h.attack(enemy);
    h.advance(3);
    let before = h.health(enemy);
    for _ in 0..90 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        h.health(enemy) < before,
        "real spell execution must damage the hostile target"
    );
    h.app.world_mut().get_mut::<Health>(enemy).unwrap().current = 0.0;
    for _ in 0..90 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        h.app
            .world()
            .get::<crate::creature_ai::FollowOwner>(follower)
            .is_some()
    );
}

#[test]
fn healer_prioritizes_the_lowest_health_party_member_over_damage() {
    let mut h = crate::spell_cast::test_harness::harness(vec![], &[585, 2061, 137031], 5).unwrap();
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&db).unwrap());
    let mut holy = profile(215011, GroupRole::Healer, 585, 2061);
    holy.class_id = 5;
    holy.spec_id = 257;
    let (_, _, passives) = super::spawning::class_stats(h.app.world(), &holy, 10).unwrap();
    assert!(
        passives.contains(&137031),
        "Holy Priest's authored passive activates Flash Heal's native cost"
    );
    let healer = spawn_follower(&mut h, holy);
    crate::auras::sync_passives(
        h.app.world_mut(),
        healer,
        crate::auras::AuraSource::KnownSpell,
        &passives,
    );
    let injured = spawn_follower(&mut h, profile(209059, GroupRole::Dps, 0, 0));
    h.app
        .world_mut()
        .get_mut::<Health>(injured)
        .unwrap()
        .current = 100.0;
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 800.0;
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = Some(h.npc);
    h.attack(h.npc);
    h.advance(3);
    for _ in 0..40 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        h.health(injured) > 100.0,
        "healer must heal the most injured ally"
    );
    assert!(
        !h.inbox.log.iter().any(|event| {
            event.source == Some(healer.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage
        }),
        "the healer must prioritize healing, even while another party member attacks"
    );
    assert!(
        h.app.world().get::<Mana>(healer).unwrap().current < 10000.0,
        "ordinary spells must spend mana"
    );
}

#[test]
fn follower_combat_ownership_credits_real_spell_kills_to_the_character() {
    let mut h = crate::spell_cast::test_harness::harness(vec![], &[133], 8).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(30));
    let follower = spawn_follower(&mut h, profile(209059, GroupRole::Dps, 133, 0));
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 100.0,
        max: 100.0,
    });
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = Some(h.npc);
    h.attack(h.npc);
    h.advance(3);
    for _ in 0..120 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert_eq!(
        h.health(h.npc),
        0.0,
        "the companion must execute a real lethal Fireball"
    );
    let tap = h
        .app
        .world()
        .get::<crate::creature_tap::CreatureTap>(h.npc)
        .expect("companion damage must tap its authenticated owner");
    assert_eq!(tap.tappers, vec![30]);
    assert_eq!(tap.xp_factor(100.0), 1.0);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitTap>(h.npc)
            .unwrap()
            .0,
        vec![30]
    );
    assert!(
        h.app
            .world()
            .get::<crate::persistence::ClientId>(follower)
            .is_none(),
        "companions must not become persisted characters"
    );
}

#[test]
fn follower_combat_ownership_allows_ruby_faction_to_retaliate() {
    let mut h = crate::spell_cast::test_harness::harness(vec![], &[458324], 8).unwrap();
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&db).unwrap());
    let kit = profile(209059, GroupRole::Dps, 133, 0);
    h.app
        .insert_resource(crate::spell_cast::test_harness::load_native_creature_world(
            &[kit.entry],
        ));
    let unit = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    super::spawning::initialize(h.app.world_mut(), h.caster, unit, kit).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(UnitFactionTemplate(16));
    assert!(
        crate::spell_cast::targets::relation_matches(
            h.app.world(),
            h.npc,
            unit,
            crate::spell_cast::targets::Relation::Enemy
        ),
        "Ruby faction16 must be able to fight an owned companion"
    );
}

#[test]
fn follower_initialization_uses_native_stats_without_unassigned_follower_aura() {
    let mut h = crate::spell_cast::test_harness::harness(vec![], &[458324], 8).unwrap();
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&db).unwrap());
    let kit = profile(209059, GroupRole::Dps, 133, 0);
    h.app
        .insert_resource(crate::spell_cast::test_harness::load_native_creature_world(
            &[kit.entry],
        ));
    let expected = super::spawning::class_stats(h.app.world(), &kit, 10)
        .unwrap()
        .0;
    let unit = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    super::spawning::initialize(h.app.world_mut(), h.caster, unit, kit).unwrap();
    assert!(!crate::auras::has_aura(h.app.world(), unit, 458324));
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitLevel>(unit)
            .unwrap()
            .0,
        10
    );
    assert_eq!(
        h.app.world().get::<Health>(unit).unwrap().max,
        expected.max_health
    );
    assert_eq!(
        h.app.world().get::<Mana>(unit).unwrap().max,
        expected.base_mana
    );
}

#[test]
fn follower_missing_spell_metadata_never_substitutes_melee_combat() {
    let mut h = crate::spell_cast::test_harness::harness(vec![], &[], 8).unwrap();
    let unit = spawn_follower(&mut h, profile(209059, GroupRole::Dps, u32::MAX, 0));
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = Some(h.npc);
    h.attack(h.npc);
    h.advance(3);
    let before = h.inbox.log.len();
    for _ in 0..60 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        !h.inbox.log[before..].iter().any(|event| {
            event.source == Some(unit.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage
        }),
        "missing metadata must not cause follower damage; owner attacks remain real"
    );
    assert!(
        h.app
            .world()
            .get::<crate::creature_ai::FollowOwner>(unit)
            .is_some()
    );
}

#[test]
fn tank_recovers_a_party_attackers_threat_with_real_hand_of_reckoning() {
    let mut h = crate::spell_cast::test_harness::harness(vec![], &[62124], 2).unwrap();
    let mut kit = profile(209057, GroupRole::Tank, 0, 0);
    kit.taunt_spell = 62124;
    kit.melee = true;
    let tank = spawn_follower(&mut h, kit);
    // This tests successful threat recovery; native taunt misses have a
    // separate forced-miss control in profile_behavior_tests.
    h.app
        .world_mut()
        .entity_mut(tank)
        .insert(crate::auras::AuraStatMods {
            melee_hit_chance_pct: 100.0,
            ..default()
        });
    let enemy = h.npc;
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
        Some(tank),
        "attacker {enemy:?} must switch from owner {:?} to tank {tank:?}; combat {:?}",
        h.caster,
        h.inbox.log
    );
    assert!(
        h.app
            .world()
            .get::<shared::components::ThreatTable>(enemy)
            .unwrap()
            .threat_for(tank.to_bits())
            >= 100.0
    );
}
