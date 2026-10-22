use super::*;

#[test]
fn follower_party_loopback_keeps_npc_members_online_and_updates_their_health() {
    let mut app = setup_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(250),
    ));
    let player = connect(&mut app, 1, "Alice");
    app.world_mut()
        .entity_mut(player.player)
        .insert(crate::maps::DungeonDifficulty(205));
    pump(&mut app);
    let mut online = OnlineView::from([(
        1,
        OnlineMember {
            entity: player.player.to_bits(),
            name: "Alice".into(),
            class: 2,
            level: 80,
            portrait: Default::default(),
            dungeon_difficulty: 205,
            raid_difficulty: Default::default(),
        },
    )]);
    let mut members = Vec::new();
    let mut tank = None;
    for (entry, name, class_id, spec_id, role, display_id) in [
        (
            209057,
            "Captain Garrick",
            2,
            66,
            shared::group::GroupRole::Tank,
            113753,
        ),
        (
            209072,
            "Crenna Earth-Daughter",
            11,
            105,
            shared::group::GroupRole::Healer,
            113757,
        ),
        (
            209059,
            "Meredy Huntswell",
            8,
            63,
            shared::group::GroupRole::Dps,
            113755,
        ),
        (
            209053,
            "Shuja Grimaxe",
            7,
            262,
            shared::group::GroupRole::Dps,
            113752,
        ),
    ] {
        let follower = crate::followers::Follower {
            owner: player.player,
            profile: crate::followers::Profile {
                entry,
                name: name.into(),
                class_id,
                spec_id,
                role,
                attack_spell: 0,
                cooldown_spell: 0,
                heal_spell: 0,
                taunt_spell: 0,
                form_spell: 0,
                melee: false,
            },
        };
        let unit = app
            .world_mut()
            .spawn((
                follower.clone(),
                shared::components::Npc {
                    template_id: entry,
                    name: name.into(),
                },
                UnitLevel(80),
                shared::components::ModelDisplay { display_id },
                Health {
                    current: 400.0,
                    max: 1000.0,
                },
                Position {
                    x: 10.0,
                    y: 0.0,
                    z: 0.0,
                },
            ))
            .id();
        let member = crate::followers::Follower::member_id(unit);
        online.insert(member, follower.online(unit, 80, 205, display_id));
        members.push((member, role));
        if entry == 209057 {
            tank = Some(unit);
        }
    }
    app.world_mut()
        .resource_mut::<GroupRegistry>()
        .create_companion_party(1, shared::group::GroupRole::Dps, &members, &online)
        .unwrap();
    pump(&mut app);
    let roster = last_roster(&mut app, &player);
    assert_eq!(roster.members.len(), 5);
    for member in &roster.members {
        assert_eq!(
            member.portrait.creature_display_id,
            online[&member.character_id].portrait.creature_display_id,
            "presence refresh must retain the actual head for {}",
            member.name
        );
    }
    assert!(
        roster
            .members
            .iter()
            .all(|member| member.online && member.entity.is_some())
    );
    for (role, count) in [
        (GroupRoleSnapshot::Tank, 1),
        (GroupRoleSnapshot::Healer, 1),
        (GroupRoleSnapshot::Damage, 3),
    ] {
        assert_eq!(
            roster
                .members
                .iter()
                .filter(|member| member.role == role)
                .count(),
            count
        );
    }
    let states = member_states(&mut app, &player);
    assert!(
        states
            .iter()
            .flat_map(|update| &update.members)
            .any(|member| member.name == "Captain Garrick"
                && member.health == 400
                && member.max_health == 1000)
    );
    app.world_mut()
        .get_mut::<Health>(tank.unwrap())
        .unwrap()
        .current = 125.0;
    pump(&mut app);
    let states = member_states(&mut app, &player);
    assert!(
        states
            .iter()
            .flat_map(|update| &update.members)
            .any(|member| member.name == "Captain Garrick" && member.health == 125)
    );
}
