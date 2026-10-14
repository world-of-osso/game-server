//! Guild bank repairs (TrinityCore `DurabilityRepairAll(guildBank = true)` and
//! `Guild::HandleMemberWithdrawMoney(repair = true)`) at Godric Rothgar.

use super::*;
use crate::persistence::banks::{
    GuildBankLogKindData, GuildData, load_guild, load_guild_bank_logs, load_guild_bank_usage,
    save_guild, save_guild_member_rank,
};
use crate::persistence::set_character_guild_by_id;

const GUILD: u32 = 7;
const GUILD_MASTER: u8 = 0;
const MEMBER: u8 = 3;

impl Harness {
    fn db(&self) -> &redb::Database {
        &self.app.world().resource::<GameDatabase>().db
    }

    /// Put the shopper in guild 7 at `rank`, with `money` copper in the guild bank.
    fn join_guild(&mut self, rank: u8, money: u64, edit: impl FnOnce(&mut GuildData)) {
        set_character_guild_by_id(self.db(), self.character_id, GUILD, "Repair Crew").unwrap();
        save_guild_member_rank(self.db(), self.character_id, rank);
        let mut guild = load_guild(self.db(), GUILD);
        guild.money = money;
        edit(&mut guild);
        save_guild(self.db(), GUILD, &guild);
    }

    fn guild_repair(&mut self, npc: Entity) {
        self.send::<_, MerchantChannel>(RepairItem {
            npc: npc.to_bits(),
            item_guid: None,
            guild_bank: true,
        });
    }

    /// Kind, actor and copper of each guild money log entry.
    fn money_log(&self) -> Vec<(GuildBankLogKindData, String, u64)> {
        load_guild_bank_logs(self.db(), GUILD)
            .money
            .into_iter()
            .map(|entry| (entry.kind, entry.actor, entry.copper))
            .collect()
    }
}

fn guild_money_offered(h: &mut Harness) -> Option<u64> {
    h.take::<VendorInventory>()
        .pop()
        .expect("vendor list sent")
        .guild_repair_money
}

#[test]
fn guild_master_repairs_from_the_guild_bank_and_the_repair_is_logged() {
    let Some(mut h) = harness() else { return };
    h.join_guild(GUILD_MASTER, 1_000, |_| {});
    damage_gear(&mut h);
    let godric = h.open(GODRIC_ROTHGAR);
    // The Guild Master may spend the whole guild bank (`GUILD_WITHDRAW_MONEY_UNLIMITED`).
    assert_eq!(guild_money_offered(&mut h), Some(1_000));

    h.guild_repair(godric);

    assert!(h.failures().is_empty());
    assert_eq!(durability(&h), vec![(55, 55), (35, 35)]);
    // 8 + 8 copper from the guild; the shopper's own money is untouched.
    assert_eq!(load_guild(h.db(), GUILD).money, 984);
    assert_eq!(h.inventory().gold, 100_000);
    assert_eq!(
        h.money_log(),
        vec![(GuildBankLogKindData::RepairMoney, "Shopper".into(), 16)]
    );
    // The vendor list comes again with what is left to spend.
    assert_eq!(guild_money_offered(&mut h), Some(984));
}

#[test]
fn ranks_without_the_repair_right_are_refused() {
    let Some(mut h) = harness() else { return };
    // TrinityCore's default Member rank holds chat rights only, even with a gold allowance.
    h.join_guild(MEMBER, 1_000, |guild| {
        let rank = &mut guild.ranks[usize::from(MEMBER)];
        rank.withdraw_gold = true;
        rank.gold_per_day = 500;
    });
    damage_gear(&mut h);
    let godric = h.open(GODRIC_ROTHGAR);
    assert_eq!(guild_money_offered(&mut h), None);

    h.guild_repair(godric);

    assert_eq!(h.failures(), vec![MerchantError::GuildPermissions]);
    assert_eq!(durability(&h), vec![(44, 55), (25, 35)]);
    assert_eq!(load_guild(h.db(), GUILD).money, 1_000);
    assert!(h.money_log().is_empty());
}

#[test]
fn a_player_without_a_guild_is_refused() {
    let Some(mut h) = harness() else { return };
    damage_gear(&mut h);
    let godric = h.open(GODRIC_ROTHGAR);
    assert_eq!(guild_money_offered(&mut h), None);

    h.guild_repair(godric);

    assert_eq!(h.failures(), vec![MerchantError::GuildPermissions]);
    assert_eq!(h.inventory().gold, 100_000);
    assert_eq!(durability(&h), vec![(44, 55), (25, 35)]);
}

