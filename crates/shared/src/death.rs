//! Corpse and death system.
//!
//! Handles player death, corpse placement, ghost form, resurrection.
//! Ref: AzerothCore `Corpse.cpp`, `Player.cpp` (RepopAtGraveyard).

use serde::{Deserialize, Serialize};

/// Player alive/dead state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeathState {
    /// Player is alive.
    Alive,
    /// Player has just died (HP reached 0). Corpse at death location.
    Dead,
    /// Player has released spirit and is a ghost at a graveyard.
    Ghost,
    /// Player is being resurrected (by spell or spirit healer).
    Resurrecting,
}

/// A player corpse left at the death location.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Corpse {
    /// Player entity who died.
    pub player: u64,
    /// Map where the corpse is.
    pub map_id: u16,
    /// Death position.
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Server timestamp when the player died.
    pub created_at: u64,
}

/// Result of a player dying.
#[derive(Debug, Clone, PartialEq)]
pub struct DeathResult {
    pub player: u64,
    pub corpse: Corpse,
}

/// Check if a player should die (HP reached 0).
pub fn check_death(current_hp: f32) -> bool {
    current_hp <= 0.0
}

/// Process a player death: create corpse at death location.
///
/// Returns the death result with the corpse data.
/// The server should:
/// 1. Set the player's `DeathState` to `Dead`
/// 2. Spawn the corpse entity at the death location
/// 3. Disable player interactions (combat, looting, trading, etc.)
pub fn on_death(player: u64, map_id: u16, x: f32, y: f32, z: f32, now: u64) -> DeathResult {
    DeathResult {
        player,
        corpse: Corpse {
            player,
            map_id,
            x,
            y,
            z,
            created_at: now,
        },
    }
}

/// Why a death operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathError {
    /// Player is not dead.
    NotDead,
    /// Player is not a ghost.
    NotGhost,
    /// Player is already alive.
    AlreadyAlive,
    /// Player is already dead.
    AlreadyDead,
}

/// Validate and transition death state.
pub fn transition_state(current: DeathState, to: DeathState) -> Result<DeathState, DeathError> {
    match (current, to) {
        (DeathState::Alive, DeathState::Dead) => Ok(DeathState::Dead),
        (DeathState::Dead, DeathState::Ghost) => Ok(DeathState::Ghost),
        (DeathState::Dead, DeathState::Resurrecting) => Ok(DeathState::Resurrecting),
        (DeathState::Ghost, DeathState::Resurrecting) => Ok(DeathState::Resurrecting),
        (DeathState::Resurrecting, DeathState::Alive) => Ok(DeathState::Alive),
        (DeathState::Alive, DeathState::Alive) => Err(DeathError::AlreadyAlive),
        (DeathState::Dead, DeathState::Dead) => Err(DeathError::AlreadyDead),
        (DeathState::Alive, _) => Err(DeathError::NotDead),
        (DeathState::Dead, DeathState::Alive) => Err(DeathError::NotGhost),
        _ => Err(DeathError::NotDead),
    }
}

// --- Release spirit / graveyards ---

/// A graveyard location where ghosts spawn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Graveyard {
    /// Unique graveyard ID.
    pub id: u32,
    /// Zone ID this graveyard serves.
    pub zone_id: u32,
    /// Faction: 0=both, 1=Alliance, 2=Horde.
    pub faction: u8,
    /// Map ID.
    pub map_id: u16,
    /// Ghost spawn position.
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Registry of graveyards, loaded from world data.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GraveyardRegistry {
    graveyards: Vec<Graveyard>,
}

impl GraveyardRegistry {
    pub fn new() -> Self {
        Self {
            graveyards: Vec::new(),
        }
    }

    pub fn add(&mut self, gy: Graveyard) {
        self.graveyards.push(gy);
    }

