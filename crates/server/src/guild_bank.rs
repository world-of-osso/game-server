//! Guild bank (docs/wiki/systems/banks.md) for the Guild Vault whose frame
//! `npc_interaction` has open.
//!
//! Rules follow AzerothCore `Guild` (Guilds/Guild.cpp): per-rank tab rights
//! (`GUILD_BANK_RIGHT_VIEW_TAB` / `DEPOSIT_ITEM`) and stacks per day
//! (`_GetMemberRemainingSlots`), the gold allowance of ranks with
//! `GR_RIGHT_WITHDRAW_GOLD` (`_GetMemberRemainingMoney`), the Guild Master's
//! unlimited rights, tab purchase by the Guild Master (`HandleBuyBankTab`, Retail
//! `BankTab` prices, `MAX_BUY_GUILDBANK_TABS` = 6), soulbound and quest items
//! refused (`Guild::_DoItemsMove`), and a log per tab plus a money log of
//! `GUILD_BANKLOG_MAX_RECORDS` entries. Daily counts reset at the configured hour
//! (`DailyReset`). Every change is sent to each member with the vault open.
//!
//! Guild repairs (TrinityCore `GR_RIGHT_WITHDRAW_REPAIR`) spend the same daily
//! allowance; `merchant` takes them through `guild_repair_money` / `pay_guild_repair`.
//!
//! Gaps: no item moves inside the bank, no partial stack withdrawals, no Guild
//! Control UI (ranks and rights are set with game-server-admin).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use redb::Database;
use shared::components::GuildMembership;
use shared::protocol::{
    GUILD_BANK_MAX_TABS, GuildBankBuyTab, GuildBankChannel, GuildBankContents, GuildBankDeposit,
    GuildBankError, GuildBankFailed, GuildBankLog, GuildBankLogEntry, GuildBankLogKind,
    GuildBankMoneyTransfer, GuildBankQueryLog, GuildBankSetTabInfo, GuildBankSetTabText,
    GuildBankTabView, GuildBankWithdraw, NpcRole,
};

use crate::auth::InWorld;
use crate::bank::{BankTabPrices, TAB_NAME_LETTERS, new_tab};
use crate::bank_storage::{self, TabsFull};
use crate::gold::{GoldUpdateQueue, MAX_MONEY};
use crate::inventory::{inventory_model::InventoryModel, load_model};
use crate::item_templates::{BIND_QUEST, ItemTemplate, ItemTemplates};
use crate::npc_interaction::ActiveInteraction;
use crate::persistence::banks::{
    BankOwner, GUILD_MASTER_RANK, GuildBankLogData, GuildBankLogKindData, GuildBankLogs,
    GuildBankUsage, GuildData, GuildRankData, load_guild, load_guild_bank_logs,
    load_guild_bank_usage, load_guild_member_rank, save_bank_move,
};
use crate::persistence::{self, GameDatabase};

/// AzerothCore `GUILD_BANK_MONEY_LIMIT` (Guild.h:54).
const GUILD_BANK_MONEY_LIMIT: u64 = 0x7_FFFF_FFFF_FFFF;
/// Retail tab info text length (`GuildBankTabInfoEditBox` letters="500").
const TAB_TEXT_LETTERS: usize = 500;

/// Hour (UTC) at which daily guild bank allowances reset; Retail's daily reset.
/// `GAME_SERVER_DAILY_RESET_HOUR_UTC` overrides the default 15 (Retail US, 15:00 UTC).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyReset {
    pub hour_utc: u8,
}

impl Default for DailyReset {
    fn default() -> Self {
        let hour_utc = std::env::var("GAME_SERVER_DAILY_RESET_HOUR_UTC")
            .ok()
            .map(|value| {
                value
                    .parse::<u8>()
                    .ok()
                    .filter(|hour| *hour < 24)
                    .unwrap_or_else(|| {
                        panic!("GAME_SERVER_DAILY_RESET_HOUR_UTC must be 0-23, got {value}")
                    })
            })
            .unwrap_or(15);
        Self { hour_utc }
    }
}

