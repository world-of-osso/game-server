use super::*;
use shared::protocol::{
    TraitLoadoutInfo, TraitLoadoutOperation, TraitLoadoutRequest, TraitLoadoutsSnapshot,
};

fn change(app: &mut App, player: Entity, operation: TraitLoadoutOperation) {
    send_request(
        app,
        player,
        TraitLoadoutRequest {
            spec_id: SPEC_RETRIBUTION,
            operation,
        },
    );
}
/// Starter is an authoritative built-in selection, never a user-named config.
#[test]
fn talent_starter_activation_level_up_relog_and_manual_edit() {
    let classes = class_spells().expect("world.db");
    let (mut app, player) = spawn_level_80_paladin(temp_database(), classes, SPEC_RETRIBUTION);
    app.world_mut().get_mut::<Level>(player).unwrap().0 = 10;
    advance(&mut app);
    drain(&mut app, player);
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Activate { id: u32::MAX },
    );
    let result = received::<TraitCommitResult>(&mut app, player)
        .pop()
        .unwrap();
    assert!(result.ok, "Starter activation: {:?}", result.reason);
    let list = received::<TraitLoadoutsSnapshot>(&mut app, player)
        .pop()
        .unwrap();
    assert_eq!(list.selected_id, u32::MAX);
    assert!(list.configs.is_empty());
    let at_ten = app.world().get::<TraitLoadout>(player).unwrap().purchases();
    assert_eq!(at_ten.iter().map(|p| p.ranks).sum::<u32>(), 1);
    assert_eq!(at_ten[0].node_id, 109999); // Authored loadout922 OrderIndex1.
    app.world_mut().get_mut::<Level>(player).unwrap().0 = 11;
    advance(&mut app);
    let at_eleven = app.world().get::<TraitLoadout>(player).unwrap().purchases();
    assert_eq!(at_eleven.iter().map(|p| p.ranks).sum::<u32>(), 2);
    assert!(at_eleven.contains(&at_ten[0]));
    app.world_mut().despawn(player);
    let relog = spawn_player(&mut app, 9, HUMAN, PALADIN, 11);
    advance(&mut app);
    assert_eq!(
        app.world().get::<TraitLoadout>(relog).unwrap().purchases(),
        at_eleven
    );
    assert_eq!(
        received::<TraitLoadoutsSnapshot>(&mut app, relog)
            .last()
            .unwrap()
            .selected_id,
        u32::MAX
    );
    send_request(
        &mut app,
        relog,
        CommitTraitConfig {
            spec_id: SPEC_RETRIBUTION,
            entries: vec![],
        },
    );
    assert!(
        received::<TraitCommitResult>(&mut app, relog)
            .last()
            .unwrap()
            .ok
    );
    assert_eq!(
        received::<TraitLoadoutsSnapshot>(&mut app, relog)
            .last()
            .unwrap()
            .selected_id,
        0
    );
    app.world_mut().get_mut::<Level>(relog).unwrap().0 = 12;
    advance(&mut app);
    assert!(
        app.world()
            .get::<TraitLoadout>(relog)
            .unwrap()
            .purchases()
            .is_empty()
    );
}