    /// Find the nearest graveyard for a given position and faction.
    /// `faction`: 0=neutral(any), 1=Alliance, 2=Horde.
    pub fn nearest(&self, map_id: u16, x: f32, y: f32, z: f32, faction: u8) -> Option<&Graveyard> {
        self.graveyards
            .iter()
            .filter(|g| g.map_id == map_id && (g.faction == 0 || g.faction == faction))
            .min_by(|a, b| {
                let da = dist_sq(x, y, z, a.x, a.y, a.z);
                let db = dist_sq(x, y, z, b.x, b.y, b.z);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn len(&self) -> usize {
        self.graveyards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.graveyards.is_empty()
    }
}

fn dist_sq(ax: f32, ay: f32, az: f32, bx: f32, by: f32, bz: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    let dz = az - bz;
    dx * dx + dy * dy + dz * dz
}

/// Result of releasing spirit.
#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseResult {
    /// Graveyard the ghost spawns at.
    pub graveyard_id: u32,
    pub map_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Release spirit: Dead → Ghost, teleport to nearest graveyard.
///
/// Returns the graveyard position for the server to teleport the player.
pub fn release_spirit(
    state: DeathState,
    death_map: u16,
    death_x: f32,
    death_y: f32,
    death_z: f32,
    player_faction: u8,
    graveyards: &GraveyardRegistry,
) -> Result<ReleaseResult, DeathError> {
    if state != DeathState::Dead {
        return Err(DeathError::NotDead);
    }
    let gy = graveyards
        .nearest(death_map, death_x, death_y, death_z, player_faction)
        .ok_or(DeathError::NotDead)?; // no graveyard = can't release
    Ok(ReleaseResult {
        graveyard_id: gy.id,
        map_id: gy.map_id,
        x: gy.x,
        y: gy.y,
        z: gy.z,
    })
}

// --- Ghost form ---

/// Actions that ghosts cannot perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhostRestriction {
    Combat,
    Loot,
    Trade,
    QuestInteract,
    UseObject,
    Mount,
    Cast,
    Chat,
}

/// Check if an action is blocked in ghost form.
pub fn is_ghost_restricted(state: DeathState, action: GhostRestriction) -> bool {
    if state != DeathState::Ghost {
        return false;
    }
    // All listed actions are restricted for ghosts
    matches!(
        action,
        GhostRestriction::Combat
            | GhostRestriction::Loot
            | GhostRestriction::Trade
            | GhostRestriction::QuestInteract
            | GhostRestriction::UseObject
            | GhostRestriction::Mount
            | GhostRestriction::Cast
            | GhostRestriction::Chat
    )
}

/// Visibility check: can observer see target?
///
/// - Ghosts are invisible to living players.
/// - Ghosts can see other ghosts.
/// - Living players can see other living players.
/// - Spirit healers (NPCs) can see ghosts.
pub fn ghost_visible(
    observer_state: DeathState,
    target_state: DeathState,
    target_is_spirit_healer: bool,
) -> bool {
    match (observer_state, target_state) {
        // Living sees living
        (DeathState::Alive, DeathState::Alive) => true,
        // Living cannot see ghosts
        (DeathState::Alive, DeathState::Ghost) => false,
        // Ghost sees ghosts
        (DeathState::Ghost, DeathState::Ghost) => true,
        // Ghost sees living (can navigate the world)
        (DeathState::Ghost, DeathState::Alive) => true,
        // Spirit healer is always visible to ghosts
        _ if target_is_spirit_healer => true,
        // Default: visible
        _ => true,
    }
}

// --- Corpse run ---

/// Maximum distance (yards) from corpse to resurrect.
pub const CORPSE_RESURRECT_RANGE: f32 = 30.0;

/// Check if a ghost player is close enough to their corpse to resurrect.
pub fn in_corpse_range(player_x: f32, player_y: f32, player_z: f32, corpse: &Corpse) -> bool {
    dist_sq(player_x, player_y, player_z, corpse.x, corpse.y, corpse.z)
        <= CORPSE_RESURRECT_RANGE * CORPSE_RESURRECT_RANGE
}

/// Result of a corpse-run resurrection.
#[derive(Debug, Clone, PartialEq)]
pub struct CorpseResurrectResult {
    /// Position to resurrect at (corpse location).
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub map_id: u16,
}

/// Attempt to resurrect at the player's corpse.
///
/// Ghost must be within 30 yards of the corpse on the same map.
/// Transitions Ghost → Resurrecting (server then completes → Alive).
pub fn resurrect_at_corpse(
    state: DeathState,
    player_x: f32,
    player_y: f32,
    player_z: f32,
    player_map: u16,
    corpse: &Corpse,
) -> Result<CorpseResurrectResult, DeathError> {
    if state != DeathState::Ghost {
        return Err(DeathError::NotGhost);
    }
    if player_map != corpse.map_id {
        return Err(DeathError::NotGhost);
    }
    if !in_corpse_range(player_x, player_y, player_z, corpse) {
        return Err(DeathError::NotGhost);
    }
    Ok(CorpseResurrectResult {
        x: corpse.x,
        y: corpse.y,
        z: corpse.z,
        map_id: corpse.map_id,
    })
}

// --- Resurrection sickness ---

/// Duration of resurrection sickness in seconds (10 minutes).
pub const RES_SICKNESS_DURATION: u64 = 600;

/// Stat/damage reduction while resurrection sickness is active (75% reduction).
pub const RES_SICKNESS_PENALTY: f32 = 0.25;

/// Minimum player level to receive resurrection sickness.
/// Below this level, no sickness is applied (retail: level 10).
pub const RES_SICKNESS_MIN_LEVEL: u8 = 10;

/// How a player was resurrected — determines if sickness applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResurrectSource {
    /// Walked back to corpse — no sickness.
    CorpseRun,
    /// Used spirit healer NPC — sickness applies.
    SpiritHealer,
    /// Resurrected by another player's spell — no sickness.
    PlayerSpell,
}

/// Resurrection sickness debuff state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResSickness {
    /// Timestamp when the debuff expires.
    pub expires_at: u64,
}

