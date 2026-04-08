//! Guild creation via petition system.
//!
//! Players purchase a guild charter, collect signatures from unguilded
//! players, then submit the petition to create the guild.
//! Ref: AzerothCore `Guild.cpp`, `PetitionHandler.cpp`.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// Cost of a guild charter in copper (10 silver = 1000 copper).
pub const CHARTER_COST: u32 = 1000;

/// Number of signatures required to submit a petition.
pub const REQUIRED_SIGNATURES: usize = 4;

/// Why a petition operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetitionError {
    /// Player doesn't have enough gold.
    InsufficientFunds,
    /// Player already has a pending petition.
    AlreadyHasPetition,
    /// Player is already in a guild.
    AlreadyInGuild,
    /// Guild name is invalid (empty or too long).
    InvalidName,
    /// Guild name is already taken.
    NameTaken,
    /// Petition not found.
    NotFound,
    /// Signer has already signed this petition.
    AlreadySigned,
    /// Not enough signatures to submit.
    NotEnoughSignatures,
    /// Player is not the petition owner.
    NotOwner,
}

/// Maximum guild name length.
pub const MAX_GUILD_NAME_LEN: usize = 24;

/// A guild charter petition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Petition {
    /// Unique petition ID.
    pub id: u64,
    /// Player who purchased the charter.
    pub owner: u64,
    /// Proposed guild name.
    pub guild_name: String,
    /// Player IDs who have signed.
    pub signatures: Vec<u64>,
    /// Server timestamp when purchased.
    pub created_at: u64,
}

impl Petition {
    /// Whether enough signatures have been collected.
    pub fn has_enough_signatures(&self) -> bool {
        self.signatures.len() >= REQUIRED_SIGNATURES
    }

    /// Whether a player has already signed.
    pub fn has_signed(&self, player: u64) -> bool {
        self.signatures.contains(&player)
    }
}

/// Manages guild petitions.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PetitionManager {
    petitions: Vec<Petition>,
    /// Taken guild names (lowercase for case-insensitive check).
    taken_names: HashSet<String>,
    next_id: u64,
}

impl PetitionManager {
    pub fn new() -> Self {
        Self {
            petitions: Vec::new(),
            taken_names: HashSet::new(),
            next_id: 1,
        }
    }

    /// Register an existing guild name as taken (loaded from DB).
    pub fn reserve_name(&mut self, name: &str) {
        self.taken_names.insert(name.to_lowercase());
    }

    /// Purchase a guild charter. Validates funds, guild membership, and name.
    pub fn purchase_charter(
        &mut self,
        player: u64,
        guild_name: &str,
        player_gold: u32,
        is_guilded: bool,
        now: u64,
    ) -> Result<u64, PetitionError> {
        self.validate_purchase(player, guild_name, player_gold, is_guilded)?;
        let id = self.next_id;
        self.next_id += 1;
        self.petitions.push(Petition {
            id,
            owner: player,
            guild_name: guild_name.to_string(),
            signatures: Vec::new(),
            created_at: now,
        });
        Ok(id)
    }

    fn validate_purchase(
        &self,
        player: u64,
        name: &str,
        gold: u32,
        is_guilded: bool,
    ) -> Result<(), PetitionError> {
        if is_guilded {
            return Err(PetitionError::AlreadyInGuild);
        }
        if gold < CHARTER_COST {
            return Err(PetitionError::InsufficientFunds);
        }
        if name.is_empty() || name.len() > MAX_GUILD_NAME_LEN {
            return Err(PetitionError::InvalidName);
        }
        if self.is_name_taken(name) {
            return Err(PetitionError::NameTaken);
        }
        if self.petitions.iter().any(|p| p.owner == player) {
            return Err(PetitionError::AlreadyHasPetition);
        }
        Ok(())
    }

    fn is_name_taken(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        self.taken_names.contains(&lower)
            || self
                .petitions
                .iter()
                .any(|p| p.guild_name.to_lowercase() == lower)
    }

    /// Look up a petition by ID.
    pub fn get(&self, id: u64) -> Option<&Petition> {
        self.petitions.iter().find(|p| p.id == id)
    }

    /// Find a player's pending petition.
    pub fn find_by_owner(&self, player: u64) -> Option<&Petition> {
        self.petitions.iter().find(|p| p.owner == player)
    }

    /// Sign a petition. Signer must not be guilded, not the owner,
    /// and not already signed.
    pub fn sign_petition(
        &mut self,
        petition_id: u64,
        signer: u64,
        signer_is_guilded: bool,
    ) -> Result<usize, PetitionError> {
        if signer_is_guilded {
            return Err(PetitionError::AlreadyInGuild);
        }
        let petition = self
            .petitions
            .iter_mut()
            .find(|p| p.id == petition_id)
            .ok_or(PetitionError::NotFound)?;
        if petition.owner == signer {
            return Err(PetitionError::NotOwner); // Can't sign your own
        }
        if petition.has_signed(signer) {
            return Err(PetitionError::AlreadySigned);
        }
        petition.signatures.push(signer);
        Ok(petition.signatures.len())
    }

