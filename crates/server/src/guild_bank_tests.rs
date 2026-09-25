//! Real-DB tests against data/world.db; each test skips when the DB is absent.
use shared::protocol::*;

use super::DailyReset;
use crate::bank_test_harness::{Harness, LINEN, harness};
use crate::persistence::banks::{
    GUILD_BANK_USAGE, GuildTabRights, load_guild, load_guild_bank_usage, save_guild,
    save_guild_member_rank,
};
use crate::persistence::set_character_guild_by_id;

const GUILD: u32 = 7;
const MEMBER_RANK: u8 = 3;

fn join_guild(h: &Harness, rank: u8) {
    set_character_guild_by_id(h.db(), h.character_id, GUILD, "Bank Testers").unwrap();
    save_guild_member_rank(h.db(), h.character_id, rank);
}

fn contents(h: &mut Harness) -> GuildBankContents {
    h.take::<GuildBankContents>()
        .pop()
        .expect("guild bank contents sent")
}

fn failures(h: &mut Harness) -> Vec<GuildBankError> {
    h.take::<GuildBankFailed>()
        .into_iter()
        .map(|failed| failed.error)
        .collect()
}

fn slot_items(view: &GuildBankTabView) -> Vec<(usize, u32, u32)> {
    view.slots
        .iter()
        .enumerate()
        .filter_map(|(slot, item)| item.as_ref().map(|item| (slot, item.item_id, item.count)))
        .collect()
}

/// Guild Master opens the vault and buys the first tab (Retail 100g).
fn leader_with_tab(h: &mut Harness) -> bevy::prelude::Entity {
    join_guild(h, 0);
    h.set_gold(2_000_000);
    let vault = h.open_vault();
    h.send::<_, GuildBankChannel>(GuildBankBuyTab {
        object: vault.to_bits(),
    });
    vault
}

fn deposit_linen(h: &mut Harness, vault: bevy::prelude::Entity) {
    let linen = h.guid_of(LINEN);
    h.send::<_, GuildBankChannel>(GuildBankDeposit {
        object: vault.to_bits(),
        tab: 0,
        item_guid: linen,
    });
}

fn withdraw_slot(h: &mut Harness, vault: bevy::prelude::Entity, slot: u8) {
    h.send::<_, GuildBankChannel>(GuildBankWithdraw {
        object: vault.to_bits(),
        tab: 0,
        slot,
    });
}

#[test]
fn using_the_vault_opens_the_guild_bank_role() {
    let Some(mut h) = harness() else { return };
    join_guild(&h, 0);
    let vault = h.open_vault();
    let opened = h.take::<InteractionOpened>();
    assert_eq!(
        opened,
        vec![InteractionOpened {
            npc: vault.to_bits(),
            kind: InteractionKind::Role(NpcRole::GuildBanker),
        }]
    );
    let bank = contents(&mut h);
    assert_eq!(bank.guild_name, "Bank Testers");
    assert!(bank.tabs.is_empty());
    assert!(bank.is_leader);
    // Retail BankTab: first guild tab 100g.
    assert_eq!(bank.next_tab_cost, Some(1_000_000));
}

#[test]
fn players_without_a_guild_are_refused() {
    let Some(mut h) = harness() else { return };
    h.open_vault();
    assert_eq!(failures(&mut h), vec![GuildBankError::NotInGuild]);
}

#[test]
fn walking_away_from_the_vault_closes_it() {
    let Some(mut h) = harness() else { return };
    join_guild(&h, 0);
    let vault = h.open_vault();
    // Vault box reaches x = 3 - 0.70 - 5 = -2.7 yd.
    h.move_player(-2.0);
    assert!(h.take::<InteractionClosed>().is_empty());
    h.move_player(-4.0);
    assert_eq!(
        h.take::<InteractionClosed>(),
        vec![InteractionClosed {
            npc: vault.to_bits()
        }]
    );
}

