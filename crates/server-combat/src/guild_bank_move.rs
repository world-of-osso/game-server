//! Authenticated bank-to-bank moves. TrinityCore Guild.cpp:833-870, 945-1029,
//! 3120-3127 and 3151-3245 define same-tab rights, placement and logging.
use super::*;
use crate::persistence::banks::save_guild_bank_rearrangement;

pub(super) fn process_moves(mut requests: Requests<GuildBankMove>, mut vault: GuildVault) {
    process(
        &mut requests,
        &mut vault,
        |request| request.object,
        move_items,
    );
}

fn check_cross_tab_rights(
    guild: &GuildData,
    member: &Member,
    usage: &GuildBankUsage,
    from: usize,
    to: usize,
) -> GuildBankResult {
    if from == to {
        return Ok(());
    }
    let (view, _, allowance) = tab_rights(guild, member.rank, from);
    if !view || !tab_rights(guild, member.rank, to).1 {
        return Err(GuildBankError::Permissions);
    }
    if allowance.is_some_and(|limit| usage.tab_withdrawals[from] >= limit) {
        return Err(GuildBankError::WithdrawLimit);
    }
    Ok(())
}

fn log_move(
    logs: &mut GuildBankLogs,
    vault: &GuildVault,
    member: &Member,
    from: u8,
    to: u8,
    item: &crate::persistence::InventoryItemData,
    count: u32,
) {
    if from == to {
        return;
    }
    logs.push(
        Some(from),
        GuildBankLogData {
            item_id: item.item_id,
            item_name: item.name.clone(),
            count,
            copper: u64::from(to),
            ..vault.log(member, GuildBankLogKindData::MoveItem)
        },
    );
}

fn move_items(vault: &mut GuildVault, member: &Member, request: GuildBankMove) -> GuildBankResult {
    member.vault_for(request.object)?;
    let mut guild = load_guild(vault.db(), member.guild_id);
    let (from, to) = (usize::from(request.from_tab), usize::from(request.to_tab));
    let (from_slot, to_slot) = (usize::from(request.from_slot), usize::from(request.to_slot));
    if from >= guild.tabs.len()
        || to >= guild.tabs.len()
        || from_slot >= guild.tabs[from].slots.len()
        || to_slot >= guild.tabs[to].slots.len()
    {
        return Err(GuildBankError::WrongTab);
    }
    if from == to && from_slot == to_slot {
        return Ok(());
    }
    let source = guild.tabs[from].slots[from_slot]
        .clone()
        .ok_or(GuildBankError::ItemNotFound)?;
    if request.count > source.stack_count {
        return Err(GuildBankError::ItemNotFound);
    }
    let amount = if request.count == 0 {
        source.stack_count
    } else {
        request.count
    };
    let split = amount < source.stack_count;
    let (source_definition, template) = vault.load_template(&source)?;
    let destination = guild.tabs[to].slots[to_slot].clone();
    let mut usage = usage_today(vault.db(), member, vault.today());
    check_cross_tab_rights(&guild, member, &usage, from, to)?;
    let mut sources = bank_storage::load_tab_definition_sources(vault.db(), &guild.tabs);
    let compatible = destination.as_ref().is_some_and(|item| {
        item.item_id == source.item_id
            && sources[&item.item_guid] == source_definition
            && item.stack_count < template.max_stack.max(1)
    });
    let swap = destination.is_some() && !compatible;
    if swap && split {
        return Err(GuildBankError::TabFull);
    }
    if swap {
        check_cross_tab_rights(&guild, member, &usage, to, from)?;
    }
    let mut clones = Vec::new();
    let mut logs = load_guild_bank_logs(vault.db(), member.guild_id);
    log_move(
        &mut logs,
        vault,
        member,
        request.from_tab,
        request.to_tab,
        &source,
        amount,
    );
    if let Some(other) = destination.as_ref().filter(|_| swap) {
        log_move(
            &mut logs,
            vault,
            member,
            request.to_tab,
            request.from_tab,
            other,
            other.stack_count,
        );
    }
    if swap {
        guild.tabs[from].slots[from_slot] = destination;
        guild.tabs[to].slots[to_slot] = Some(source);
    } else {
        let mut moving = source.clone();
        moving.stack_count = amount;
        if split {
            guild.tabs[from].slots[from_slot]
                .as_mut()
                .expect("source exists")
                .stack_count -= amount;
            moving.item_guid = vault.mint()();
            sources.insert(moving.item_guid, source_definition);
            clones.push((source.item_guid, moving.item_guid));
        } else {
            guild.tabs[from].slots[from_slot] = None;
        }
        if let Some(target) = guild.tabs[to].slots[to_slot].as_mut() {
            let merged = amount.min(template.max_stack.max(1) - target.stack_count);
            target.stack_count += merged;
            moving.stack_count -= merged;
        } else {
            guild.tabs[to].slots[to_slot] = Some(moving.clone());
            moving.stack_count = 0;
        }
        if moving.stack_count > 0 {
            let original = source.item_guid;
            let mut mint = vault.mint();
            bank_storage::deposit(
                &mut guild.tabs,
                &[to],
                moving,
                template.max_stack,
                &mut sources,
                &mut || {
                    let guid = mint();
                    clones.push((original, guid));
                    guid
                },
            )
            .map_err(|TabsFull| GuildBankError::TabFull)?;
        }
    }
    if from != to {
        if tab_rights(&guild, member.rank, from).2.is_some() {
            usage.tab_withdrawals[from] += 1;
        }
        if swap && tab_rights(&guild, member.rank, to).2.is_some() {
            usage.tab_withdrawals[to] += 1;
        }
    }
    // A partial merge can consume its freshly minted GUID without creating a slot.
    clones.retain(|(_, guid)| {
        guild.tabs.iter().any(|tab| {
            tab.slots
                .iter()
                .flatten()
                .any(|item| item.item_guid == *guid)
        })
    });
    let owner = BankOwner::Guild {
        guild_id: member.guild_id,
        guild: &guild,
        logs: &logs,
        usage: Some(&usage),
    };
    save_guild_bank_rearrangement(vault.db(), member.character_id, owner, &clones);
    vault.changed.0.push(member.guild_id);
    Ok(())
}