#[test]
fn the_daily_allowance_repairs_the_cheapest_items_it_covers() {
    let Some(mut h) = harness() else { return };
    // A Member given the repair right and 10 copper a day.
    h.join_guild(MEMBER, 1_000, |guild| {
        let rank = &mut guild.ranks[usize::from(MEMBER)];
        rank.withdraw_repair = true;
        rank.gold_per_day = 10;
    });
    damage_gear(&mut h);
    let godric = h.open(GODRIC_ROTHGAR);
    assert_eq!(guild_money_offered(&mut h), Some(10));

    h.guild_repair(godric);

    // Both items cost 8: the vest fits the 10 copper, the shield no longer does.
    assert_eq!(h.failures(), vec![MerchantError::GuildNotEnoughMoney]);
    assert_eq!(durability(&h), vec![(55, 55), (25, 35)]);
    assert_eq!(load_guild(h.db(), GUILD).money, 992);
    assert_eq!(h.inventory().gold, 100_000);
    assert_eq!(
        load_guild_bank_usage(h.db(), h.character_id).copper_withdrawn,
        8
    );
    assert_eq!(guild_money_offered(&mut h), Some(2));

    // 2 copper left today covers nothing: no repair, no debit, no log entry.
    h.guild_repair(godric);
    assert_eq!(h.failures(), vec![MerchantError::GuildNotEnoughMoney]);
    assert_eq!(durability(&h), vec![(55, 55), (25, 35)]);
    assert_eq!(load_guild(h.db(), GUILD).money, 992);
    assert_eq!(h.money_log().len(), 1);
}

#[test]
fn a_guild_bank_short_of_the_cheapest_repair_refuses_it() {
    let Some(mut h) = harness() else { return };
    h.join_guild(GUILD_MASTER, 5, |_| {});
    damage_gear(&mut h);
    let godric = h.open(GODRIC_ROTHGAR);
    assert_eq!(guild_money_offered(&mut h), Some(5));

    h.guild_repair(godric);

    assert_eq!(h.failures(), vec![MerchantError::GuildNotEnoughMoney]);
    assert_eq!(durability(&h), vec![(44, 55), (25, 35)]);
    assert_eq!(load_guild(h.db(), GUILD).money, 5);
    assert_eq!(h.inventory().gold, 100_000);
    assert!(h.money_log().is_empty());
}

#[test]
fn repairs_and_gold_withdrawals_share_one_daily_allowance() {
    let Some(mut h) = harness() else { return };
    // An Officer (TrinityCore `GR_RIGHT_ALL`) with 20 copper a day, 15 already withdrawn.
    h.join_guild(1, 1_000, |guild| guild.ranks[1].gold_per_day = 20);
    let day = h.app.world().resource::<DailyReset>().day(unix_now());
    let usage = crate::persistence::banks::GuildBankUsage {
        guild_id: GUILD,
        day,
        tab_withdrawals: vec![0; shared::protocol::GUILD_BANK_MAX_TABS],
        copper_withdrawn: 15,
    };
    let txn = h.db().begin_write().unwrap();
    txn.open_table(crate::persistence::banks::GUILD_BANK_USAGE)
        .unwrap()
        .insert(h.character_id, bitcode::encode(&usage).as_slice())
        .unwrap();
    crate::persistence_commit::commit(txn).unwrap();
    damage_gear(&mut h);
    h.open(GODRIC_ROTHGAR);

    assert_eq!(guild_money_offered(&mut h), Some(5));
}

#[test]
fn repairing_one_item_with_the_guild_flag_pays_personally() {
    // TrinityCore `HandleRepairItemOpcode` uses the guild bank only to repair all.
    let Some(mut h) = harness() else { return };
    h.join_guild(GUILD_MASTER, 1_000, |_| {});
    let vest = equip(&mut h, TARNISHED_CHAIN_VEST);
    damage_gear(&mut h);
    let godric = h.open(GODRIC_ROTHGAR);

    h.send::<_, MerchantChannel>(RepairItem {
        npc: godric.to_bits(),
        item_guid: Some(vest),
        guild_bank: true,
    });

    assert!(h.failures().is_empty());
    assert_eq!(h.inventory().gold, 100_000 - 8);
    assert_eq!(load_guild(h.db(), GUILD).money, 1_000);
}
