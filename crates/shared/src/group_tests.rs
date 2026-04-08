use super::*;

#[test]
fn party_create() {
    let party = Party::new(1);
    assert_eq!(party.leader, 1);
    assert_eq!(party.size(), 1);
    assert!(party.contains(1));
}

#[test]
fn party_invite_and_accept() {
    let mut party = Party::new(1);
    let invite = party.invite(2).unwrap();
    assert_eq!(invite.invitee, 2);

    party.accept(2).unwrap();
    assert_eq!(party.size(), 2);
    assert!(party.contains(2));
}

#[test]
fn party_reject_duplicate() {
    let party = Party::new(1);
    assert_eq!(party.invite(1), Err(PartyError::AlreadyMember));
}

#[test]
fn party_full_at_5() {
    let mut party = Party::new(1);
    for i in 2..=5 {
        party.accept(i).unwrap();
    }
    assert!(party.is_full());
    assert_eq!(party.invite(6), Err(PartyError::Full));
    assert_eq!(party.accept(6), Err(PartyError::Full));
}

#[test]
fn party_leave_promotes_leader() {
    let mut party = Party::new(1);
    party.accept(2).unwrap();
    party.accept(3).unwrap();

    let disband = party.leave(1); // leader leaves
    assert!(!disband);
    assert_eq!(party.leader, 2); // next member promoted
    assert!(!party.contains(1));
}

#[test]
fn party_leave_last_disbands() {
    let mut party = Party::new(1);
    party.accept(2).unwrap();
    party.leave(1);
    let disband = party.leave(2);
    assert!(disband); // 0 members
}

#[test]
fn party_disband_only_leader() {
    let mut party = Party::new(1);
    party.accept(2).unwrap();
    assert_eq!(party.disband(2), Err(PartyError::NotLeader));
    assert!(party.disband(1).is_ok());
    assert_eq!(party.size(), 0);
}

#[test]
fn party_leave_non_leader_keeps_leader() {
    let mut party = Party::new(1);
    party.accept(2).unwrap();
    party.accept(3).unwrap();
    party.leave(2);
    assert_eq!(party.leader, 1);
    assert_eq!(party.size(), 2);
}

// --- Raid tests ---

#[test]
fn raid_from_party() {
    let mut party = Party::new(1);
    party.accept(2).unwrap();
    party.accept(3).unwrap();
    let raid = Raid::from_party(&party);
    assert_eq!(raid.leader, 1);
    assert_eq!(raid.total_members(), 3);
    assert_eq!(raid.subgroup_of(1), Some(0));
    assert_eq!(raid.subgroup_of(2), Some(0));
}

#[test]
fn raid_add_fills_subgroups() {
    let party = Party::new(1);
    let mut raid = Raid::from_party(&party);
    // Fill subgroup 0 (already has 1 member)
    for i in 2..=5 {
        raid.add_member(i).unwrap();
    }
    assert_eq!(raid.subgroups[0].len(), 5);
    // Next member goes to subgroup 1
    let group = raid.add_member(6).unwrap();
    assert_eq!(group, 1);
    assert_eq!(raid.subgroup_of(6), Some(1));
}

#[test]
fn raid_full_at_40() {
    let party = Party::new(1);
    let mut raid = Raid::from_party(&party);
    for i in 2..=40 {
        raid.add_member(i).unwrap();
    }
    assert_eq!(raid.total_members(), 40);
    assert_eq!(raid.add_member(41), Err(PartyError::Full));
}

#[test]
fn raid_move_subgroup() {
    let party = Party::new(1);
    let mut raid = Raid::from_party(&party);
    raid.add_member(2).unwrap();
    raid.move_to_subgroup(2, 3).unwrap();
    assert_eq!(raid.subgroup_of(2), Some(3));
    assert_eq!(raid.subgroups[0].len(), 1);
}

#[test]
fn raid_move_to_full_subgroup_fails() {
    let party = Party::new(1);
    let mut raid = Raid::from_party(&party);
    for i in 2..=5 {
        raid.add_member(i).unwrap();
    }
    raid.add_member(6).unwrap(); // goes to group 1
    assert_eq!(raid.move_to_subgroup(6, 0), Err(PartyError::Full));
}

#[test]
fn raid_leave_promotes_leader() {
    let mut party = Party::new(1);
    party.accept(2).unwrap();
    let mut raid = Raid::from_party(&party);
    raid.leave(1);
    assert_eq!(raid.leader, 2);
    assert!(!raid.contains(1));
}

#[test]
fn raid_reject_duplicate() {
    let party = Party::new(1);
    let mut raid = Raid::from_party(&party);
    assert_eq!(raid.add_member(1), Err(PartyError::AlreadyMember));
}

// --- Role assignment tests ---