#[test]
fn talent_starter_preserves_named_configs_spec_selection_and_rejected_edits() {
    let classes = class_spells().expect("world.db");
    let (mut app, player) = spawn_level_80_paladin(temp_database(), classes, SPEC_RETRIBUTION);
    drain(&mut app, player);
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Create {
            name: "Saved".into(),
            entries: selections(&[BLADE_OF_JUSTICE]),
        },
    );
    let saved = received::<TraitLoadoutsSnapshot>(&mut app, player)
        .pop()
        .unwrap()
        .selected_id;
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Activate { id: u32::MAX },
    );
    let initial = app.world().get::<TraitLoadout>(player).unwrap().clone();
    let list = received::<TraitLoadoutsSnapshot>(&mut app, player)
        .pop()
        .unwrap();
    assert!(list.starter_available);
    assert_eq!(
        list.configs,
        vec![TraitLoadoutInfo {
            id: saved,
            name: "Saved".into()
        }]
    );
    send_request(
        &mut app,
        player,
        CommitTraitConfig {
            spec_id: SPEC_RETRIBUTION,
            entries: vec![TraitEntrySelection {
                node_id: u32::MAX,
                entry_id: 1,
                rank: 1,
            }],
        },
    );
    assert!(
        !received::<TraitCommitResult>(&mut app, player)
            .last()
            .unwrap()
            .ok
    );
    assert_eq!(app.world().get::<TraitLoadout>(player).unwrap(), &initial);
    assert_eq!(
        load_trait_configs(&app.world().resource::<GameDatabase>().db, 9)
            .spec_mut(SPEC_RETRIBUTION)
            .selected_id,
        u32::MAX
    );
    send_request(&mut app, player, SetSpecialization { spec_id: SPEC_HOLY });
    send_request(
        &mut app,
        player,
        SetSpecialization {
            spec_id: SPEC_RETRIBUTION,
        },
    );
    assert_eq!(app.world().get::<TraitLoadout>(player).unwrap(), &initial);
    assert_eq!(
        received::<TraitLoadoutsSnapshot>(&mut app, player)
            .last()
            .unwrap()
            .selected_id,
        u32::MAX
    );
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Activate { id: saved },
    );
    assert_eq!(
        received::<TraitLoadoutsSnapshot>(&mut app, player)
            .last()
            .unwrap()
            .selected_id,
        saved
    );
    assert_eq!(
        app.world().get::<TraitLoadout>(player).unwrap().purchases(),
        purchases(&[BLADE_OF_JUSTICE])
    );
}

#[test]
fn talent_loadouts_create_apply_switch_rename_delete_and_login_restore() {
    let classes = class_spells().expect("world.db");
    let (mut app, player) = spawn_level_80_paladin(temp_database(), classes, SPEC_RETRIBUTION);
    drain(&mut app, player);
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Create {
            name: "Raid".into(),
            entries: selections(&[BLADE_OF_JUSTICE, DIVINE_STORM, EXPURGATION]),
        },
    );
    let list = received::<TraitLoadoutsSnapshot>(&mut app, player)
        .pop()
        .expect("created loadout list");
    let raid = list.selected_id;
    assert_eq!(
        list.configs,
        vec![TraitLoadoutInfo {
            id: raid,
            name: "Raid".into()
        }]
    );
    assert!(
        app.world()
            .get::<KnownSpells>(player)
            .unwrap()
            .contains(383344)
    );
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Create {
            name: "Dungeons".into(),
            entries: vec![],
        },
    );
    let dungeon = received::<TraitLoadoutsSnapshot>(&mut app, player)
        .pop()
        .unwrap()
        .selected_id;
    assert!(
        !app.world()
            .get::<KnownSpells>(player)
            .unwrap()
            .contains(383344)
    );
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Activate { id: raid },
    );
    assert!(
        app.world()
            .get::<KnownSpells>(player)
            .unwrap()
            .contains(383344)
    );
    send_request(
        &mut app,
        player,
        CommitTraitConfig {
            spec_id: SPEC_RETRIBUTION,
            entries: selections(&[BLADE_OF_JUSTICE]),
        },
    );
    assert!(
        !app.world()
            .get::<KnownSpells>(player)
            .unwrap()
            .contains(383344)
    );
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Rename {
            id: raid,
            name: "Single Target".into(),
        },
    );
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Activate { id: dungeon },
    );
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Activate { id: raid },
    );
    let data = load_trait_configs(&app.world().resource::<GameDatabase>().db, 9);
    assert_eq!(data.spec(SPEC_RETRIBUTION), purchases(&[BLADE_OF_JUSTICE]));
    let saved = data
        .specs
        .iter()
        .find(|s| s.spec_id == SPEC_RETRIBUTION)
        .unwrap();
    assert_eq!(saved.selected_id, raid);
    assert_eq!(saved.configs[0].name, "Single Target");
    // Actual login system, same redb, new player entity.
    app.world_mut().despawn(player);
    let relog = spawn_player(&mut app, 9, HUMAN, PALADIN, 80);
    advance(&mut app);
    let lists = received::<TraitLoadoutsSnapshot>(&mut app, relog);
    assert_eq!(lists.last().unwrap().selected_id, raid);
    assert_eq!(lists.last().unwrap().configs.len(), 2);
    change(&mut app, relog, TraitLoadoutOperation::Delete { id: raid });
    let list = received::<TraitLoadoutsSnapshot>(&mut app, relog)
        .pop()
        .unwrap();
    assert_eq!(list.selected_id, 0);
    assert_eq!(
        list.configs,
        vec![TraitLoadoutInfo {
            id: dungeon,
            name: "Dungeons".into()
        }]
    );
    assert_eq!(
        load_trait_configs(&app.world().resource::<GameDatabase>().db, 9).spec(SPEC_RETRIBUTION),
        purchases(&[BLADE_OF_JUSTICE])
    );
}