impl DailyReset {
    /// Days since the epoch, counted from the reset hour.
    pub fn day(self, unix_secs: u64) -> i64 {
        (unix_secs as i64 - i64::from(self.hour_utc) * 3600).div_euclid(86_400)
    }
}

pub(crate) fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

pub struct GuildBankPlugin;

impl Plugin for GuildBankPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DailyReset>()
            .init_resource::<ChangedGuilds>()
            .init_resource::<PendingGuildBankCommits>()
            .add_systems(
                Update,
                (
                    send_opened_guild_banks,
                    process_deposits,
                    process_withdrawals,
                    process_money_transfers,
                    process_tab_purchases,
                    process_tab_info,
                    process_tab_text,
                    process_log_queries,
                    publish_guild_bank_commits,
                    broadcast_changed_guild_banks,
                )
                    .chain(),
            );
    }
}

mod actor;
mod transactions;
pub(crate) use actor::PendingGuildBankCommits;
use actor::publish_guild_bank_commits;

type GuildBankResult = Result<(), GuildBankError>;

/// Guilds whose bank changed this frame.
#[derive(Resource, Default)]
pub(crate) struct ChangedGuilds(pub(crate) Vec<u32>);

/// A guild member at the vault, with everything their rank allows.
struct Member {
    connection: Entity,
    character_id: u64,
    name: String,
    guild_id: u32,
    rank: u8,
    vault: Option<Entity>,
}

impl Member {
    fn vault_for(&self, object: u64) -> Result<Entity, GuildBankError> {
        self.vault
            .filter(|vault| vault.to_bits() == object)
            .ok_or(GuildBankError::NotInteracting)
    }

    fn is_leader(&self) -> bool {
        self.rank == GUILD_MASTER_RANK
    }
}

/// Rights of `rank` on `tab`: view, deposit, stacks per day (`None` unlimited).
fn tab_rights(guild: &GuildData, rank: u8, tab: usize) -> (bool, bool, Option<u32>) {
    if rank == GUILD_MASTER_RANK {
        return (true, true, None);
    }
    let rights = guild
        .ranks
        .get(usize::from(rank))
        .and_then(|rank| rank.tabs.get(tab))
        .copied()
        .unwrap_or_default();
    (
        rights.view,
        rights.view && rights.deposit,
        Some(if rights.view {
            rights.withdrawals_per_day
        } else {
            0
        }),
    )
}

/// Whether `rank` holds a right; the Guild Master holds every one.
fn has_right(guild: &GuildData, rank: u8, right: fn(&GuildRankData) -> bool) -> bool {
    rank == GUILD_MASTER_RANK || guild.ranks.get(usize::from(rank)).is_some_and(right)
}

/// Copper per day `rank` may take from the guild money (`None` unlimited): TrinityCore
/// `_GetMemberRemainingMoney` (Guild.cpp:3358-3371) gives a rank with
/// `GR_RIGHT_WITHDRAW_REPAIR` or `GR_RIGHT_WITHDRAW_GOLD` its `BankMoneyPerDay`, one
/// allowance for withdrawals and repairs.
fn money_allowance(guild: &GuildData, rank: u8) -> Option<u64> {
    if rank == GUILD_MASTER_RANK {
        return None;
    }
    Some(
        guild
            .ranks
            .get(usize::from(rank))
            .filter(|rank| rank.withdraw_gold || rank.withdraw_repair)
            .map_or(0, |rank| rank.gold_per_day),
    )
}

/// Copper per day `rank` may withdraw as gold (`None` unlimited).
fn gold_allowance(guild: &GuildData, rank: u8) -> Option<u64> {
    if has_right(guild, rank, |rank| rank.withdraw_gold) {
        money_allowance(guild, rank)
    } else {
        Some(0)
    }
}