impl ResSickness {
    /// Whether the debuff is still active.
    pub fn is_active(&self, now: u64) -> bool {
        now < self.expires_at
    }

    /// Seconds remaining.
    pub fn remaining(&self, now: u64) -> u64 {
        self.expires_at.saturating_sub(now)
    }

    /// Apply the stat penalty to a base value.
    pub fn apply_penalty(&self, base: f32, now: u64) -> f32 {
        if self.is_active(now) {
            base * RES_SICKNESS_PENALTY
        } else {
            base
        }
    }
}

/// Determine if resurrection sickness should be applied.
pub fn should_apply_sickness(source: ResurrectSource, player_level: u8) -> bool {
    source == ResurrectSource::SpiritHealer && player_level >= RES_SICKNESS_MIN_LEVEL
}

/// Create a resurrection sickness debuff if applicable.
pub fn apply_res_sickness(
    source: ResurrectSource,
    player_level: u8,
    now: u64,
) -> Option<ResSickness> {
    if should_apply_sickness(source, player_level) {
        Some(ResSickness {
            expires_at: now + RES_SICKNESS_DURATION,
        })
    } else {
        None
    }
}

// --- Spirit healer ---

/// NPC flag identifying a spirit healer.
pub const NPC_FLAG_SPIRIT_HEALER: u32 = 0x0800;

/// Maximum distance to interact with a spirit healer (yards).
pub const SPIRIT_HEALER_RANGE: f32 = 20.0;

/// Result of accepting a spirit healer resurrection.
#[derive(Debug, Clone, PartialEq)]
pub struct SpiritHealerResult {
    /// Position to resurrect at (graveyard / spirit healer location).
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub map_id: u16,
    /// Resurrection sickness debuff (None if low level).
    pub sickness: Option<ResSickness>,
}

/// Context for a spirit healer interaction.
pub struct SpiritHealerContext {
    pub state: DeathState,
    pub player_level: u8,
    pub player_pos: (f32, f32, f32),
    pub healer_pos: (f32, f32, f32),
    pub healer_map: u16,
    pub now: u64,
}

/// Accept resurrection from a spirit healer NPC.
///
/// Player must be a Ghost within range of the spirit healer.
/// Applies resurrection sickness for players at level 10+.
pub fn accept_spirit_healer(ctx: &SpiritHealerContext) -> Result<SpiritHealerResult, DeathError> {
    if ctx.state != DeathState::Ghost {
        return Err(DeathError::NotGhost);
    }
    let range_sq = dist_sq(
        ctx.player_pos.0,
        ctx.player_pos.1,
        ctx.player_pos.2,
        ctx.healer_pos.0,
        ctx.healer_pos.1,
        ctx.healer_pos.2,
    );
    if range_sq > SPIRIT_HEALER_RANGE * SPIRIT_HEALER_RANGE {
        return Err(DeathError::NotGhost);
    }
    let sickness = apply_res_sickness(ResurrectSource::SpiritHealer, ctx.player_level, ctx.now);
    Ok(SpiritHealerResult {
        x: ctx.healer_pos.0,
        y: ctx.healer_pos.1,
        z: ctx.healer_pos.2,
        map_id: ctx.healer_map,
        sickness,
    })
}

// --- Durability loss ---

/// Durability loss on death (10% of max durability).
pub const DEATH_DURABILITY_LOSS: f32 = 0.10;
/// Additional durability loss when using spirit healer (25% of max).
pub const SPIRIT_HEALER_DURABILITY_LOSS: f32 = 0.25;

/// Durability state for a single equipment slot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Durability {
    pub current: u32,
    pub max: u32,
}

impl Durability {
    /// Apply a percentage loss to max durability.
    pub fn apply_loss(&mut self, fraction: f32) {
        let loss = (self.max as f32 * fraction) as u32;
        self.current = self.current.saturating_sub(loss);
    }

    /// Whether the item is broken (0 durability).
    pub fn is_broken(&self) -> bool {
        self.current == 0 && self.max > 0
    }
}

