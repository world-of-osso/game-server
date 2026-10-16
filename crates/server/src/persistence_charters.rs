//! Owned charter items, PetitionManager, unique signing accounts and atomic guild creation.
//! Retail authority: TrinityCore a352b1fa PetitionsHandler.cpp; realm defaults1000c/4signs.
use super::inventory_store::{InventoryRecord, write_inventory_record};
use super::{
    CHARACTER_GUILDS, GUILD_PROFILES, GuildMembershipData, GuildProfileData, InventoryItemData,
    PENDING_GUILD_INVITES,
};
use crate::inventory::load_model;
use crate::item_templates::ItemTemplates;
use crate::merchant::VendorCatalog;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};
use shared::guild_creation::{
    CHARTER_COST, CreatedGuild, PetitionError, PetitionManager, REQUIRED_SIGNATURES,
};
use shared::item_data::ItemDefinitionSource;
use shared::protocol::{GUILD_CHARTER_ITEM, GuildPetitionSnapshot};
use std::collections::{BTreeMap, BTreeSet};

const STATE_ROW: u8 = 0;
pub(crate) const STATE: TableDefinition<u8, &[u8]> = TableDefinition::new("guild_charters");
pub(crate) const CREATED_AT: TableDefinition<u32, u64> = TableDefinition::new("guild_created_at");
const MAX_SIGNATURES: usize = 10; // PetitionsHandler.cpp:253, not the UI's minimum quota.

#[derive(Serialize, Deserialize)]
struct CharterState {
    manager: PetitionManager,
    /// Owned item GUID -> internal petition ID. Only the item GUID crosses the wire.
    charters: BTreeMap<u64, u64>,
    /// Petition ID -> signer character/account identity.
    accounts: BTreeMap<u64, BTreeMap<u64, u64>>,
    next_guild_id: u32,
}
impl Default for CharterState {
    fn default() -> Self {
        Self {
            manager: PetitionManager::new(),
            charters: BTreeMap::new(),
            accounts: BTreeMap::new(),
            next_guild_id: 1,
        }
    }
}
impl CharterState {
    fn petition(&self, guid: u64) -> Result<&shared::guild_creation::Petition, String> {
        self.charters
            .get(&guid)
            .and_then(|id| self.manager.get(*id))
            .ok_or_else(|| "Guild charter not found.".into())
    }
    fn owned_guid(&self, owner: u64) -> Option<u64> {
        let id = self.manager.find_by_owner(owner)?.id;
        self.charters
            .iter()
            .find_map(|(&guid, &petition)| (petition == id).then_some(guid))
    }
    fn prune(&mut self) {
        let alive: BTreeSet<_> = self.manager.petitions().iter().map(|p| p.id).collect();
        self.charters.retain(|_, id| alive.contains(id));
        self.accounts.retain(|id, accounts| {
            let Some(petition) = self.manager.get(*id) else {
                return false;
            };
            accounts.retain(|character, _| petition.signatures.contains(character));
            true
        });
    }
}

fn read_state(db: &Database) -> CharterState {
    let txn = db.begin_read().expect("read guild charter state");
    let table = txn.open_table(STATE).expect("open guild charter state");
    table
        .get(STATE_ROW)
        .expect("read charter state row")
        .map(|row| serde_json::from_slice(row.value()).expect("decode guild charter state"))
        .unwrap_or_default()
}
fn read_state_in(txn: &WriteTransaction) -> CharterState {
    let table = txn.open_table(STATE).expect("open guild charter state");
    table
        .get(STATE_ROW)
        .expect("read charter state row")
        .map(|row| serde_json::from_slice(row.value()).expect("decode guild charter state"))
        .unwrap_or_default()
}
fn write_state(txn: &WriteTransaction, state: &CharterState) {
    let bytes = serde_json::to_vec(state).expect("encode guild charter state");
    txn.open_table(STATE)
        .expect("open guild charter state")
        .insert(STATE_ROW, bytes.as_slice())
        .expect("write guild charter state");
}

