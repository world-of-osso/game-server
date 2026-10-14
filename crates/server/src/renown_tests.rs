use super::*;
use crate::bank_test_harness::Harness;

#[test]
fn db2_follow_install_renown_reads_new_active_faction_without_recompiling() {
    let path = std::env::temp_dir().join(format!("db2-follow-renown-{}.json", std::process::id()));
    for (build, id) in [("12.1.0.69933", 12), ("12.1.5.70391", 55)] {
        let fixture = serde_json::json!({"build": build, "major_factions": [{
            "id": id, "faction_id": 2655, "name": "Source faction", "description": "Current input",
            "expansion": 10, "max_level": 25, "level_threshold": 2500,
            "player_condition_id": 0, "unlock_description": null, "rewards": []
        }]});
        std::fs::write(&path, fixture.to_string()).unwrap();
        let current = read_catalog(&path).unwrap();
        assert_eq!(current.build, build);
        assert_eq!(current.major_factions.len(), 1);
        assert_eq!(current.major_factions[0].id, id);
        assert_eq!(current.major_factions[0].name, "Source faction");
    }
    std::fs::remove_file(path).unwrap();
}

const MARUUK: u32 = 12;

fn maruuk() -> &'static MajorFaction {
    catalog()
        .iter()
        .find(|faction| faction.id == MARUUK)
        .unwrap()
}

fn query(h: &mut Harness) -> RenownSnapshot {
    request(h, RenownRequest::Query)
}

fn request(h: &mut Harness, request: RenownRequest) -> RenownSnapshot {
    request_with_updates(h, request).0
}

fn request_with_updates(
    h: &mut Harness,
    request: RenownRequest,
) -> (RenownSnapshot, Vec<shared::protocol::ReputationStateUpdate>) {
    h.app.add_systems(Update, receive_requests);
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<RenownRequest>::default(),
        MessageReceiver::<RenownRequest>::default(),
        MessageSender::<RenownSnapshot>::default(),
        MessageReceiver::<RenownSnapshot>::default(),
        MessageSender::<shared::protocol::ReputationStateUpdate>::default(),
        MessageReceiver::<shared::protocol::ReputationStateUpdate>::default(),
    ));
    h.app
        .world_mut()
        .get_mut::<MessageSender<RenownRequest>>(h.connection)
        .unwrap()
        .send::<InstanceChannel>(request);
    let mut replies = Vec::new();
    let mut updates = Vec::new();
    for _ in 0..10 {
        h.app.update();
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<RenownSnapshot>>(h.connection)
                .unwrap()
                .receive(),
        );
        updates.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<shared::protocol::ReputationStateUpdate>>(h.connection)
                .unwrap()
                .receive(),
        );
    }
    (replies.pop().expect("renown reply"), updates)
}

#[test]
fn renown_watch_query_reads_live_reputation_and_zero_clears_persistently() {
    use shared::reputation::CharacterReputation;
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Watchseeker").unwrap();
    let mut reputation = CharacterReputation::default();
    reputation.watch(72);
    h.app.world_mut().entity_mut(h.player).insert(reputation);
    assert_eq!(query(&mut h).watched_faction, Some(72));
    let (cleared, updates) =
        request_with_updates(&mut h, RenownRequest::SetWatchedFaction { faction_id: 0 });
    assert_eq!(cleared.watched_faction, None);
    assert_eq!(cleared.watch_error, None);
    assert!(
        updates.iter().any(|update| update
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.watched.is_none())),
        "Clearing must publish the main-screen bar removal"
    );
    assert_eq!(
        h.app
            .world()
            .get::<CharacterReputation>(h.player)
            .unwrap()
            .watched_faction(),
        None
    );
    let character = h.character_id;
    drop(h);
    let reopened = Database::open(file.path()).unwrap();
    let stored =
        crate::persistence::load_reputation(&reopened, character, || panic!("must persist"));
    assert_eq!(stored.watched_faction(), None);
}