#[test]
fn guild_master_deposit_and_withdraw_are_logged() {
    let Some(mut h) = harness() else { return };
    let vault = leader_with_tab(&mut h);
    assert_eq!(h.gold(), 1_000_000);
    deposit_linen(&mut h, vault);
    let bank = contents(&mut h);
    assert_eq!(slot_items(&bank.tabs[0]), vec![(0, LINEN, 20)]);
    assert_eq!(h.count_of(LINEN), 0);
    withdraw_slot(&mut h, vault, 0);
    assert!(slot_items(&contents(&mut h).tabs[0]).is_empty());
    assert_eq!(h.count_of(LINEN), 20);

    h.send::<_, GuildBankChannel>(GuildBankQueryLog {
        object: vault.to_bits(),
        tab: Some(0),
    });
    let log = h.take::<GuildBankLog>().pop().unwrap();
    let entries: Vec<_> = log
        .entries
        .iter()
        .map(|e| (e.kind, e.actor.as_str(), e.item_id, e.count))
        .collect();
    assert_eq!(
        entries,
        vec![
            (GuildBankLogKind::DepositItem, "Banker", LINEN, 20),
            (GuildBankLogKind::WithdrawItem, "Banker", LINEN, 20),
        ]
    );
    h.send::<_, GuildBankChannel>(GuildBankQueryLog {
        object: vault.to_bits(),
        tab: None,
    });
    let money = h.take::<GuildBankLog>().pop().unwrap();
    assert_eq!(money.entries.len(), 1);
    assert_eq!(money.entries[0].kind, GuildBankLogKind::BuyTab);
    assert_eq!(money.entries[0].copper, 1_000_000);
}

#[test]
fn members_follow_their_rank_tab_rights_and_daily_stacks() {
    let Some(mut h) = harness() else { return };
    let vault = leader_with_tab(&mut h);
    // Two Linen stacks in tab 1.
    let mut guild = load_guild(h.db(), GUILD);
    for (slot, guid) in [(0, 99_001), (1, 99_002)] {
        guild.tabs[0].slots[slot] = Some(crate::persistence::InventoryItemData {
            item_guid: guid,
            item_id: LINEN,
            name: "Linen Cloth".into(),
            quality: 1,
            required_level: 0,
            stack_count: 5,
            vendor_sell_price: 13,
            unique_auction: false,
        });
    }
    save_guild(h.db(), GUILD, &guild);

    save_guild_member_rank(h.db(), h.character_id, MEMBER_RANK);
    h.take::<GuildBankContents>();
    withdraw_slot(&mut h, vault, 0);
    assert_eq!(failures(&mut h), vec![GuildBankError::Permissions]);

    let mut guild = load_guild(h.db(), GUILD);
    guild.ranks[usize::from(MEMBER_RANK)].tabs[0] = GuildTabRights {
        view: true,
        deposit: false,
        withdrawals_per_day: 1,
    };
    save_guild(h.db(), GUILD, &guild);
    withdraw_slot(&mut h, vault, 1);
    let bank = contents(&mut h);
    assert_eq!(bank.tabs[0].remaining_withdrawals, Some(0));
    assert!(!bank.tabs[0].can_deposit);
    assert!(!bank.is_leader);
    withdraw_slot(&mut h, vault, 0);
    deposit_linen(&mut h, vault);
    assert_eq!(
        failures(&mut h),
        vec![GuildBankError::WithdrawLimit, GuildBankError::Permissions]
    );

    // Yesterday's usage does not count.
    let mut usage = load_guild_bank_usage(h.db(), h.character_id);
    usage.day -= 1;
    let txn = h.db().begin_write().unwrap();
    txn.open_table(GUILD_BANK_USAGE)
        .unwrap()
        .insert(h.character_id, bitcode::encode(&usage).as_slice())
        .unwrap();
    txn.commit().unwrap();
    withdraw_slot(&mut h, vault, 0);
    assert!(failures(&mut h).is_empty());
}

