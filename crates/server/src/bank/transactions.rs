//! Bank mutations load fresh FIFO state and return committed owner publications.
use super::*;

#[derive(Clone, Default)]
pub(super) struct Publication {
    pub gold: Option<u64>,
    pub contents: Vec<BankContents>,
    pub criteria: Vec<(u64, crate::achievement::CriteriaEvent)>,
    pub refusal: Option<BankError>,
}

pub(super) struct Transaction<'a> {
    database: &'a Database,
    templates: Option<ItemTemplates>,
    prices: BankTabPrices,
    effects: Publication,
}

pub(super) fn execute<M>(
    db: &Database,
    templates: ItemTemplates,
    prices: BankTabPrices,
    patron: Patron,
    request: M,
    apply: impl Fn(&mut Transaction, &Patron, M) -> BankResult,
) -> Publication {
    let mut vault = Transaction {
        database: db,
        templates: Some(templates),
        prices,
        effects: Publication::default(),
    };
    vault.effects.refusal = apply(&mut vault, &patron, request).err();
    vault.effects
}

impl Transaction<'_> {
    fn db(&self) -> &Database {
        self.database
    }
    fn load_template(
        &self,
        item: &crate::persistence::InventoryItemData,
    ) -> Result<(shared::item_data::ItemDefinitionSource, ItemTemplate), BankError> {
        let source =
            persistence::inventory_store::load_item_definition_source(self.db(), item.item_guid);
        let template = self
            .templates
            .as_ref()
            .and_then(|templates| templates.get(source, item.item_id).copied())
            .ok_or(BankError::ItemNotFound)?;
        Ok((source, template))
    }

    fn mint(&self) -> impl FnMut() -> u64 + '_ {
        let db = self.db();
        move || persistence::next_id(db, "item_instance")
    }

    fn load_model(&self, character_id: u64) -> Result<InventoryModel, BankError> {
        let templates = self.templates.as_ref().expect("checked by ready");
        load_model(self.db(), templates, character_id).map_err(|_| BankError::ItemNotFound)
    }

    fn load_bank(&self, patron: &Patron, bank: BankType) -> BankData {
        match bank {
            BankType::Character => load_character_bank(self.db(), patron.character_id),
            BankType::Account => load_account_bank(self.db(), patron.account_id),
        }
    }

    /// Save the bank, plus the bags and money when `model` is given, then send the
    /// new bank contents.
    fn save(
        &mut self,
        patron: &Patron,
        bank: BankType,
        data: &BankData,
        model: Option<InventoryModel>,
    ) {
        let owner = match bank {
            BankType::Character => BankOwner::Character(patron.character_id, data),
            BankType::Account => BankOwner::Account(patron.account_id, data),
        };
        let gold = model.as_ref().map(InventoryModel::gold);
        let record = model.map(InventoryModel::into_record);
        save_bank_move(self.db(), patron.character_id, owner, record.as_ref());
        if let Some(gold) = gold {
            self.effects.gold = Some(gold);
        }
        self.send_contents(patron.connection, bank, data);
    }

    fn contents(&self, bank: BankType, data: &BankData) -> BankContents {
        let guids = data.tabs.iter().flat_map(|tab| tab.slots.iter().flatten());
        let bound =
            persistence::inventory_store::soulbound_guids(self.db(), guids.map(|i| i.item_guid));
        let soulbound = |guid| bound.contains(&guid);
        BankContents {
            bank,
            tabs: data
                .tabs
                .iter()
                .map(|tab| BankTabView {
                    name: tab.name.clone(),
                    icon: tab.icon,
                    deposit_flags: tab.deposit_flags,
                    slots: bank_storage::tab_slots(tab, soulbound, |guid| {
                        persistence::inventory_store::load_item_definition_source(self.db(), guid)
                    }),
                })
                .collect(),
            next_tab_cost: self.prices.of(bank).get(data.tabs.len()).copied(),
            money: (bank == BankType::Account).then_some(data.money),
        }
    }

    fn send_contents(&mut self, connection: Entity, bank: BankType, data: &BankData) {
        let contents = self.contents(bank, data);
        let _ = connection;
        self.effects.contents.push(contents);
    }
}