#[test]
fn renown_watch_published_ungated_faction_acknowledges_bar_and_survives_reopen() {
    use shared::reputation::CharacterReputation;
    let faction = catalog()
        .iter()
        .find(|faction| faction.id == 55)
        .expect("published Labyrinth faction");
    assert_eq!(
        unlock(faction),
        RenownUnlock::Ungated,
        "eligibility must come from the published source, not a fixture override"
    );
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Nativewatch").unwrap();
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(CharacterReputation::default());
    let (reply, updates) = request_with_updates(
        &mut h,
        RenownRequest::SetWatchedFaction {
            faction_id: faction.faction_id,
        },
    );
    assert_eq!(reply.watched_faction, Some(faction.faction_id));
    assert_eq!(reply.watch_error, None);
    let watched = updates
        .into_iter()
        .find_map(|update| update.snapshot.and_then(|snapshot| snapshot.watched))
        .expect("accepted watch must publish its main-screen bar");
    assert_eq!(
        (
            watched.faction_id,
            watched.name.as_str(),
            watched.value,
            watched.maximum,
            watched.is_major_faction
        ),
        (
            faction.faction_id,
            faction.name.as_str(),
            0,
            faction.level_threshold,
            true
        )
    );
    let character = h.character_id;
    drop(h);
    let reopened = Database::open(file.path()).unwrap();
    let stored = crate::persistence::load_reputation(&reopened, character, || {
        panic!("accepted watch must persist")
    });
    assert_eq!(stored.watched_faction(), Some(faction.faction_id));
}

#[test]
fn renown_watch_evaluated_eligibility_replaces_the_single_watched_faction() {
    use shared::reputation::CharacterReputation;
    let mut reputation = CharacterReputation::default();
    reputation.watch(72);
    // Explicit evaluator input; no claim that production Maruuk's gate is evaluated.
    set_watched_faction(&mut reputation, 2503, Some(&RenownUnlock::Ungated)).unwrap();
    assert_eq!(reputation.watched_faction(), Some(2503));
    assert!(reputation.is_tracked(2503));
    set_watched_faction(&mut reputation, 2570, Some(&RenownUnlock::Ungated)).unwrap();
    assert_eq!(reputation.watched_faction(), Some(2570));
}

#[test]
fn renown_watch_unknown_or_unevaluated_faction_does_not_replace_watch() {
    use shared::reputation::CharacterReputation;
    let mut h = crate::bank_test_harness::harness().unwrap();
    let mut reputation = CharacterReputation::default();
    reputation.watch(72);
    h.app.world_mut().entity_mut(h.player).insert(reputation);
    let baseline = query(&mut h);
    for faction_id in [2503, 4_000_000] {
        let (denied, updates) =
            request_with_updates(&mut h, RenownRequest::SetWatchedFaction { faction_id });
        assert!(
            updates.iter().any(|update| update
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.watched.as_ref())
                .is_some_and(|watched| watched.faction_id == 72)),
            "Refusal must retain the previous main-screen bar"
        );
        assert!(denied.watch_error.is_some());
        assert_eq!(denied.watched_faction, Some(72));
        assert_eq!(
            denied.factions, baseline.factions,
            "Refusal must not discard the catalog"
        );
    }
}

#[test]
fn renown_reputation_rolls_into_levels_and_stops_at_currency_max() {
    let faction = maruuk();
    assert_eq!((faction.level_threshold, faction.max_level), (2500, 25));
    let first = gain(Progress::default(), faction, 6000);
    assert_eq!(
        first,
        Progress {
            level: 2,
            standing: 1000
        }
    );
    let maxed = gain(first, faction, 1_000_000);
    assert_eq!(
        maxed,
        Progress {
            level: 25,
            standing: 0
        }
    );
    assert_eq!(
        gain(maxed, faction, 2500),
        maxed,
        "Maximum renown ignores gains"
    );
}

#[test]
fn admin_grant_persists_and_query_reports_sourced_catalog_and_progress() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Renownseeker").unwrap();
    assert!(admin_grant(h.db(), "Nobody", MARUUK, 1).is_err());
    assert!(
        admin_grant(h.db(), "Renownseeker", 2503, 1).is_err(),
        "Faction ID is not a major faction ID"
    );
    assert_eq!(
        admin_grant(h.db(), "Renownseeker", MARUUK, 6000).unwrap(),
        Progress {
            level: 2,
            standing: 1000
        }
    );
    let reply = query(&mut h);
    assert_eq!(reply.factions.len(), super::catalog().len());
    let row = reply.factions.iter().find(|f| f.id == MARUUK).unwrap();
    assert_eq!(
        (
            row.faction_id,
            row.name.as_str(),
            row.level,
            row.standing,
            row.rewards.len()
        ),
        (2503, "Maruuk Centaur", 2, 1000, 43)
    );
    assert_eq!(
        row.unlock,
        RenownUnlock::Unsupported {
            condition_id: 104408,
            description:
                "Complete the quest Maruukai with the centaur caravan in the Ohn'ahran Plains."
                    .into(),
        }
    );
    let untouched = reply.factions.iter().find(|f| f.id == 37).unwrap();
    assert_eq!(
        (untouched.name.as_str(), untouched.level, untouched.standing),
        ("Amani Tribe", 0, 0)
    );
}