/// Apply a fractional durability loss to all items, returning total loss.
fn apply_durability_loss(items: &mut [Durability], fraction: f32) -> u32 {
    let mut total = 0;
    for item in items.iter_mut() {
        let loss = (item.max as f32 * fraction) as u32;
        item.current = item.current.saturating_sub(loss);
        total += loss;
    }
    total
}

/// Calculate durability loss for all equipped items on death.
///
/// Returns the total loss applied (sum of losses across all slots).
pub fn apply_death_durability_loss(items: &mut [Durability]) -> u32 {
    apply_durability_loss(items, DEATH_DURABILITY_LOSS)
}

/// Calculate additional durability loss for spirit healer resurrection.
///
/// Applied on top of the death loss. Returns the additional loss.
pub fn apply_spirit_healer_durability_loss(items: &mut [Durability]) -> u32 {
    apply_durability_loss(items, SPIRIT_HEALER_DURABILITY_LOSS)
}

// --- Player resurrect spells ---

/// Maximum range for a player resurrect spell (yards).
pub const RESURRECT_SPELL_RANGE: f32 = 40.0;

/// Percentage of max HP restored by a resurrect spell.
pub const RESURRECT_HP_PERCENT: f32 = 0.35;

/// Percentage of max mana restored by a resurrect spell.
pub const RESURRECT_MANA_PERCENT: f32 = 0.35;

/// Result of a player casting a resurrect spell on a dead player.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerResurrectResult {
    /// HP to restore (35% of max).
    pub restored_hp: f32,
    /// Mana to restore (35% of max).
    pub restored_mana: f32,
    /// Position to resurrect at (caster or target location).
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Cast a resurrect spell on a dead or ghost player.
///
/// - No resurrection sickness.
/// - No additional durability loss.
/// - Target must be Dead or Ghost.
/// - Caster must be within range.
pub fn cast_resurrect(
    target_state: DeathState,
    target_max_hp: f32,
    target_max_mana: f32,
    target_pos: (f32, f32, f32),
    caster_pos: (f32, f32, f32),
) -> Result<PlayerResurrectResult, DeathError> {
    if !matches!(target_state, DeathState::Dead | DeathState::Ghost) {
        return Err(DeathError::AlreadyAlive);
    }
    let range_sq = dist_sq(
        caster_pos.0,
        caster_pos.1,
        caster_pos.2,
        target_pos.0,
        target_pos.1,
        target_pos.2,
    );
    if range_sq > RESURRECT_SPELL_RANGE * RESURRECT_SPELL_RANGE {
        return Err(DeathError::NotDead);
    }
    Ok(PlayerResurrectResult {
        restored_hp: target_max_hp * RESURRECT_HP_PERCENT,
        restored_mana: target_max_mana * RESURRECT_MANA_PERCENT,
        x: target_pos.0,
        y: target_pos.1,
        z: target_pos.2,
    })
}

// --- Corpse despawn ---

/// Time in seconds before a corpse despawns after the player resurrects or releases.
pub const CORPSE_DESPAWN_SECS: u64 = 300;

/// A corpse awaiting despawn.
#[derive(Debug, Clone, PartialEq)]
pub struct CorpseDespawnTimer {
    pub player: u64,
    pub despawn_at: u64,
}

/// Tracks corpses pending despawn.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CorpseDespawnTracker {
    pending: Vec<CorpseDespawnTimer>,
}

impl CorpseDespawnTracker {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    /// Start the despawn timer for a corpse (called on resurrect or release).
    pub fn start_timer(&mut self, player: u64, now: u64) {
        self.pending.retain(|c| c.player != player);
        self.pending.push(CorpseDespawnTimer {
            player,
            despawn_at: now + CORPSE_DESPAWN_SECS,
        });
    }

    /// Collect all corpses whose despawn timer has elapsed.
    pub fn collect_despawns(&mut self, now: u64) -> Vec<u64> {
        let ready: Vec<u64> = self
            .pending
            .iter()
            .filter(|c| now >= c.despawn_at)
            .map(|c| c.player)
            .collect();
        self.pending.retain(|c| now < c.despawn_at);
        ready
    }

    /// Whether a player's corpse is pending despawn.
    pub fn is_pending(&self, player: u64) -> bool {
        self.pending.iter().any(|c| c.player == player)
    }

    /// Cancel a pending despawn (e.g. player dies again before corpse despawns).
    pub fn cancel(&mut self, player: u64) {
        self.pending.retain(|c| c.player != player);
    }

    /// Number of corpses pending despawn.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
#[path = "death_tests.rs"]
mod tests;