#[test]
fn talent_loadouts_metadata_does_not_replace_the_clients_pending_allocation() {
    let classes = class_spells().expect("world.db");
    let (mut app, player) = spawn_level_80_paladin(temp_database(), classes, SPEC_RETRIBUTION);
    drain(&mut app, player);
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Create {
            name: "Raid".into(),
            entries: selections(&[BLADE_OF_JUSTICE]),
        },
    );
    let raid = received::<TraitLoadoutsSnapshot>(&mut app, player)
        .pop()
        .unwrap()
        .selected_id;
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Create {
            name: "Solo".into(),
            entries: vec![],
        },
    );
    let solo = received::<TraitLoadoutsSnapshot>(&mut app, player)
        .pop()
        .unwrap()
        .selected_id;
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Activate { id: raid },
    );
    drain(&mut app, player);
    let before = app.world().get::<TraitLoadout>(player).unwrap().clone();
    for operation in [
        TraitLoadoutOperation::Rename {
            id: raid,
            name: "Raid Updated".into(),
        },
        TraitLoadoutOperation::Delete { id: solo },
        TraitLoadoutOperation::Delete { id: raid },
    ] {
        change(&mut app, player, operation);
        assert!(
            received::<TraitConfigSnapshot>(&mut app, player).is_empty(),
            "Metadata must not discard the client's pending ranks via an allocation snapshot"
        );
        assert_eq!(app.world().get::<TraitLoadout>(player), Some(&before));
        assert_eq!(received::<TraitLoadoutsSnapshot>(&mut app, player).len(), 1);
    }
    assert_eq!(
        load_trait_configs(&app.world().resource::<GameDatabase>().db, 9).spec(SPEC_RETRIBUTION),
        purchases(&[BLADE_OF_JUSTICE])
    );
}

#[test]
fn talent_loadouts_reject_invalid_allocation_wrong_spec_and_foreign_id_without_mutation() {
    let classes = class_spells().expect("world.db");
    let (mut app, player) = spawn_level_80_paladin(temp_database(), classes, SPEC_RETRIBUTION);
    drain(&mut app, player);
    let before = load_trait_configs(&app.world().resource::<GameDatabase>().db, 9);
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Create {
            name: "Invalid".into(),
            entries: selections(&[AVENGING_WRATH]),
        },
    );
    change(
        &mut app,
        player,
        TraitLoadoutOperation::Activate { id: 999 },
    );
    send_request(
        &mut app,
        player,
        TraitLoadoutRequest {
            spec_id: SPEC_HOLY,
            operation: TraitLoadoutOperation::Create {
                name: "Holy".into(),
                entries: vec![],
            },
        },
    );
    assert_eq!(
        received::<TraitCommitResult>(&mut app, player)
            .iter()
            .filter(|r| !r.ok)
            .count(),
        3
    );
    assert!(received::<TraitLoadoutsSnapshot>(&mut app, player).is_empty());
    assert_eq!(
        load_trait_configs(&app.world().resource::<GameDatabase>().db, 9),
        before
    );
}