    /// Remove a signature from a petition.
    pub fn unsign_petition(&mut self, petition_id: u64, signer: u64) -> Result<(), PetitionError> {
        let petition = self
            .petitions
            .iter_mut()
            .find(|p| p.id == petition_id)
            .ok_or(PetitionError::NotFound)?;
        if !petition.has_signed(signer) {
            return Err(PetitionError::NotFound);
        }
        petition.signatures.retain(|&s| s != signer);
        Ok(())
    }

    /// Cancel a petition (owner only).
    pub fn cancel(&mut self, id: u64, player: u64) -> Result<(), PetitionError> {
        let petition = self
            .petitions
            .iter()
            .find(|p| p.id == id)
            .ok_or(PetitionError::NotFound)?;
        if petition.owner != player {
            return Err(PetitionError::NotOwner);
        }
        self.petitions.retain(|p| p.id != id);
        Ok(())
    }

    /// Submit a signed petition to create the guild.
    ///
    /// Validates enough signatures, then returns the guild info.
    /// The petition is consumed and the guild name is reserved.
    pub fn submit_petition(
        &mut self,
        petition_id: u64,
        submitter: u64,
        guild_id: u32,
    ) -> Result<CreatedGuild, PetitionError> {
        let petition = self
            .petitions
            .iter()
            .find(|p| p.id == petition_id)
            .ok_or(PetitionError::NotFound)?;
        if petition.owner != submitter {
            return Err(PetitionError::NotOwner);
        }
        if !petition.has_enough_signatures() {
            return Err(PetitionError::NotEnoughSignatures);
        }
        let result = CreatedGuild {
            guild_id,
            guild_name: petition.guild_name.clone(),
            guild_master: petition.owner,
            founding_members: petition.signatures.clone(),
        };
        self.reserve_name(&result.guild_name);
        self.petitions.retain(|p| p.id != petition_id);
        Ok(result)
    }

    /// Number of pending petitions.
    pub fn len(&self) -> usize {
        self.petitions.len()
    }

    /// Whether there are no petitions.
    pub fn is_empty(&self) -> bool {
        self.petitions.is_empty()
    }
}

/// Result of successfully submitting a guild petition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedGuild {
    /// Assigned guild ID.
    pub guild_id: u32,
    /// Guild name.
    pub guild_name: String,
    /// Player who becomes Guild Master.
    pub guild_master: u64,
    /// Players who signed (initial members).
    pub founding_members: Vec<u64>,
}

// --- Guild tabard ---

/// Cost to register a guild tabard design (in copper). 10 gold = 100,000 copper.
pub const TABARD_COST: u32 = 100_000;

/// A guild tabard design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildTabardDesign {
    /// Emblem icon ID.
    pub icon: u8,
    /// Emblem icon color index.
    pub icon_color: u8,
    /// Border style ID.
    pub border: u8,
    /// Border color index.
    pub border_color: u8,
    /// Background color index.
    pub background_color: u8,
}

/// Why a tabard operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabardError {
    /// Player doesn't have enough gold.
    InsufficientFunds,
    /// Player is not the guild master.
    NotGuildMaster,
}

/// Validate and create a tabard design.
///
/// Only the guild master can register a tabard. Costs 10 gold.
/// Returns the design for the server to store on the guild.
pub fn register_tabard(
    design: GuildTabardDesign,
    player: u64,
    guild_master: u64,
    player_gold: u32,
) -> Result<GuildTabardDesign, TabardError> {
    if player != guild_master {
        return Err(TabardError::NotGuildMaster);
    }
    if player_gold < TABARD_COST {
        return Err(TabardError::InsufficientFunds);
    }
    Ok(design)
}

// --- Guild charter NPC ---

/// NPC flags indicating this vendor sells guild charters.
pub const NPC_FLAG_GUILD_CHARTER: u32 = 0x0200;

/// Maximum interaction distance with a charter NPC (yards).
pub const CHARTER_NPC_RANGE: f32 = 10.0;

/// Why an NPC charter interaction failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharterNpcError {
    /// NPC doesn't sell guild charters.
    NotCharterVendor,
    /// Player is too far from the NPC.
    OutOfRange,
    /// Charter purchase validation failed.
    Purchase(PetitionError),
}

/// Context for a charter NPC interaction.
pub struct CharterNpcContext<'a> {
    pub npc_flags: u32,
    pub distance_squared: f32,
    pub player: u64,
    pub guild_name: &'a str,
    pub player_gold: u32,
    pub is_guilded: bool,
    pub now: u64,
}

/// Attempt to purchase a guild charter from an NPC vendor.
///
/// Validates the NPC is a charter vendor, the player is in range,
/// then delegates to `PetitionManager::purchase_charter`.
pub fn buy_charter_from_npc(
    mgr: &mut PetitionManager,
    ctx: &CharterNpcContext<'_>,
) -> Result<u64, CharterNpcError> {
    if ctx.npc_flags & NPC_FLAG_GUILD_CHARTER == 0 {
        return Err(CharterNpcError::NotCharterVendor);
    }
    if ctx.distance_squared > CHARTER_NPC_RANGE * CHARTER_NPC_RANGE {
        return Err(CharterNpcError::OutOfRange);
    }
    mgr.purchase_charter(
        ctx.player,
        ctx.guild_name,
        ctx.player_gold,
        ctx.is_guilded,
        ctx.now,
    )
    .map_err(CharterNpcError::Purchase)
}

#[cfg(test)]
#[path = "guild_creation_tests.rs"]
mod tests;