/// The guild money `character_id` may spend on repairs now: TrinityCore
/// `GetMemberAvailableMoneyForRepairItems` (Guild.cpp:3040-3047), the guild money capped by
/// the allowance left today. `None` outside a guild or for a rank without
/// `GR_RIGHT_WITHDRAW_REPAIR` (Retail `CanGuildBankRepair`).
pub(crate) fn guild_repair_money(db: &Database, character_id: u64, day: i64) -> Option<u64> {
    let (member, _) = member(db, Entity::PLACEHOLDER, character_id, None)?;
    let guild = load_guild(db, member.guild_id);
    if !has_right(&guild, member.rank, |rank| rank.withdraw_repair) {
        return None;
    }
    let used = usage_today(db, &member, day).copper_withdrawn;
    Some(match money_allowance(&guild, member.rank) {
        None => guild.money,
        Some(limit) => limit.saturating_sub(used).min(guild.money),
    })
}

/// Take `copper` of repairs from the guild money: TrinityCore
/// `HandleMemberWithdrawMoney(repair = true)` (Guild.cpp:2080-2124) counts it against the
/// member's daily allowance and logs `GUILD_BANK_LOG_REPAIR_MONEY` in the money log.
/// The caller sized `copper` to `guild_repair_money`. Returns the guild id.
pub(crate) fn pay_guild_repair(db: &Database, character_id: u64, day: i64, copper: u64) -> u32 {
    let (member, _) = member(db, Entity::PLACEHOLDER, character_id, None)
        .expect("guild repair paid by a guild member");
    let mut guild = load_guild(db, member.guild_id);
    guild.money = guild
        .money
        .checked_sub(copper)
        .expect("guild repair within the guild money");
    let mut usage = usage_today(db, &member, day);
    usage.copper_withdrawn += copper;
    let mut logs = load_guild_bank_logs(db, member.guild_id);
    logs.push(
        None,
        GuildBankLogData {
            kind: GuildBankLogKindData::RepairMoney,
            actor: member.name.clone(),
            item_id: 0,
            item_name: String::new(),
            count: 0,
            copper,
            time: unix_now(),
        },
    );
    let owner = BankOwner::Guild {
        guild_id: member.guild_id,
        guild: &guild,
        logs: &logs,
        usage: Some(&usage),
    };
    save_bank_move(db, character_id, owner, None);
    member.guild_id
}

/// Today's usage of `member`, reset when the day or the guild changed.
fn usage_today(db: &Database, member: &Member, day: i64) -> GuildBankUsage {
    let usage = load_guild_bank_usage(db, member.character_id);
    if usage.guild_id == member.guild_id && usage.day == day {
        return usage;
    }
    GuildBankUsage {
        guild_id: member.guild_id,
        day,
        tab_withdrawals: vec![0; GUILD_BANK_MAX_TABS],
        copper_withdrawn: 0,
    }
}

#[derive(SystemParam)]
struct GuildVault<'w, 's> {
    db: Option<Res<'w, GameDatabase>>,
    templates: Option<Res<'w, ItemTemplates>>,
    prices: Res<'w, BankTabPrices>,
    reset: Res<'w, DailyReset>,
    gold: ResMut<'w, GoldUpdateQueue>,
    changed: ResMut<'w, ChangedGuilds>,
    contents: Query<'w, 's, &'static mut MessageSender<GuildBankContents>>,
    logs: Query<'w, 's, &'static mut MessageSender<GuildBankLog>>,
    failures: Query<'w, 's, &'static mut MessageSender<GuildBankFailed>>,
}

impl GuildVault<'_, '_> {
    fn ready(&self) -> bool {
        self.db.is_some() && self.templates.is_some()
    }

    fn db(&self) -> &Database {
        &self.db.as_ref().expect("checked by ready").db
    }

    fn today(&self) -> i64 {
        self.reset.day(unix_now())
    }

    fn next_tab_cost(&self, guild: &GuildData) -> Option<u64> {
        self.prices.guild.get(guild.tabs.len()).copied()
    }