/// Initialize once from authentic existing memberships, without inventing legacy dates.
pub(crate) fn initialize(db: &Database) {
    let txn = db.begin_write().expect("initialize guild charter state");
    if txn
        .open_table(STATE)
        .expect("open charter state")
        .get(STATE_ROW)
        .expect("read state")
        .is_some()
    {
        return;
    }
    let mut state = CharterState::default();
    let members = txn.open_table(CHARACTER_GUILDS).expect("open memberships");
    for row in members.iter().expect("iterate memberships") {
        let (_, value) = row.expect("read membership");
        let membership: GuildMembershipData =
            bitcode::decode(value.value()).expect("decode membership");
        state.manager.reserve_name(&membership.guild_name);
        state.next_guild_id = state.next_guild_id.max(
            membership
                .guild_id
                .checked_add(1)
                .expect("guild IDs exhausted"),
        );
    }
    drop(members);
    if let Some((guild, _)) = txn
        .open_table(super::banks::GUILDS)
        .expect("open existing guilds")
        .last()
        .expect("read last guild")
    {
        state.next_guild_id = state
            .next_guild_id
            .max(guild.value().checked_add(1).expect("guild IDs exhausted"));
    }
    write_state(&txn, &state);
    crate::persistence_commit::commit(txn).expect("commit charter initialization");
}

fn is_guilded(txn: &WriteTransaction, character: u64) -> bool {
    txn.open_table(CHARACTER_GUILDS)
        .expect("open memberships")
        .get(character)
        .expect("read membership")
        .is_some()
}
fn has_invite(txn: &WriteTransaction, character: u64) -> bool {
    txn.open_table(PENDING_GUILD_INVITES)
        .expect("open invitations")
        .get(character)
        .expect("read invitation")
        .is_some()
}
fn petition_error(error: PetitionError) -> String {
    match error {
        PetitionError::InsufficientFunds => "You don't have enough money.",
        PetitionError::AlreadyHasPetition => "You already have a guild charter.",
        PetitionError::AlreadyInGuild => "You are already in a guild.",
        PetitionError::InvalidName => "Invalid guild name.",
        PetitionError::NameTaken => "A guild with that name already exists.",
        PetitionError::NotFound => "Guild charter not found.",
        PetitionError::AlreadySigned => "You have already signed that guild charter.",
        PetitionError::NotEnoughSignatures => "You need more signatures on your guild charter.",
        PetitionError::NotOwner => "You are not the petition owner.",
    }
    .into()
}

pub(crate) fn purchase(
    db: &Database,
    templates: &ItemTemplates,
    catalog: &VendorCatalog,
    owner: u64,
    name: &str,
    now: u64,
) -> Result<u64, String> {
    let (mut model, item, template) = prepare_charter_purchase(db, templates, catalog, owner)?;
    let guid = item.item_guid;
    let txn = db.begin_write().expect("begin charter purchase");
    let mut state = read_state_in(&txn);
    let id = state
        .manager
        .purchase_charter(
            owner,
            name.trim(),
            model.gold(),
            is_guilded(&txn, owner),
            now,
        )
        .map_err(petition_error)?;
    model
        .store_item(item, ItemDefinitionSource::Retail, template, &mut || {
            unreachable!("one charter retains its minted GUID")
        })
        .map_err(|error| format!("{error:?}"))?;
    model.set_gold(model.gold() - CHARTER_COST);
    state.charters.insert(guid, id);
    write_inventory_record(&txn, owner, &model.into_record());
    write_state(&txn, &state);
    crate::persistence_commit::commit(txn).expect("commit charter purchase");
    Ok(guid)
}

fn prepare_charter_purchase(
    db: &Database,
    templates: &ItemTemplates,
    catalog: &VendorCatalog,
    owner: u64,
) -> Result<
    (
        crate::inventory::inventory_model::InventoryModel,
        InventoryItemData,
        crate::item_templates::ItemTemplate,
    ),
    String,
> {
    let template = *templates
        .get(ItemDefinitionSource::Retail, GUILD_CHARTER_ITEM)
        .ok_or("Guild Charter item5863 is unavailable.")?;
    let goods = catalog
        .goods
        .get(&GUILD_CHARTER_ITEM)
        .ok_or("Guild Charter item5863 has no authentic item data.")?;
    let model = load_model(db, templates, owner).map_err(|error| format!("{error:?}"))?;
    if goods.max_count != 0
        && model.owned_count(ItemDefinitionSource::Retail, GUILD_CHARTER_ITEM) >= goods.max_count
    {
        return Err("You can't carry any more of those items.".into());
    }
    let item = InventoryItemData {
        item_guid: super::next_id(db, "item_instance"),
        item_id: GUILD_CHARTER_ITEM,
        name: goods.name.clone(),
        quality: goods.quality,
        required_level: template.required_level,
        stack_count: 1,
        vendor_sell_price: goods.sell_price,
        unique_auction: false,
    };
    Ok((model, item, template))
}