#[test]
fn role_assign_and_query() {
    let mut roles = RoleAssignments::default();
    roles.assign(1, GroupRole::Tank);
    roles.assign(2, GroupRole::Healer);
    roles.assign(3, GroupRole::Dps);
    assert_eq!(roles.role_of(1), Some(GroupRole::Tank));
    assert_eq!(roles.role_of(2), Some(GroupRole::Healer));
    assert_eq!(roles.role_of(99), None);
}

#[test]
fn role_overwrite() {
    let mut roles = RoleAssignments::default();
    roles.assign(1, GroupRole::Dps);
    roles.assign(1, GroupRole::Tank);
    assert_eq!(roles.role_of(1), Some(GroupRole::Tank));
}

#[test]
fn role_remove() {
    let mut roles = RoleAssignments::default();
    roles.assign(1, GroupRole::Tank);
    roles.remove(1);
    assert_eq!(roles.role_of(1), None);
}

#[test]
fn role_counts() {
    let mut roles = RoleAssignments::default();
    roles.assign(1, GroupRole::Tank);
    roles.assign(2, GroupRole::Healer);
    roles.assign(3, GroupRole::Dps);
    roles.assign(4, GroupRole::Dps);
    roles.assign(5, GroupRole::Dps);
    assert_eq!(roles.counts(), (1, 1, 3));
}

#[test]
fn role_standard_composition() {
    let mut roles = RoleAssignments::default();
    roles.assign(1, GroupRole::Tank);
    roles.assign(2, GroupRole::Healer);
    roles.assign(3, GroupRole::Dps);
    assert!(roles.has_standard_composition());
}

#[test]
fn role_no_tank_not_standard() {
    let mut roles = RoleAssignments::default();
    roles.assign(1, GroupRole::Healer);
    roles.assign(2, GroupRole::Dps);
    assert!(!roles.has_standard_composition());
}

// --- Ready check tests ---

#[test]
fn ready_check_all_ready() {
    let mut check = ReadyCheck::new(&[1, 2, 3]);
    assert!(!check.all_responded());
    check.respond(1, ReadyResponse::Ready);
    check.respond(2, ReadyResponse::Ready);
    check.respond(3, ReadyResponse::Ready);
    assert!(check.all_responded());
    assert!(check.all_ready());
}

#[test]
fn ready_check_not_ready() {
    let mut check = ReadyCheck::new(&[1, 2]);
    check.respond(1, ReadyResponse::Ready);
    check.respond(2, ReadyResponse::NotReady);
    assert!(check.all_responded());
    assert!(!check.all_ready());
}

#[test]
fn ready_check_timeout() {
    let mut check = ReadyCheck::new(&[1, 2]);
    check.respond(1, ReadyResponse::Ready);
    assert!(!check.tick(20.0));
    assert!(check.tick(15.0)); // 35s > 30s timeout
}

#[test]
fn ready_check_counts() {
    let mut check = ReadyCheck::new(&[1, 2, 3, 4]);
    check.respond(1, ReadyResponse::Ready);
    check.respond(2, ReadyResponse::NotReady);
    let (pending, ready, not_ready) = check.counts();
    assert_eq!(pending, 2);
    assert_eq!(ready, 1);
    assert_eq!(not_ready, 1);
}

#[test]
fn ready_check_unknown_player() {
    let mut check = ReadyCheck::new(&[1, 2]);
    assert!(!check.respond(99, ReadyResponse::Ready));
}

// --- Group loot integration tests ---

#[test]
fn party_default_loot_mode_personal() {
    let party = Party::new(1);
    assert_eq!(party.loot_mode, LootMode::PersonalLoot);
}

#[test]
fn party_set_loot_mode_leader_only() {
    let mut party = Party::new(1);
    party.accept(2).unwrap();
    assert!(party.set_loot_mode(1, LootMode::NeedBeforeGreed).is_ok());
    assert_eq!(party.loot_mode, LootMode::NeedBeforeGreed);
    assert_eq!(
        party.set_loot_mode(2, LootMode::FreeForAll),
        Err(PartyError::NotLeader)
    );
}

#[test]
fn party_round_robin_rotates() {
    let mut party = Party::new(1);
    party.accept(2).unwrap();
    party.accept(3).unwrap();
    assert_eq!(party.next_round_robin(), 1);
    assert_eq!(party.next_round_robin(), 2);
    assert_eq!(party.next_round_robin(), 3);
    assert_eq!(party.next_round_robin(), 1); // wraps
}

#[test]
fn raid_inherits_loot_mode() {
    let mut party = Party::new(1);
    party.set_loot_mode(1, LootMode::NeedBeforeGreed).unwrap();
    let raid = Raid::from_party(&party);
    assert_eq!(raid.loot_mode, LootMode::NeedBeforeGreed);
}

