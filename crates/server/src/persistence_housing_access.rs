//! Retail HouseAccess* relationships, read from persisted social state and the live party registry.
use super::{Actor, OwnedHouse, State};
use crate::group::GroupRegistry;
use crate::persistence::{CHARACTER_GUILDS, CHARACTERS, FRIENDS};
use redb::{Database, ReadTransaction, ReadableDatabase};

pub(super) fn require_visit(
    db: &Database,
    state: &State,
    actor: Actor,
    house_id: u64,
    groups: Option<&GroupRegistry>,
) -> Result<(), String> {
    let house = state.houses.get(&house_id).ok_or("House not found.")?;
    if house.account_id == actor.account_id {
        return Ok(());
    }
    let txn = db.begin_read().map_err(|e| e.to_string())?;
    let allowed = match house.house_access {
        None => false,
        Some(access) => {
            access.anyone
                || (access.neighbors && is_neighbor(state, actor, house))
                || (access.guild
                    && read_same_guild(&txn, house.owner_character_id, actor.character_id)?)
                || (access.friends
                    && read_owner_friend(&txn, house.owner_character_id, actor.character_id)?)
                || (access.party
                    && groups.is_some_and(|groups| {
                        groups.is_in_same_group(house.owner_character_id, actor.character_id)
                    }))
        }
    };
    if allowed {
        Ok(())
    } else {
        Err("The owner has not allowed you to visit this house.".into())
    }
}

fn is_neighbor(state: &State, actor: Actor, house: &OwnedHouse) -> bool {
    state.houses.values().any(|other| {
        other.account_id == actor.account_id && other.neighborhood_id == house.neighborhood_id
    })
}

fn read_same_guild(txn: &ReadTransaction, owner: u64, visitor: u64) -> Result<bool, String> {
    let table = txn
        .open_table(CHARACTER_GUILDS)
        .map_err(|e| e.to_string())?;
    let guild = |character| -> Result<Option<u32>, String> {
        table
            .get(character)
            .map_err(|e| e.to_string())?
            .map(|row| {
                bitcode::decode::<crate::persistence_types::GuildMembershipData>(row.value())
                    .map(|guild| guild.guild_id)
                    .map_err(|e| format!("Read housing guild relationship: {e}"))
            })
            .transpose()
    };
    Ok(matches!((guild(owner)?, guild(visitor)?), (Some(a), Some(b)) if a != 0 && a == b))
}

fn read_owner_friend(txn: &ReadTransaction, owner: u64, visitor: u64) -> Result<bool, String> {
    let friends = txn.open_table(FRIENDS).map_err(|e| e.to_string())?;
    let Some(row) = friends.get(owner).map_err(|e| e.to_string())? else {
        return Ok(false);
    };
    let friends: crate::friends::PlayerFriendsState = serde_json::from_slice(row.value())
        .map_err(|e| format!("Read housing owner friends: {e}"))?;
    let characters = txn.open_table(CHARACTERS).map_err(|e| e.to_string())?;
    let Some(row) = characters.get(visitor).map_err(|e| e.to_string())? else {
        return Ok(false);
    };
    let character = crate::persistence::CharacterData::decode_compatible(row.value())
        .map_err(|e| format!("Read housing visitor identity: {e}"))?;
    Ok(friends
        .entries
        .iter()
        .any(|friend| friend.name.eq_ignore_ascii_case(&character.name)))
}