/// Move one bag stack into `data`, visiting tabs in `order`; nothing changes when
/// it doesn't fit.
fn move_into_bank(
    vault: &Transaction,
    model: &mut InventoryModel,
    data: &mut BankData,
    bank: BankType,
    order: &[usize],
    item_guid: u64,
) -> BankResult {
    let item = model
        .bag_item(item_guid)
        .cloned()
        .ok_or(BankError::ItemNotFound)?;
    if bank == BankType::Account && model.is_soulbound(item_guid) {
        return Err(BankError::SoulboundInAccountBank);
    }
    let source = model.definition_source(item_guid);
    let template = vault
        .templates
        .as_ref()
        .and_then(|templates| templates.get(source, item.item_id))
        .ok_or(BankError::ItemNotFound)?;
    let mut sources = bank_storage::load_tab_definition_sources(vault.db(), &data.tabs);
    sources.insert(item_guid, source);
    let mut mint = vault.mint();
    bank_storage::deposit(
        &mut data.tabs,
        order,
        item,
        template.max_stack,
        &mut sources,
        &mut mint,
    )
    .map_err(|TabsFull| BankError::BankFull)?;
    model.remember_definition_sources(sources);
    model
        .take_bag_item(item_guid, 0, &mut mint)
        .map_err(|_| BankError::ItemNotFound)?;
    Ok(())
}

pub(super) fn deposit(vault: &mut Transaction, patron: &Patron, req: BankDeposit) -> BankResult {
    patron.banker_for(req.npc)?;
    let mut data = vault.load_bank(patron, req.bank);
    if usize::from(req.tab) >= data.tabs.len() {
        return Err(BankError::TabLocked);
    }
    let mut model = vault.load_model(patron.character_id)?;
    let order = bank_storage::tab_order(data.tabs.len(), usize::from(req.tab));
    move_into_bank(
        vault,
        &mut model,
        &mut data,
        req.bank,
        &order,
        req.item_guid,
    )?;
    vault.save(patron, req.bank, &data, Some(model));
    Ok(())
}

pub(super) fn withdraw(vault: &mut Transaction, patron: &Patron, req: BankWithdraw) -> BankResult {
    patron.banker_for(req.npc)?;
    let mut data = vault.load_bank(patron, req.bank);
    let item =
        bank_storage::take(&mut data.tabs, req.tab, req.slot).ok_or(BankError::ItemNotFound)?;
    let mut model = vault.load_model(patron.character_id)?;
    let (source, template) = vault.load_template(&item)?;
    let mut mint = vault.mint();
    model
        .store_item(item, source, template, &mut mint)
        .map_err(|_| BankError::InventoryFull)?;
    drop(mint);
    vault.save(patron, req.bank, &data, Some(model));
    Ok(())
}

/// `C_Bank.PurchaseBankTab`: the player pays the next `BankTab` price.
pub(super) fn purchase_tab(
    vault: &mut Transaction,
    patron: &Patron,
    req: BankPurchaseTab,
) -> BankResult {
    patron.banker_for(req.npc)?;
    let mut data = vault.load_bank(patron, req.bank);
    let cost = vault
        .prices
        .of(req.bank)
        .get(data.tabs.len())
        .copied()
        .ok_or(BankError::MaxTabs)?;
    let mut model = vault.load_model(patron.character_id)?;
    let gold = model.gold();
    if gold < cost {
        return Err(BankError::CantAfford);
    }
    model.set_gold(gold - cost);
    data.tabs.push(new_tab(data.tabs.len() + 1));
    vault.save(patron, req.bank, &data, Some(model));
    let (character, event) = (
        patron.character_id,
        crate::achievement::CriteriaEvent::BankTabsOwned(
            criteria_bank_type(req.bank),
            data.tabs.len() as u32,
        ),
    );
    vault.effects.criteria.push((character, event));
    Ok(())
}

/// TC `BankType` numbering of achievement criteria (Character 0, Account 2).
pub(crate) fn criteria_bank_type(bank: BankType) -> u32 {
    match bank {
        BankType::Character => 0,
        BankType::Account => 2,
    }
}

