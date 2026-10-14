//! Guild bank operations use fresh member/allowance rows on the FIFO writer.
use super::*;
#[derive(Clone, Default)]
pub(super) struct Publication {
    pub gold: Option<u64>,
    pub guild: Option<u32>,
    pub logs: Vec<GuildBankLog>,
    pub refusal: Option<GuildBankError>,
}
pub(super) struct Transaction<'a> {
    database: &'a Database,
    templates: Option<ItemTemplates>,
    prices: BankTabPrices,
    reset: DailyReset,
    effects: Publication,
}
pub(super) fn execute<M>(
    db: &Database,
    templates: ItemTemplates,
    prices: BankTabPrices,
    reset: DailyReset,
    connection: Entity,
    character: u64,
    open: Option<Entity>,
    request: M,
    apply: impl Fn(&mut Transaction, &Member, M) -> GuildBankResult,
) -> Publication {
    let Some((member, _)) = member(db, connection, character, open) else {
        return Publication {
            refusal: Some(GuildBankError::NotInGuild),
            ..Default::default()
        };
    };
    let mut vault = Transaction {
        database: db,
        templates: Some(templates),
        prices,
        reset,
        effects: Publication::default(),
    };
    vault.effects.refusal = apply(&mut vault, &member, request).err();
    vault.effects
}
impl Transaction<'_> {
    fn db(&self) -> &Database {
        self.database
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
            self.effects.gold = Some(gold);
        }
        self.effects.guild = Some(member.guild_id);
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

/// AzerothCore `Guild::SwapItemsWithInventory` (bags to bank): the rank must view and
/// deposit into the tab; soulbound and quest items are refused.
pub(super) fn deposit(
    vault: &mut Transaction,
    member: &Member,
    req: GuildBankDeposit,
) -> GuildBankResult {
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
pub(super) fn withdraw(
    vault: &mut Transaction,
    member: &Member,
    req: GuildBankWithdraw,
) -> GuildBankResult {
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
pub(super) fn transfer_money(
    vault: &mut Transaction,
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
pub(super) fn buy_tab(
    vault: &mut Transaction,
    member: &Member,
    req: GuildBankBuyTab,
) -> GuildBankResult {
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
pub(super) fn set_tab_info(
    vault: &mut Transaction,
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
pub(super) fn set_tab_text(
    vault: &mut Transaction,
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
pub(super) fn send_log(
    vault: &mut Transaction,
    member: &Member,
    req: GuildBankQueryLog,
) -> GuildBankResult {
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
    vault.effects.logs.push(GuildBankLog {
        tab: req.tab,
        entries,
    });
    Ok(())
}