pub(crate) fn owned_charter_guid(db: &Database, owner: u64) -> Option<u64> {
    read_state(db).owned_guid(owner)
}

pub(crate) fn snapshot(
    db: &Database,
    guid: u64,
    actor: u64,
    account: u64,
) -> Result<GuildPetitionSnapshot, String> {
    let state = read_state(db);
    let petition = state.petition(guid)?;
    let owner = super::load_character(db, petition.owner).ok_or("Petition owner not found.")?;
    let signatures = petition
        .signatures
        .iter()
        .map(|&signer| {
            super::load_character(db, signer)
                .map(|character| character.name)
                .ok_or_else(|| "Petition signer not found.".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let signed = state
        .accounts
        .get(&petition.id)
        .is_some_and(|accounts| accounts.values().any(|&signed| signed == account));
    Ok(GuildPetitionSnapshot {
        charter_guid: guid,
        guild_name: petition.guild_name.clone(),
        owner_name: owner.name,
        signatures,
        required_signatures: REQUIRED_SIGNATURES as u8,
        is_owner: petition.owner == actor,
        can_sign: actor != petition.owner
            && !signed
            && petition.signatures.len() < REQUIRED_SIGNATURES
            && super::load_character_guild(db, actor).is_none()
            && super::load_pending_guild_invite(db, actor).is_none(),
    })
}

pub(crate) fn sign(db: &Database, guid: u64, signer: u64, account: u64) -> Result<(), String> {
    let txn = db.begin_write().expect("begin charter signing");
    let mut state = read_state_in(&txn);
    let petition = state.petition(guid)?;
    let id = petition.id;
    if has_invite(&txn, signer) {
        return Err("You have already been invited into a guild.".into());
    }
    if petition.signatures.len() >= MAX_SIGNATURES {
        return Err("This charter already has the maximum number of signatures.".into());
    }
    if state
        .accounts
        .get(&id)
        .is_some_and(|accounts| accounts.values().any(|&signed| signed == account))
    {
        return Err(petition_error(PetitionError::AlreadySigned));
    }
    state
        .manager
        .sign_petition(id, signer, is_guilded(&txn, signer))
        .map_err(petition_error)?;
    state
        .accounts
        .entry(id)
        .or_default()
        .insert(signer, account);
    write_state(&txn, &state);
    crate::persistence_commit::commit(txn).expect("commit charter signature");
    Ok(())
}

pub(crate) fn rename(db: &Database, guid: u64, owner: u64, name: &str) -> Result<(), String> {
    let txn = db.begin_write().expect("begin charter rename");
    let mut state = read_state_in(&txn);
    let id = state.petition(guid)?.id;
    state
        .manager
        .rename(id, owner, name.trim())
        .map_err(petition_error)?;
    write_state(&txn, &state);
    crate::persistence_commit::commit(txn).expect("commit charter rename");
    Ok(())
}

pub(crate) fn turn_in(
    db: &Database,
    templates: &ItemTemplates,
    guid: u64,
    owner: u64,
    now: u64,
) -> Result<CreatedGuild, String> {
    let mut model = load_model(db, templates, owner).map_err(|error| format!("{error:?}"))?;
    let location = model
        .view()
        .into_iter()
        .find_map(|(location, item)| {
            (item.item_guid == guid
                && item.item_id == GUILD_CHARTER_ITEM
                && item.definition_source == ItemDefinitionSource::Retail)
                .then_some(location)
        })
        .ok_or("You don't own that guild charter.")?;
    let txn = db.begin_write().expect("begin guild creation");
    let mut state = read_state_in(&txn);
    if is_guilded(&txn, owner) {
        return Err("You are already in a guild.".into());
    }
    let id = state.petition(guid)?.id;
    let created = state
        .manager
        .submit_petition(id, owner, state.next_guild_id)
        .map_err(petition_error)?;
    state.next_guild_id = state
        .next_guild_id
        .checked_add(1)
        .ok_or("Guild IDs exhausted.")?;
    model
        .destroy(location, 1)
        .map_err(|error| format!("{error:?}"))?;
    for member in std::iter::once(owner).chain(created.founding_members.iter().copied()) {
        state.manager.remove_guilded_player(member);
    }
    state.prune();
    write_state(&txn, &state);
    write_inventory_record(&txn, owner, &model.into_record());
    write_created_guild(&txn, &created, now);
    crate::persistence_commit::commit(txn).expect("commit guild creation and charter consumption");
    Ok(created)
}

fn write_created_guild(txn: &WriteTransaction, created: &CreatedGuild, now: u64) {
    let guild = super::banks::GuildData::default();
    let bytes = bitcode::encode(&guild);
    txn.open_table(super::banks::GUILDS)
        .expect("open guilds")
        .insert(created.guild_id, bytes.as_slice())
        .expect("write created guild");
    let profile = bitcode::encode(&GuildProfileData::default());
    txn.open_table(GUILD_PROFILES)
        .expect("open profiles")
        .insert(created.guild_id, profile.as_slice())
        .expect("write initial profile");
    txn.open_table(CREATED_AT)
        .expect("open guild dates")
        .insert(created.guild_id, now)
        .expect("write actual creation date");
    write_founders(txn, created, &guild, now);
}

fn write_founders(
    txn: &WriteTransaction,
    created: &CreatedGuild,
    guild: &super::banks::GuildData,
    now: u64,
) {
    let membership = bitcode::encode(&GuildMembershipData {
        guild_id: created.guild_id,
        guild_name: created.guild_name.clone(),
    });
    for member in
        std::iter::once(created.guild_master).chain(created.founding_members.iter().copied())
    {
        txn.open_table(CHARACTER_GUILDS)
            .expect("open memberships")
            .insert(member, membership.as_slice())
            .expect("join founding member");
        let rank = if member == created.guild_master {
            0
        } else {
            guild.lowest_rank()
        };
        txn.open_table(super::banks::GUILD_MEMBER_RANKS)
            .expect("open ranks")
            .insert(member, rank)
            .expect("write founding rank");
        txn.open_table(PENDING_GUILD_INVITES)
            .expect("open invites")
            .remove(member)
            .expect("clear founding invite");
        crate::guild_event_log::append_in(
            txn,
            created.guild_id,
            shared::protocol::GuildEvent {
                timestamp: now,
                kind: shared::protocol::GuildEventKind::Join {
                    member: crate::guild_event_log::character_name_in(txn, member),
                },
            },
        );
    }
}

pub(crate) fn created_at(db: &Database, guild: u32) -> Option<u64> {
    let txn = db.begin_read().expect("read guild creation date");
    txn.open_table(CREATED_AT)
        .expect("open guild dates")
        .get(guild)
        .expect("read guild date")
        .map(|date| date.value())
}

/// Existing joins also remove pending ownership and signatures in the same transaction.
pub(crate) fn join_guild_in(txn: &WriteTransaction, character: u64, guild: u32, name: &str) {
    let mut state = read_state_in(txn);
    state.manager.reserve_name(name);
    state.next_guild_id = state
        .next_guild_id
        .max(guild.checked_add(1).expect("guild IDs exhausted"));
    state.manager.remove_guilded_player(character);
    state.prune();
    write_state(txn, &state);
}

/// Item destruction/sale/trade cannot leave a usable petition without the owned charter.
pub(crate) fn prune_destroyed_in(txn: &WriteTransaction, owner: u64, record: &InventoryRecord) {
    let mut state = read_state_in(txn);
    let Some(guid) = state.owned_guid(owner) else {
        return;
    };
    if record
        .inventory
        .items
        .iter()
        .any(|item| item.item_guid == guid)
    {
        return;
    }
    let id = state.charters[&guid];
    state.manager.cancel(id, owner).expect("owned petition");
    state.prune();
    write_state(txn, &state);
}

pub(crate) fn owner_of(db: &Database, guid: u64) -> Result<u64, String> {
    Ok(read_state(db).petition(guid)?.owner)
}
