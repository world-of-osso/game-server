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
    h.app.add_systems(Update, receive_requests);
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<RenownRequest>::default(),
        MessageReceiver::<RenownRequest>::default(),
        MessageSender::<RenownSnapshot>::default(),
        MessageReceiver::<RenownSnapshot>::default(),
    ));
    h.app
        .world_mut()
        .get_mut::<MessageSender<RenownRequest>>(h.connection)
        .unwrap()
        .send::<InstanceChannel>(RenownRequest::Query);
    let mut replies = Vec::new();
    for _ in 0..10 {
        h.app.update();
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<RenownSnapshot>>(h.connection)
                .unwrap()
                .receive(),
        );
    }
    replies.pop().expect("renown reply")
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