/// Retail `C_Bank.DepositMoney` / `WithdrawMoney`; only the Warband bank holds money.
pub(super) fn transfer_money(
    vault: &mut Transaction,
    patron: &Patron,
    req: BankMoneyTransfer,
) -> BankResult {
    patron.banker_for(req.npc)?;
    if req.bank != BankType::Account {
        return Err(BankError::NoMoneyTransfer);
    }
    let mut data = vault.load_bank(patron, req.bank);
    let mut model = vault.load_model(patron.character_id)?;
    let gold = model.gold();
    let (gold, money) = if req.deposit {
        if gold < req.copper {
            return Err(BankError::NotEnoughMoney);
        }
        if data.money + req.copper > MAX_MONEY {
            return Err(BankError::TooMuchGold);
        }
        (gold - req.copper, data.money + req.copper)
    } else {
        if data.money < req.copper {
            return Err(BankError::NotEnoughMoney);
        }
        if gold + req.copper > MAX_MONEY {
            return Err(BankError::TooMuchGold);
        }
        (gold + req.copper, data.money - req.copper)
    };
    model.set_gold(gold);
    data.money = money;
    vault.save(patron, req.bank, &data, Some(model));
    Ok(())
}

/// Retail `Enum.BagSlotFlags` assignments an item matches.
fn item_flags(template: &ItemTemplate, quality: u8) -> u32 {
    let class = match template.scaling.class_id {
        ITEM_CLASS_CONSUMABLE => bank_tab_flags::CONSUMABLES,
        ITEM_CLASS_WEAPON | ITEM_CLASS_ARMOR => bank_tab_flags::EQUIPMENT,
        ITEM_CLASS_TRADEGOODS => bank_tab_flags::REAGENTS | bank_tab_flags::PROFESSION_GOODS,
        _ => 0,
    };
    let junk = if quality == 0 {
        bank_tab_flags::JUNK
    } else {
        0
    };
    class | junk
}

/// Tabs assigned to any of `flags` first, then the others (Retail tab deposit settings).
fn assigned_order(tabs: &[BankTabData], flags: u32) -> Vec<usize> {
    let (assigned, rest): (Vec<usize>, Vec<usize>) =
        (0..tabs.len()).partition(|&tab| tabs[tab].deposit_flags & flags != 0);
    assigned.into_iter().chain(rest).collect()
}

/// Bag stacks `C_Bank.AutoDepositItemsIntoBank` moves: reagents for the character
/// bank ("Deposit All Reagents"); Warbound items, plus tradeable reagents when asked,
/// for the Warband bank ("Deposit All Warbound Items").
fn auto_deposit_items(
    vault: &Transaction,
    model: &InventoryModel,
    bank: BankType,
    include_reagents: bool,
) -> Vec<(u64, u32)> {
    model
        .bag_items()
        .filter_map(|item: &InventoryItemData| {
            let (_, template) = vault
                .load_template(item)
                .expect("loaded owned item has a definition");
            let reagent = template.scaling.class_id == ITEM_CLASS_TRADEGOODS;
            let selected = match bank {
                BankType::Character => reagent,
                BankType::Account => {
                    WARBOUND_BONDING.contains(&template.bonding)
                        || (include_reagents && reagent && !model.is_soulbound(item.item_guid))
                }
            };
            selected.then(|| (item.item_guid, item_flags(&template, item.quality)))
        })
        .collect()
}

pub(super) fn auto_deposit(
    vault: &mut Transaction,
    patron: &Patron,
    req: BankAutoDeposit,
) -> BankResult {
    patron.banker_for(req.npc)?;
    let mut data = vault.load_bank(patron, req.bank);
    let mut model = vault.load_model(patron.character_id)?;
    let items = auto_deposit_items(vault, &model, req.bank, req.include_reagents);
    let mut full = false;
    for (item_guid, flags) in items {
        let order = assigned_order(&data.tabs, flags);
        match move_into_bank(vault, &mut model, &mut data, req.bank, &order, item_guid) {
            Ok(()) => {}
            // The stack stays in the bags; later, smaller stacks may still fit.
            Err(BankError::BankFull) => full = true,
            Err(error) => return Err(error),
        }
    }
    vault.save(patron, req.bank, &data, Some(model));
    if full {
        return Err(BankError::BankFull);
    }
    Ok(())
}

pub(super) fn update_tab_settings(
    vault: &mut Transaction,
    patron: &Patron,
    req: BankUpdateTabSettings,
) -> BankResult {
    patron.banker_for(req.npc)?;
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > TAB_NAME_LETTERS {
        return Err(BankError::InvalidTabName);
    }
    let mut data = vault.load_bank(patron, req.bank);
    let tab = data
        .tabs
        .get_mut(usize::from(req.tab))
        .ok_or(BankError::TabLocked)?;
    tab.name = name.to_owned();
    tab.icon = req.icon;
    tab.deposit_flags = req.deposit_flags;
    vault.save(patron, req.bank, &data, None);
    Ok(())
}