// --- Group XP sharing tests ---

#[test]
fn party_xp_equal_levels() {
    let mut party = Party::new(1);
    party.accept(2).unwrap();
    let info = vec![
        MemberInfo {
            entity: 1,
            level: 80,
            distance_from_kill: 5.0,
        },
        MemberInfo {
            entity: 2,
            level: 80,
            distance_from_kill: 10.0,
        },
    ];
    let shares = party_kill_xp(&party, 80, &info);
    assert_eq!(shares.len(), 2);
    // Equal levels → equal shares
    assert_eq!(shares[0].1, shares[1].1);
    assert!(shares[0].1 > 0);
}

#[test]
fn party_xp_out_of_range_gets_zero() {
    let mut party = Party::new(1);
    party.accept(2).unwrap();
    let info = vec![
        MemberInfo {
            entity: 1,
            level: 80,
            distance_from_kill: 5.0,
        },
        MemberInfo {
            entity: 2,
            level: 80,
            distance_from_kill: 200.0,
        }, // out of range
    ];
    let shares = party_kill_xp(&party, 80, &info);
    assert!(shares[0].1 > 0);
    assert_eq!(shares[1].1, 0);
}

#[test]
fn party_xp_level_weighted() {
    let mut party = Party::new(1);
    party.accept(2).unwrap();
    let info = vec![
        MemberInfo {
            entity: 1,
            level: 80,
            distance_from_kill: 5.0,
        },
        MemberInfo {
            entity: 2,
            level: 40,
            distance_from_kill: 5.0,
        },
    ];
    let shares = party_kill_xp(&party, 80, &info);
    assert!(shares[0].1 > shares[1].1); // higher level gets more
}

#[test]
fn raid_xp_distributes_across_subgroups() {
    let mut party = Party::new(1);
    party.accept(2).unwrap();
    let mut raid = Raid::from_party(&party);
    raid.add_member(3).unwrap();
    let info = vec![
        MemberInfo {
            entity: 1,
            level: 80,
            distance_from_kill: 5.0,
        },
        MemberInfo {
            entity: 2,
            level: 80,
            distance_from_kill: 5.0,
        },
        MemberInfo {
            entity: 3,
            level: 80,
            distance_from_kill: 5.0,
        },
    ];
    let shares = raid_kill_xp(&raid, 80, &info);
    assert_eq!(shares.len(), 3);
    assert!(shares.iter().all(|(_, xp)| *xp > 0));
}

// --- Shared threat tests ---

#[test]
fn group_damage_threat_shared_table() {
    let mut table = ThreatTable::default();
    // Two party members both damage the same mob
    apply_group_damage_threat(&mut table, 1, 500.0, 1.0);
    apply_group_damage_threat(&mut table, 2, 300.0, 1.0);
    assert_eq!(table.threat_for(1), 500.0);
    assert_eq!(table.threat_for(2), 300.0);
    assert_eq!(table.top_threat().unwrap().entity, 1);
}

#[test]
fn group_heal_threat_split_across_mobs() {
    let mut table1 = ThreatTable::default();
    let mut table2 = ThreatTable::default();
    // Healer heals 1000, split across 2 mobs
    apply_group_heal_threat(&mut [&mut table1, &mut table2], 3, 1000.0, 1.0);
    // Each mob gets 500 heal → 500 * 0.5 = 250 threat
    assert!((table1.threat_for(3) - 250.0).abs() < 0.01);
    assert!((table2.threat_for(3) - 250.0).abs() < 0.01);
}

#[test]
fn group_heal_no_mobs_no_crash() {
    apply_group_heal_threat(&mut [], 1, 1000.0, 1.0);
}

#[test]
fn engaged_mobs_filters_by_group() {
    let mut t1 = ThreatTable::default();
    t1.add_damage_threat(1, 100.0, 1.0); // player 1 in group

    let mut t2 = ThreatTable::default();
    t2.add_damage_threat(99, 100.0, 1.0); // player 99 NOT in group

    let mobs: Vec<(u64, &ThreatTable)> = vec![(10, &t1), (20, &t2)];
    let group_members = vec![1, 2, 3];
    let engaged = engaged_mobs(&mobs, &group_members);
    assert_eq!(engaged, vec![10]); // only mob 10 is engaged by the group
}

#[test]
fn heal_threat_with_tank_modifier() {
    let mut table = ThreatTable::default();
    // Tank heals with 1.43x threat modifier
    apply_group_heal_threat(&mut [&mut table], 1, 1000.0, 1.43);
    // 1000 * 0.5 * 1.43 = 715
    assert!((table.threat_for(1) - 715.0).abs() < 0.01);
}