    fn contents(&self, member: &Member, vault: Entity, guild_name: &str) -> GuildBankContents {
        let db = self.db();
        let guild = load_guild(db, member.guild_id);
        let usage = usage_today(db, member, self.today());
        let guids = guild.tabs.iter().flat_map(|tab| tab.slots.iter().flatten());
        let bound =
            persistence::inventory_store::soulbound_guids(db, guids.map(|item| item.item_guid));
        let tabs = guild
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                let (viewable, can_deposit, per_day) = tab_rights(&guild, member.rank, index);
                let used = usage.tab_withdrawals.get(index).copied().unwrap_or(0);
                GuildBankTabView {
                    name: tab.name.clone(),
                    icon: tab.icon,
                    viewable,
                    can_deposit,
                    withdrawals_per_day: per_day,
                    remaining_withdrawals: per_day.map(|limit| limit.saturating_sub(used)),
                    text: if viewable {
                        tab.text.clone()
                    } else {
                        String::new()
                    },
                    slots: if viewable {
                        bank_storage::tab_slots(
                            tab,
                            |guid| bound.contains(&guid),
                            |guid| {
                                persistence::inventory_store::load_item_definition_source(
                                    self.db(),
                                    guid,
                                )
                            },
                        )
                    } else {
                        Vec::new()
                    },
                }
            })
            .collect();
        let remaining_money = gold_allowance(&guild, member.rank).map(|limit| {
            limit
                .saturating_sub(usage.copper_withdrawn)
                .min(guild.money)
        });
        GuildBankContents {
            object: vault.to_bits(),
            guild_name: guild_name.to_owned(),
            tabs,
            money: guild.money,
            withdraw_money_remaining: remaining_money,
            next_tab_cost: member
                .is_leader()
                .then(|| self.next_tab_cost(&guild))
                .flatten(),
            is_leader: member.is_leader(),
        }
    }

    fn send_contents(&mut self, member: &Member, vault: Entity, guild_name: &str) {
        let contents = self.contents(member, vault, guild_name);
        if let Ok(mut sender) = self.contents.get_mut(member.connection) {
            sender.send::<GuildBankChannel>(contents);
        }
    }

    fn fail(&mut self, connection: Entity, object: u64, error: GuildBankError) {
        if let Ok(mut sender) = self.failures.get_mut(connection) {
            sender.send::<GuildBankChannel>(GuildBankFailed { object, error });
        }
    }
}

type Requests<'w, 's, M> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        Option<&'static ActiveInteraction>,
        &'static mut MessageReceiver<M>,
    ),
>;

fn open_vault(interaction: Option<&ActiveInteraction>) -> Option<Entity> {
    interaction
        .filter(|interaction| interaction.role == Some(NpcRole::GuildBanker))
        .map(|interaction| interaction.npc)
}

/// The guild member behind a connection; `None` when the character has no guild.
fn member(
    db: &Database,
    connection: Entity,
    character_id: u64,
    vault: Option<Entity>,
) -> Option<(Member, GuildMembership)> {
    let guild = persistence::load_character_guild(db, character_id)?;
    let data = load_guild(db, guild.guild_id);
    let name = persistence::load_character(db, character_id).map_or_else(String::new, |c| c.name);
    Some((
        Member {
            connection,
            character_id,
            name,
            guild_id: guild.guild_id,
            rank: load_guild_member_rank(db, &data, character_id),
            vault,
        },
        guild,
    ))
}

/// Enqueue owned requests; membership and daily allowances load in FIFO order.
fn process<M: lightyear::prelude::Message>(
    requests: &mut Requests<M>,
    vault: &mut GuildVault,
    pending: &mut PendingGuildBankCommits,
    object_of: impl Fn(&M) -> u64,
    handle: impl Fn(&mut transactions::Transaction, &Member, M) -> GuildBankResult
    + Copy
    + Send
    + 'static,
) {
    if !vault.ready() {
        return;
    }
    for (connection, in_world, interaction, mut receiver) in requests.iter_mut() {
        let open = open_vault(interaction);
        let character = in_world.character_id;
        for request in receiver.receive() {
            let object = object_of(&request);
            let templates = vault.templates.as_deref().expect("ready").clone();
            let prices = vault.prices.clone();
            let reset = *vault.reset;
            let receipt = vault.db.as_ref().expect("ready").writer.enqueue(
                "guild bank transaction",
                move |db| {
                    Ok(transactions::execute(
                        db, templates, prices, reset, connection, character, open, request, handle,
                    ))
                },
            );
            pending.push(connection, character, object, receipt);
        }
    }
}

