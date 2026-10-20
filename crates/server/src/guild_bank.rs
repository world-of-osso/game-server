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
                    broadcast_changed_guild_banks,
                )
                    .chain(),
            );
    }
}

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

    fn load_template(
        &self,
        item: &crate::persistence::InventoryItemData,
    ) -> Result<(shared::item_data::ItemDefinitionSource, ItemTemplate), GuildBankError> {
        let source =
            persistence::inventory_store::load_item_definition_source(self.db(), item.item_guid);
        let template = self
            .templates
            .as_ref()
            .and_then(|templates| templates.get(source, item.item_id).copied())
            .ok_or(GuildBankError::ItemNotFound)?;
        Ok((source, template))
    }

    fn mint(&self) -> impl FnMut() -> u64 + '_ {
        let db = self.db();
        move || persistence::next_id(db, "item_instance")
    }

    fn load_model(&self, character_id: u64) -> Result<InventoryModel, GuildBankError> {
        let templates = self.templates.as_ref().expect("checked by ready");
        load_model(self.db(), templates, character_id).map_err(|_| GuildBankError::ItemNotFound)
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
                            |item| {
                                crate::inventory::stored_crafting_origin(
                                    self.db(),
                                    self.templates.as_ref().expect("checked by ready"),
                                    item,
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

    /// Write the guild, its logs, the member's usage and (when given) their bags.
    fn save(
        &mut self,
        member: &Member,
        guild: &GuildData,
        logs: &GuildBankLogs,
        usage: Option<&GuildBankUsage>,
        model: Option<InventoryModel>,
    ) {
        let gold = model.as_ref().map(InventoryModel::gold);
        let record = model.map(InventoryModel::into_record);
        let owner = BankOwner::Guild {
            guild_id: member.guild_id,
            guild,
            logs,
            usage,
        };
        save_bank_move(self.db(), member.character_id, owner, record.as_ref());
        if let Some(gold) = gold {
            self.gold.record(member.character_id, gold);
        }
        self.changed.0.push(member.guild_id);
    }

    fn log(&self, member: &Member, kind: GuildBankLogKindData) -> GuildBankLogData {
        GuildBankLogData {
            kind,
            actor: member.name.clone(),
            item_id: 0,
            item_name: String::new(),
            count: 0,
            copper: 0,
            time: unix_now(),
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

/// Run `handle` for each request; a refusal answers `GuildBankFailed`.
fn process<M: lightyear::prelude::Message>(
    requests: &mut Requests<M>,
    vault: &mut GuildVault,
    object_of: impl Fn(&M) -> u64,
    handle: impl Fn(&mut GuildVault, &Member, M) -> GuildBankResult,
) {
    if !vault.ready() {
        return;
    }
    for (connection, in_world, interaction, mut receiver) in requests.iter_mut() {
        let open = open_vault(interaction);
        for request in receiver.receive() {
            let object = object_of(&request);
            let Some((member, _)) = member(vault.db(), connection, in_world.character_id, open)
            else {
                vault.fail(connection, object, GuildBankError::NotInGuild);
                continue;
            };
            if let Err(error) = handle(vault, &member, request) {
                vault.fail(connection, object, error);
            }
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

fn process_deposits(mut requests: Requests<GuildBankDeposit>, mut vault: GuildVault) {
    process(&mut requests, &mut vault, |req| req.object, deposit);
}

fn process_withdrawals(mut requests: Requests<GuildBankWithdraw>, mut vault: GuildVault) {
    process(&mut requests, &mut vault, |req| req.object, withdraw);
}

fn process_money_transfers(mut requests: Requests<GuildBankMoneyTransfer>, mut vault: GuildVault) {
    process(&mut requests, &mut vault, |req| req.object, transfer_money);
}

fn process_tab_purchases(mut requests: Requests<GuildBankBuyTab>, mut vault: GuildVault) {
    process(&mut requests, &mut vault, |req| req.object, buy_tab);
}

fn process_tab_info(mut requests: Requests<GuildBankSetTabInfo>, mut vault: GuildVault) {
    process(&mut requests, &mut vault, |req| req.object, set_tab_info);
}

fn process_tab_text(mut requests: Requests<GuildBankSetTabText>, mut vault: GuildVault) {
    process(&mut requests, &mut vault, |req| req.object, set_tab_text);
}

fn process_log_queries(mut requests: Requests<GuildBankQueryLog>, mut vault: GuildVault) {
    process(&mut requests, &mut vault, |req| req.object, send_log);
}

/// AzerothCore `Guild::SwapItemsWithInventory` (bags to bank): the rank must view and
/// deposit into the tab; soulbound and quest items are refused.
fn deposit(vault: &mut GuildVault, member: &Member, req: GuildBankDeposit) -> GuildBankResult {
    member.vault_for(req.object)?;
    let mut guild = load_guild(vault.db(), member.guild_id);
    let tab = usize::from(req.tab);
    if tab >= guild.tabs.len() {
        return Err(GuildBankError::WrongTab);
    }
    if !tab_rights(&guild, member.rank, tab).1 {
        return Err(GuildBankError::Permissions);
    }
    let mut model = vault.load_model(member.character_id)?;
    let item = model
        .bag_item(req.item_guid)
        .cloned()
        .ok_or(GuildBankError::ItemNotFound)?;
    let (source, template) = vault.load_template(&item)?;
    if template.bonding == BIND_QUEST {
        return Err(GuildBankError::QuestItem);
    }
    if model.is_soulbound(req.item_guid) {
        return Err(GuildBankError::BoundItem);
    }
    let mut sources = bank_storage::load_tab_definition_sources(vault.db(), &guild.tabs);
    sources.insert(item.item_guid, source);
    let mut mint = vault.mint();
    bank_storage::deposit(
        &mut guild.tabs,
        &[tab],
        item.clone(),
        template.max_stack,
        &mut sources,
        &mut mint,
    )
    .map_err(|TabsFull| GuildBankError::TabFull)?;
    model.remember_definition_sources(sources);
    model
        .take_bag_item(req.item_guid, 0, &mut mint)
        .map_err(|_| GuildBankError::ItemNotFound)?;
    drop(mint);
    let mut logs = load_guild_bank_logs(vault.db(), member.guild_id);
    logs.push(
        Some(req.tab),
        GuildBankLogData {
            item_id: item.item_id,
            item_name: item.name.clone(),
            count: item.stack_count,
            ..vault.log(member, GuildBankLogKindData::DepositItem)
        },
    );
    vault.save(member, &guild, &logs, None, Some(model));
    Ok(())
}

/// AzerothCore `Guild::SwapItemsWithInventory` (bank to bags): one stack of the
/// rank's stacks per day (`_UpdateMemberWithdrawSlots`).
fn withdraw(vault: &mut GuildVault, member: &Member, req: GuildBankWithdraw) -> GuildBankResult {
    member.vault_for(req.object)?;
    let mut guild = load_guild(vault.db(), member.guild_id);
    let tab = usize::from(req.tab);
    if tab >= guild.tabs.len() {
        return Err(GuildBankError::WrongTab);
    }
    let (viewable, _, per_day) = tab_rights(&guild, member.rank, tab);
    if !viewable {
        return Err(GuildBankError::Permissions);
    }
    let mut usage = usage_today(vault.db(), member, vault.today());
    if per_day.is_some_and(|limit| usage.tab_withdrawals[tab] >= limit) {
        return Err(GuildBankError::WithdrawLimit);
    }
    let item = bank_storage::take(&mut guild.tabs, req.tab, req.slot)
        .ok_or(GuildBankError::ItemNotFound)?;
    let mut model = vault.load_model(member.character_id)?;
    let (source, template) = vault.load_template(&item)?;
    let mut mint = vault.mint();
    model
        .store_item(item.clone(), source, template, &mut mint)
        .map_err(|_| GuildBankError::InventoryFull)?;
    drop(mint);
    if per_day.is_some() {
        usage.tab_withdrawals[tab] += 1;
    }
    let mut logs = load_guild_bank_logs(vault.db(), member.guild_id);
    logs.push(
        Some(req.tab),
        GuildBankLogData {
            item_id: item.item_id,
            item_name: item.name.clone(),
            count: item.stack_count,
            ..vault.log(member, GuildBankLogKindData::WithdrawItem)
        },
    );
    vault.save(member, &guild, &logs, Some(&usage), Some(model));
    Ok(())
}

/// AzerothCore `HandleMemberDepositMoney` / `HandleMemberWithdrawMoney`.
fn transfer_money(
    vault: &mut GuildVault,
    member: &Member,
    req: GuildBankMoneyTransfer,
) -> GuildBankResult {
    member.vault_for(req.object)?;
    let mut guild = load_guild(vault.db(), member.guild_id);
    let mut model = vault.load_model(member.character_id)?;
    let gold = model.gold();
    let mut usage = None;
    let kind = if req.deposit {
        if gold < req.copper {
            return Err(GuildBankError::NotEnoughMoney);
        }
        if guild.money > GUILD_BANK_MONEY_LIMIT - req.copper {
            return Err(GuildBankError::TooMuchMoney);
        }
        guild.money += req.copper;
        model.set_gold(gold - req.copper);
        GuildBankLogKindData::DepositMoney
    } else {
        if guild.money < req.copper {
            return Err(GuildBankError::NotEnoughGuildMoney);
        }
        let mut today = usage_today(vault.db(), member, vault.today());
        if let Some(limit) = gold_allowance(&guild, member.rank) {
            if today.copper_withdrawn + req.copper > limit {
                return Err(GuildBankError::WithdrawLimit);
            }
            today.copper_withdrawn += req.copper;
        }
        if gold + req.copper > MAX_MONEY {
            return Err(GuildBankError::TooMuchMoney);
        }
        guild.money -= req.copper;
        model.set_gold(gold + req.copper);
        usage = Some(today);
        GuildBankLogKindData::WithdrawMoney
    };
    let mut logs = load_guild_bank_logs(vault.db(), member.guild_id);
    logs.push(
        None,
        GuildBankLogData {
            copper: req.copper,
            ..vault.log(member, kind)
        },
    );
    vault.save(member, &guild, &logs, usage.as_ref(), Some(model));
    Ok(())
}

/// AzerothCore `Guild::HandleBuyBankTab`: the Guild Master pays the next price.
fn buy_tab(vault: &mut GuildVault, member: &Member, req: GuildBankBuyTab) -> GuildBankResult {
    member.vault_for(req.object)?;
    if !member.is_leader() {
        return Err(GuildBankError::Permissions);
    }
    let mut guild = load_guild(vault.db(), member.guild_id);
    let cost = vault
        .next_tab_cost(&guild)
        .ok_or(GuildBankError::Permissions)?;
    let mut model = vault.load_model(member.character_id)?;
    let gold = model.gold();
    if gold < cost {
        return Err(GuildBankError::NotEnoughMoney);
    }
    model.set_gold(gold - cost);
    guild.tabs.push(new_tab(guild.tabs.len() + 1));
    let mut logs = load_guild_bank_logs(vault.db(), member.guild_id);
    logs.push(
        None,
        GuildBankLogData {
            copper: cost,
            ..vault.log(member, GuildBankLogKindData::BuyTab)
        },
    );
    vault.save(member, &guild, &logs, None, Some(model));
    Ok(())
}

fn leader_tab<'a>(
    guild: &'a mut GuildData,
    member: &Member,
    tab: u8,
) -> Result<&'a mut crate::bank_storage::BankTabData, GuildBankError> {
    if !member.is_leader() {
        return Err(GuildBankError::Permissions);
    }
    guild
        .tabs
        .get_mut(usize::from(tab))
        .ok_or(GuildBankError::WrongTab)
}

/// `SetGuildBankTabInfo` (AzerothCore `HandleSetBankTabInfo`).
fn set_tab_info(
    vault: &mut GuildVault,
    member: &Member,
    req: GuildBankSetTabInfo,
) -> GuildBankResult {
    member.vault_for(req.object)?;
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > TAB_NAME_LETTERS {
        return Err(GuildBankError::WrongTab);
    }
    let mut guild = load_guild(vault.db(), member.guild_id);
    let tab = leader_tab(&mut guild, member, req.tab)?;
    tab.name = name.to_owned();
    tab.icon = req.icon;
    let logs = load_guild_bank_logs(vault.db(), member.guild_id);
    vault.save(member, &guild, &logs, None, None);
    Ok(())
}

/// `SetGuildBankText` (AzerothCore `HandleSetBankTabText`).
fn set_tab_text(
    vault: &mut GuildVault,
    member: &Member,
    req: GuildBankSetTabText,
) -> GuildBankResult {
    member.vault_for(req.object)?;
    let mut guild = load_guild(vault.db(), member.guild_id);
    let tab = leader_tab(&mut guild, member, req.tab)?;
    tab.text = req.text.chars().take(TAB_TEXT_LETTERS).collect();
    let logs = load_guild_bank_logs(vault.db(), member.guild_id);
    vault.save(member, &guild, &logs, None, None);
    Ok(())
}

fn log_kind(kind: GuildBankLogKindData) -> GuildBankLogKind {
    match kind {
        GuildBankLogKindData::DepositItem => GuildBankLogKind::DepositItem,
        GuildBankLogKindData::WithdrawItem => GuildBankLogKind::WithdrawItem,
        GuildBankLogKindData::DepositMoney => GuildBankLogKind::DepositMoney,
        GuildBankLogKindData::WithdrawMoney => GuildBankLogKind::WithdrawMoney,
        GuildBankLogKindData::BuyTab => GuildBankLogKind::BuyTab,
        GuildBankLogKindData::RepairMoney => GuildBankLogKind::RepairMoney,
    }
}

/// `QueryGuildBankLog`: a tab log needs view rights on the tab.
fn send_log(vault: &mut GuildVault, member: &Member, req: GuildBankQueryLog) -> GuildBankResult {
    member.vault_for(req.object)?;
    if let Some(tab) = req.tab {
        let guild = load_guild(vault.db(), member.guild_id);
        if usize::from(tab) >= guild.tabs.len() {
            return Err(GuildBankError::WrongTab);
        }
        if !tab_rights(&guild, member.rank, usize::from(tab)).0 {
            return Err(GuildBankError::Permissions);
        }
    }
    let now = unix_now();
    let logs = load_guild_bank_logs(vault.db(), member.guild_id);
    let entries = logs
        .log(req.tab)
        .iter()
        .map(|entry| GuildBankLogEntry {
            kind: log_kind(entry.kind),
            actor: entry.actor.clone(),
            item_id: entry.item_id,
            item_name: entry.item_name.clone(),
            count: entry.count,
            copper: entry.copper,
            seconds_ago: now.saturating_sub(entry.time),
        })
        .collect();
    if let Ok(mut sender) = vault.logs.get_mut(member.connection) {
        sender.send::<GuildBankChannel>(GuildBankLog {
            tab: req.tab,
            entries,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "guild_bank_tests.rs"]
mod tests;
