use super::*;
use crate::bank_test_harness::harness_at;
use crate::owned_commerce_source_tests::{Catalog, assert_owned, assert_stacks};
use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};

#[test]
fn owned_commerce_sources_guild_bank_transfers_mixed_collision_stacks_after_restart() {
    let catalog = Catalog::load();
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = harness_at(file.path(), "Depositor").expect("real bank fixture");
    h.app.insert_resource(catalog.templates.clone());
    let depositor = h.character_id;
    let guids = catalog.seed(
        h.db(),
        depositor,
        &[(Retail, 2512, 7), (Forever70205, 2512, 11)],
    );
    let expected = [
        (guids[0], 2512, Retail, 7),
        (guids[1], 2512, Forever70205, 11),
    ];
    let vault = leader_with_tab(&mut h);
    h.take::<GuildBankContents>();
    for &guid in &guids {
        h.send::<_, GuildBankChannel>(GuildBankDeposit {
            object: vault.to_bits(),
            tab: 0,
            item_guid: guid,
        });
    }
    assert!(failures(&mut h).is_empty());
    assert_owned(h.db(), &catalog, depositor, &[]);
    let bank = contents(&mut h);
    assert_stacks(bank.tabs[0].slots.iter().flatten().cloned(), &expected);
    drop(h);

    // New app, new connection and another owner: no in-memory source map survives.
    let mut h = harness_at(file.path(), "Withdrawer").expect("reopened bank fixture");
    h.app.insert_resource(catalog.templates.clone());
    let recipient = h.character_id;
    assert_ne!(recipient, depositor);
    catalog.seed(h.db(), recipient, &[]);
    join_guild(&h, MEMBER_RANK);
    let mut guild = load_guild(h.db(), GUILD);
    guild.ranks[usize::from(MEMBER_RANK)].tabs[0] = GuildTabRights {
        view: true,
        deposit: true,
        withdrawals_per_day: 2,
    };
    save_guild(h.db(), GUILD, &guild);
    let vault = h.open_vault();
    let bank = contents(&mut h);
    assert_stacks(bank.tabs[0].slots.iter().flatten().cloned(), &expected);
    withdraw_slot(&mut h, vault, 0);
    withdraw_slot(&mut h, vault, 1);
    assert!(failures(&mut h).is_empty());
    assert_owned(h.db(), &catalog, recipient, &expected);
    assert!(contents(&mut h).tabs[0].slots.iter().all(Option::is_none));
    assert_eq!(
        load_guild_bank_usage(h.db(), recipient).tab_withdrawals[0],
        2
    );
    drop(h);

    let db = redb::Database::open(file.path()).unwrap();
    assert_owned(&db, &catalog, recipient, &expected);
    assert_owned(&db, &catalog, depositor, &[]);
    assert!(
        load_guild(&db, GUILD).tabs[0]
            .slots
            .iter()
            .all(Option::is_none)
    );
    let logs = crate::persistence::banks::load_guild_bank_logs(&db, GUILD);
    assert_eq!(
        logs.tabs[0]
            .iter()
            .filter(|e| e.kind == crate::persistence::banks::GuildBankLogKindData::WithdrawItem)
            .count(),
        2
    );
}