#[test]
fn gold_withdrawals_follow_the_rank_allowance() {
    let Some(mut h) = harness() else { return };
    let vault = leader_with_tab(&mut h);
    let transfer = |copper, deposit| GuildBankMoneyTransfer {
        object: vault.to_bits(),
        copper,
        deposit,
    };
    h.send::<_, GuildBankChannel>(transfer(50_000, true));
    assert_eq!(contents(&mut h).money, 50_000);
    assert_eq!(h.gold(), 950_000);

    save_guild_member_rank(h.db(), h.character_id, MEMBER_RANK);
    h.send::<_, GuildBankChannel>(transfer(1, false));
    let mut guild = load_guild(h.db(), GUILD);
    let rank = &mut guild.ranks[usize::from(MEMBER_RANK)];
    rank.withdraw_gold = true;
    rank.gold_per_day = 10_000;
    save_guild(h.db(), GUILD, &guild);
    h.send::<_, GuildBankChannel>(transfer(10_000, false));
    let bank = contents(&mut h);
    assert_eq!(bank.money, 40_000);
    assert_eq!(bank.withdraw_money_remaining, Some(0));
    assert_eq!(h.gold(), 960_000);
    h.send::<_, GuildBankChannel>(transfer(1, false));
    save_guild_member_rank(h.db(), h.character_id, 0);
    h.send::<_, GuildBankChannel>(transfer(40_001, false));
    assert_eq!(
        failures(&mut h),
        vec![
            GuildBankError::WithdrawLimit,
            GuildBankError::WithdrawLimit,
            GuildBankError::NotEnoughGuildMoney
        ]
    );
}

#[test]
fn soulbound_items_stay_out_of_the_guild_bank() {
    let Some(mut h) = harness() else { return };
    let vault = leader_with_tab(&mut h);
    h.make_soulbound(h.guid_of(LINEN));
    deposit_linen(&mut h, vault);
    assert_eq!(failures(&mut h), vec![GuildBankError::BoundItem]);
    assert_eq!(h.count_of(LINEN), 20);
}

#[test]
fn only_the_guild_master_buys_tabs_and_edits_tab_info() {
    let Some(mut h) = harness() else { return };
    let vault = leader_with_tab(&mut h);
    h.send::<_, GuildBankChannel>(GuildBankSetTabText {
        object: vault.to_bits(),
        tab: 0,
        text: "Cloth for the tailors".into(),
    });
    assert_eq!(contents(&mut h).tabs[0].text, "Cloth for the tailors");
    save_guild_member_rank(h.db(), h.character_id, MEMBER_RANK);
    h.send::<_, GuildBankChannel>(GuildBankBuyTab {
        object: vault.to_bits(),
    });
    h.send::<_, GuildBankChannel>(GuildBankSetTabInfo {
        object: vault.to_bits(),
        tab: 0,
        name: "Mine".into(),
        icon: 0,
    });
    assert_eq!(
        failures(&mut h),
        vec![GuildBankError::Permissions, GuildBankError::Permissions]
    );
}

#[test]
fn daily_reset_starts_the_day_at_the_reset_hour() {
    let reset = DailyReset { hour_utc: 15 };
    // 2026-09-24 14:59:59 UTC and 15:00:00 UTC.
    let before = 1_790_261_999;
    assert_eq!(reset.day(before + 1), reset.day(before) + 1);
    assert_eq!(reset.day(before + 86_400), reset.day(before + 1));
}

#[test]
fn admin_sets_member_ranks_and_rank_bank_rights() {
    use crate::admin_handlers::{handle_set_guild_bank_rights, handle_set_guild_rank};
    use crate::admin_types::AdminResponse;
    let Some(mut h) = harness() else { return };
    join_guild(&h, 4);
    let db = h.app.world().resource::<crate::persistence::GameDatabase>();
    assert!(matches!(
        handle_set_guild_rank(db, "Banker", 3),
        AdminResponse::GuildRankSet { rank: 3, ref rank_name, .. } if rank_name == "Member"
    ));
    assert!(matches!(
        handle_set_guild_rank(db, "Banker", 5),
        AdminResponse::Error(_)
    ));
    let set_tab = |rank| {
        handle_set_guild_bank_rights(db, GUILD, rank, |data| {
            data.tabs[0] = GuildTabRights {
                view: true,
                deposit: true,
                withdrawals_per_day: 2,
            };
            Ok(())
        })
    };
    assert!(matches!(set_tab(0), AdminResponse::Error(_)));
    assert!(matches!(
        set_tab(3),
        AdminResponse::GuildBankRightsSet { rank: 3, .. }
    ));
    let guild = load_guild(h.db(), GUILD);
    assert_eq!(guild.ranks[3].tabs[0].withdrawals_per_day, 2);
    assert_eq!(
        crate::persistence::banks::load_guild_member_rank(h.db(), &guild, h.character_id),
        3
    );
    h.take::<GuildBankContents>();
}