/// Opening the vault sends the bank (Retail `GUILDBANKFRAME_OPENED`); a player
/// without a guild gets `ERR_GUILD_PLAYER_NOT_IN_GUILD`.
fn send_opened_guild_banks(
    opened: Query<(Entity, &InWorld, &ActiveInteraction), Changed<ActiveInteraction>>,
    mut vault: GuildVault,
) {
    if !vault.ready() {
        return;
    }
    for (connection, in_world, interaction) in &opened {
        let Some(object) = open_vault(Some(interaction)) else {
            continue;
        };
        match member(vault.db(), connection, in_world.character_id, Some(object)) {
            Some((member, guild)) => vault.send_contents(&member, object, &guild.guild_name),
            None => vault.fail(connection, object.to_bits(), GuildBankError::NotInGuild),
        }
    }
}

/// Send the bank again to every member of a changed guild with its vault open.
fn broadcast_changed_guild_banks(
    open: Query<(Entity, &InWorld, &ActiveInteraction)>,
    mut vault: GuildVault,
) {
    let changed = std::mem::take(&mut vault.changed.0);
    if changed.is_empty() || !vault.ready() {
        return;
    }
    for (connection, in_world, interaction) in &open {
        let Some(object) = open_vault(Some(interaction)) else {
            continue;
        };
        let Some((member, guild)) =
            member(vault.db(), connection, in_world.character_id, Some(object))
        else {
            continue;
        };
        if changed.contains(&member.guild_id) {
            vault.send_contents(&member, object, &guild.guild_name);
        }
    }
}

fn process_deposits(
    mut requests: Requests<GuildBankDeposit>,
    mut pending: ResMut<PendingGuildBankCommits>,
    mut vault: GuildVault,
) {
    process(
        &mut requests,
        &mut vault,
        &mut pending,
        |req| req.object,
        transactions::deposit,
    );
}

fn process_withdrawals(
    mut requests: Requests<GuildBankWithdraw>,
    mut pending: ResMut<PendingGuildBankCommits>,
    mut vault: GuildVault,
) {
    process(
        &mut requests,
        &mut vault,
        &mut pending,
        |req| req.object,
        transactions::withdraw,
    );
}

fn process_money_transfers(
    mut requests: Requests<GuildBankMoneyTransfer>,
    mut pending: ResMut<PendingGuildBankCommits>,
    mut vault: GuildVault,
) {
    process(
        &mut requests,
        &mut vault,
        &mut pending,
        |req| req.object,
        transactions::transfer_money,
    );
}

fn process_tab_purchases(
    mut requests: Requests<GuildBankBuyTab>,
    mut pending: ResMut<PendingGuildBankCommits>,
    mut vault: GuildVault,
) {
    process(
        &mut requests,
        &mut vault,
        &mut pending,
        |req| req.object,
        transactions::buy_tab,
    );
}

fn process_tab_info(
    mut requests: Requests<GuildBankSetTabInfo>,
    mut pending: ResMut<PendingGuildBankCommits>,
    mut vault: GuildVault,
) {
    process(
        &mut requests,
        &mut vault,
        &mut pending,
        |req| req.object,
        transactions::set_tab_info,
    );
}

fn process_tab_text(
    mut requests: Requests<GuildBankSetTabText>,
    mut pending: ResMut<PendingGuildBankCommits>,
    mut vault: GuildVault,
) {
    process(
        &mut requests,
        &mut vault,
        &mut pending,
        |req| req.object,
        transactions::set_tab_text,
    );
}

fn process_log_queries(
    mut requests: Requests<GuildBankQueryLog>,
    mut pending: ResMut<PendingGuildBankCommits>,
    mut vault: GuildVault,
) {
    process(
        &mut requests,
        &mut vault,
        &mut pending,
        |req| req.object,
        transactions::send_log,
    );
}

#[cfg(test)]
#[path = "guild_bank_tests.rs"]
mod tests;
