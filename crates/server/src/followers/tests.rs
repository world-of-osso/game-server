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
    let mut h = crate::spell_cast::test_harness::harness(vec![], &[585, 2061], 5).unwrap();
    let healer = spawn_follower(&mut h, profile(215011, GroupRole::Healer, 585, 2061));
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
    let enemy_before = h.health(h.npc);
    for _ in 0..40 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        h.health(injured) > 100.0,
        "healer must heal the most injured ally"
    );
    assert_eq!(h.health(h.npc), enemy_before);
    assert!(
        h.app.world().get::<Mana>(healer).unwrap().current < 10000.0,
        "ordinary spells must spend mana"
    );
}
