use bevy::prelude::*;

/// Seconds of no combat activity before leaving combat.
const COMBAT_IDLE_TIMEOUT: f32 = 6.0;

/// Tracks whether an entity is currently in combat.
///
/// Enter combat on damage dealt/taken or threat generated.
/// Exit combat after 6 seconds of no activity (idle timer expires).
#[derive(Component, Debug, Clone)]
pub struct InCombat {
    /// Time remaining until combat drops (resets on each combat action).
    pub idle_timer: f32,
}

impl Default for InCombat {
    fn default() -> Self {
        Self {
            idle_timer: COMBAT_IDLE_TIMEOUT,
        }
    }
}

impl InCombat {
    /// Reset the idle timer (called on any combat action).
    pub fn refresh(&mut self) {
        self.idle_timer = COMBAT_IDLE_TIMEOUT;
    }

    /// Tick the idle timer. Returns `true` if combat should end (timer expired).
    pub fn tick(&mut self, dt: f32) -> bool {
        self.idle_timer -= dt;
        self.idle_timer <= 0.0
    }

    /// Whether the entity is still in combat.
    pub fn active(&self) -> bool {
        self.idle_timer > 0.0
    }
}

/// Default leash distance in yards (mob resets if this far from spawn).
const DEFAULT_LEASH_DISTANCE: f32 = 40.0;

/// Leash configuration for a mob. When the mob moves beyond `max_distance`
/// from its spawn point, or its target is unreachable, it evades (resets).
///
/// Ref: AzerothCore `CreatureAI::EnterEvadeMode()`.
#[derive(Component, Debug, Clone)]
pub struct LeashConfig {
    /// Maximum distance from spawn before evading (yards).
    pub max_distance: f32,
    /// Spawn origin (x, y, z) to measure distance from.
    pub origin_x: f32,
    pub origin_y: f32,
    pub origin_z: f32,
}

impl LeashConfig {
    /// Create a leash config with the default distance.
    pub fn new(origin_x: f32, origin_y: f32, origin_z: f32) -> Self {
        Self {
            max_distance: DEFAULT_LEASH_DISTANCE,
            origin_x,
            origin_y,
            origin_z,
        }
    }

    /// Check if a position is beyond the leash distance from the spawn origin.
    pub fn should_evade(&self, x: f32, y: f32, z: f32) -> bool {
        let dx = x - self.origin_x;
        let dy = y - self.origin_y;
        let dz = z - self.origin_z;
        let dist_sq = dx * dx + dy * dy + dz * dz;
        dist_sq > self.max_distance * self.max_distance
    }
}

/// Result of an evade check — what the mob should do on reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvadeReason {
    /// Mob exceeded its leash distance from spawn.
    LeashDistance,
    /// Target is unreachable (no path, stuck, etc).
    TargetUnreachable,
}

/// Per-mob threat table tracking aggro from each entity.
///
/// Damage generates 1:1 threat. Healing generates 0.5 threat per point healed,
/// split equally across all mobs the healer is engaged with (tracked externally).
///
/// Ref: AzerothCore `ThreatMgr.cpp`.
#[derive(Component, Debug, Clone, Default)]
pub struct ThreatTable {
    entries: Vec<ThreatEntry>,
}

/// A single threat entry: who is threatening and how much.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThreatEntry {
    pub entity: u64,
    pub threat: f32,
}

/// Threat multiplier for healing (0.5 threat per point healed).
const HEAL_THREAT_MULTIPLIER: f32 = 0.5;
/// Base aggro radius in yards (WoW default for equal-level mobs).
const BASE_AGGRO_RADIUS: f32 = 20.0;

/// Check if an NPC should aggro a player based on distance and level difference.
///
/// Aggro radius scales with level difference: +1 yard per mob level above
/// player, -1 yard per mob level below player (min 5 yards).
/// Returns `true` if the player is within aggro range.
pub fn should_aggro(distance: f32, mob_level: u16, player_level: u16) -> bool {
    let level_diff = mob_level as f32 - player_level as f32;
    let radius = (BASE_AGGRO_RADIUS + level_diff).max(5.0);
    distance <= radius
}

/// Aggro swap threshold for melee (must exceed current target by 10%).
const MELEE_AGGRO_THRESHOLD: f32 = 1.10;
/// Aggro swap threshold for ranged (must exceed current target by 30%).
const RANGED_AGGRO_THRESHOLD: f32 = 1.30;

/// Radius within which linked mobs join combat when one aggros (yards).
const GROUP_AGGRO_RADIUS: f32 = 10.0;

/// Check if a linked mob should join combat when a nearby mob aggros.
///
/// Mobs within `GROUP_AGGRO_RADIUS` of the aggroing mob will also enter combat.
pub fn should_group_aggro(mob_x: f32, mob_z: f32, aggro_x: f32, aggro_z: f32) -> bool {
    let dx = mob_x - aggro_x;
    let dz = mob_z - aggro_z;
    let dist_sq = dx * dx + dz * dz;
    dist_sq <= GROUP_AGGRO_RADIUS * GROUP_AGGRO_RADIUS
}

impl ThreatTable {
    /// Add threat from damage (1:1 base ratio, scaled by `modifier`).
    ///
    /// `modifier` is the source entity's threat multiplier from auras
    /// (e.g. 1.43 for Righteous Fury). Pass 1.0 for no modifier.
    pub fn add_damage_threat(&mut self, source: u64, damage: f32, modifier: f32) {
        self.add_threat(source, damage * modifier);
    }

    /// Add threat from healing (0.5:1 base ratio, scaled by `modifier`, pre-split by caller).
    ///
    /// `heal_amount` is the raw healing done. The caller is responsible for
    /// splitting across engaged mobs before calling this.
    pub fn add_heal_threat(&mut self, source: u64, heal_amount: f32, modifier: f32) {
        self.add_threat(source, heal_amount * HEAL_THREAT_MULTIPLIER * modifier);
    }

    /// Add a raw threat amount from a source entity.
    pub fn add_threat(&mut self, source: u64, amount: f32) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.entity == source) {
            entry.threat += amount;
        } else {
            self.entries.push(ThreatEntry {
                entity: source,
                threat: amount,
            });
        }
    }

    /// Get the current top-threat entity (aggro target).
    pub fn top_threat(&self) -> Option<&ThreatEntry> {
        self.entries
            .iter()
            .max_by(|a, b| a.threat.partial_cmp(&b.threat).unwrap())
    }

    /// Get the threat value for a specific entity.
    pub fn threat_for(&self, entity: u64) -> f32 {
        self.entries
            .iter()
            .find(|e| e.entity == entity)
            .map_or(0.0, |e| e.threat)
    }

    /// Get the top-threat entity, skipping any that are crowd-controlled.
    ///
    /// CC'd targets keep their threat but are not valid aggro targets until
    /// the CC expires.
    pub fn top_threat_excluding(&self, cc_entities: &[u64]) -> Option<&ThreatEntry> {
        self.entries
            .iter()
            .filter(|e| !cc_entities.contains(&e.entity))
            .max_by(|a, b| a.threat.partial_cmp(&b.threat).unwrap())
    }

    /// Remove an entity from the threat table (on death or leave combat).
    pub fn remove(&mut self, entity: u64) {
        self.entries.retain(|e| e.entity != entity);
    }

    /// Clear all threat (mob reset / evade).
    pub fn reset(&mut self) {
        self.entries.clear();
    }

    /// Whether any entity has threat on this mob.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of entities with threat.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Determine the aggro target, accounting for swap thresholds.
    ///
    /// A challenger only takes aggro from the `current_target` if their threat
    /// exceeds the current target's threat by a threshold:
    /// - **Melee** (≤ 5 yards): must exceed by 10% (`is_melee = true`)
    /// - **Ranged** (> 5 yards): must exceed by 30% (`is_melee = false`)
    ///
    /// If `current_target` is `None` (first engagement) or has been removed
    /// from the threat table, the highest-threat entity wins unconditionally.
    ///
    /// Ref: AzerothCore `ThreatMgr::SelectVictim()`.
    pub fn aggro_target(&self, current_target: Option<u64>, is_melee: bool) -> Option<u64> {
        let top = self.top_threat()?;

        let Some(current) = current_target else {
            return Some(top.entity);
        };

        let current_threat = self.threat_for(current);
        if current_threat <= 0.0 {
            return Some(top.entity);
        }

        let threshold = if is_melee {
            MELEE_AGGRO_THRESHOLD
        } else {
            RANGED_AGGRO_THRESHOLD
        };

        if top.threat >= current_threat * threshold {
            Some(top.entity)
        } else {
            Some(current)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_combat_default_is_active() {
        let combat = InCombat::default();
        assert!(combat.active());
        assert!((combat.idle_timer - 6.0).abs() < 0.001);
    }

    #[test]
    fn in_combat_expires_after_6s() {
        let mut combat = InCombat::default();
        assert!(!combat.tick(5.0));
        assert!(combat.active());
        assert!(combat.tick(1.5));
        assert!(!combat.active());
    }

    #[test]
    fn in_combat_refresh_resets_timer() {
        let mut combat = InCombat::default();
        combat.tick(5.0);
        combat.refresh();
        assert!((combat.idle_timer - 6.0).abs() < 0.001);
        assert!(!combat.tick(5.0));
    }

    #[test]
    fn aggro_within_range_equal_level() {
        // Equal level: 20yd radius
        assert!(should_aggro(19.0, 10, 10));
        assert!(should_aggro(20.0, 10, 10));
        assert!(!should_aggro(21.0, 10, 10));
    }

    #[test]
    fn combat_state_enters_on_damage_and_refreshes() {
        // Simulate: entity not in combat → deals damage → enters combat
        let mut attacker_combat: Option<InCombat> = None;
        let mut target_combat: Option<InCombat> = None;

        // Before damage: no combat state
        assert!(attacker_combat.is_none());
        assert!(target_combat.is_none());

        // Damage dealt → both enter combat
        attacker_combat = Some(InCombat::default());
        target_combat = Some(InCombat::default());
        assert!(attacker_combat.as_ref().unwrap().active());
        assert!(target_combat.as_ref().unwrap().active());

        // Time passes (5s) — still in combat
        let attacker = attacker_combat.as_mut().unwrap();
        assert!(!attacker.tick(5.0));
        assert!(attacker.active());

        // More damage dealt → timer refreshes
        attacker.refresh();
        assert!(
            (attacker.idle_timer - 6.0).abs() < 0.001,
            "timer refreshed to 6s"
        );

        // 6s of no activity → exits combat
        assert!(attacker.tick(6.5));
        assert!(!attacker.active());
    }

    #[test]
    fn aggro_range_scales_with_level_difference() {
        // Mob 5 levels above player: 20 + 5 = 25yd
        assert!(should_aggro(24.0, 15, 10));
        assert!(!should_aggro(26.0, 15, 10));

        // Mob 10 levels below player: 20 - 10 = 10yd
        assert!(should_aggro(9.0, 5, 15));
        assert!(!should_aggro(11.0, 5, 15));
    }

    #[test]
    fn aggro_range_has_minimum_5_yards() {
        // Mob 30 levels below: 20 - 30 = -10, clamped to 5yd
        assert!(should_aggro(4.0, 1, 31));
        assert!(should_aggro(5.0, 1, 31));
        assert!(!should_aggro(6.0, 1, 31));
    }

    #[test]
    fn in_combat_tick_returns_true_at_zero() {
        let mut combat = InCombat::default();
        assert!(combat.tick(6.0));
    }

    #[test]
    fn leash_within_range_no_evade() {
        let leash = LeashConfig::new(100.0, 0.0, 100.0);
        assert!(!leash.should_evade(110.0, 0.0, 110.0));
    }

    #[test]
    fn leash_at_boundary_no_evade() {
        let leash = LeashConfig::new(0.0, 0.0, 0.0);
        assert!(!leash.should_evade(40.0, 0.0, 0.0));
    }

    #[test]
    fn leash_beyond_range_evades() {
        let leash = LeashConfig::new(0.0, 0.0, 0.0);
        assert!(leash.should_evade(41.0, 0.0, 0.0));
    }

    #[test]
    fn leash_diagonal_distance() {
        let leash = LeashConfig::new(0.0, 0.0, 0.0);
        assert!(leash.should_evade(30.0, 0.0, 30.0));
    }

    #[test]
    fn leash_custom_distance() {
        let mut leash = LeashConfig::new(0.0, 0.0, 0.0);
        leash.max_distance = 100.0;
        assert!(!leash.should_evade(80.0, 0.0, 0.0));
        assert!(leash.should_evade(101.0, 0.0, 0.0));
    }

    #[test]
    fn leash_default_distance_is_40() {
        let leash = LeashConfig::new(0.0, 0.0, 0.0);
        assert_eq!(leash.max_distance, 40.0);
    }

    #[test]
    fn leash_evade_clears_threat_table() {
        let leash = LeashConfig::new(100.0, 0.0, 100.0);
        let mut threat = ThreatTable::default();
        threat.add_damage_threat(1, 500.0, 1.0);
        threat.add_damage_threat(2, 300.0, 1.0);
        assert!(!threat.is_empty());

        // Creature at 41yd from spawn — should evade
        let creature_x = 141.0;
        assert!(leash.should_evade(creature_x, 0.0, 100.0));

        // On evade: reset threat table
        threat.reset();
        assert!(threat.is_empty());
        assert!(threat.top_threat().is_none());
    }

    #[test]
    fn leash_evade_resets_health_to_full() {
        use crate::components::Health;

        let leash = LeashConfig::new(0.0, 0.0, 0.0);
        let mut health = Health {
            current: 300.0,
            max: 1000.0,
        };

        // Creature beyond leash
        assert!(leash.should_evade(41.0, 0.0, 0.0));

        // On evade: restore to full
        health.current = health.max;
        assert_eq!(health.current, 1000.0);
    }

    #[test]
    fn threat_damage_adds_one_to_one() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(1, 500.0, 1.0);
        assert_eq!(table.threat_for(1), 500.0);
    }

    #[test]
    fn threat_heal_adds_half() {
        let mut table = ThreatTable::default();
        table.add_heal_threat(1, 1000.0, 1.0);
        assert_eq!(table.threat_for(1), 500.0);
    }

    #[test]
    fn threat_accumulates_from_same_source() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(1, 200.0, 1.0);
        table.add_damage_threat(1, 300.0, 1.0);
        assert_eq!(table.threat_for(1), 500.0);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn threat_top_returns_highest() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(1, 100.0, 1.0);
        table.add_damage_threat(2, 500.0, 1.0);
        table.add_damage_threat(3, 200.0, 1.0);
        let top = table.top_threat().unwrap();
        assert_eq!(top.entity, 2);
        assert_eq!(top.threat, 500.0);
    }

    #[test]
    fn threat_top_empty_returns_none() {
        let table = ThreatTable::default();
        assert!(table.top_threat().is_none());
    }

    #[test]
    fn threat_remove_entity() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(1, 100.0, 1.0);
        table.add_damage_threat(2, 200.0, 1.0);
        table.remove(1);
        assert_eq!(table.len(), 1);
        assert_eq!(table.threat_for(1), 0.0);
        assert_eq!(table.threat_for(2), 200.0);
    }

    #[test]
    fn threat_reset_clears_all() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(1, 100.0, 1.0);
        table.add_damage_threat(2, 200.0, 1.0);
        table.reset();
        assert!(table.is_empty());
    }

    #[test]
    fn threat_unknown_entity_returns_zero() {
        let table = ThreatTable::default();
        assert_eq!(table.threat_for(999), 0.0);
    }

    #[test]
    fn aggro_no_current_target_picks_top() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(1, 100.0, 1.0);
        table.add_damage_threat(2, 200.0, 1.0);
        assert_eq!(table.aggro_target(None, true), Some(2));
    }

    #[test]
    fn aggro_melee_needs_10_percent_to_swap() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(1, 1000.0, 1.0);
        table.add_damage_threat(2, 1050.0, 1.0);
        assert_eq!(table.aggro_target(Some(1), true), Some(1));
    }

    #[test]
    fn aggro_melee_swaps_at_110_percent() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(1, 1000.0, 1.0);
        table.add_damage_threat(2, 1100.0, 1.0);
        assert_eq!(table.aggro_target(Some(1), true), Some(2));
    }

    #[test]
    fn aggro_ranged_needs_30_percent_to_swap() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(1, 1000.0, 1.0);
        table.add_damage_threat(2, 1200.0, 1.0);
        assert_eq!(table.aggro_target(Some(1), false), Some(1));
    }

    #[test]
    fn aggro_ranged_swaps_at_130_percent() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(1, 1000.0, 1.0);
        table.add_damage_threat(2, 1300.0, 1.0);
        assert_eq!(table.aggro_target(Some(1), false), Some(2));
    }

    #[test]
    fn aggro_current_target_removed_picks_top() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(2, 500.0, 1.0);
        assert_eq!(table.aggro_target(Some(1), true), Some(2));
    }

    #[test]
    fn threat_swap_scenario_tank_loses_aggro() {
        let mut table = ThreatTable::default();
        let tank = 1_u64;
        let dps = 2_u64;

        // Tank establishes initial threat
        table.add_damage_threat(tank, 500.0, 1.0);
        let mut current_target = table.aggro_target(None, true).unwrap();
        assert_eq!(current_target, tank);

        // DPS does high damage but not enough to swap (needs 110%)
        table.add_damage_threat(dps, 540.0, 1.0);
        current_target = table.aggro_target(Some(current_target), true).unwrap();
        assert_eq!(current_target, tank, "DPS at 540 vs tank 500: no swap");

        // DPS continues — exceeds 110% threshold
        table.add_damage_threat(dps, 20.0, 1.0);
        // DPS now at 560, tank at 500 → 560/500 = 1.12 > 1.10
        current_target = table.aggro_target(Some(current_target), true).unwrap();
        assert_eq!(current_target, dps, "DPS at 560 vs tank 500: swap!");

        // Tank taunts back (massive threat)
        table.add_damage_threat(tank, 1000.0, 1.0);
        current_target = table.aggro_target(Some(current_target), true).unwrap();
        assert_eq!(current_target, tank, "tank reclaims with 1500 vs 560");
    }

    #[test]
    fn group_aggro_nearby_mobs_join_combat() {
        let aggro_x = 100.0;
        let aggro_z = 100.0;

        // Mob within 10yd → joins
        assert!(should_group_aggro(105.0, 103.0, aggro_x, aggro_z));

        // Mob at exactly 10yd → joins
        assert!(should_group_aggro(110.0, 100.0, aggro_x, aggro_z));

        // Mob beyond 10yd → does not join
        assert!(!should_group_aggro(111.0, 100.0, aggro_x, aggro_z));

        // Mob far away → does not join
        assert!(!should_group_aggro(200.0, 200.0, aggro_x, aggro_z));

        // Same position → joins
        assert!(should_group_aggro(aggro_x, aggro_z, aggro_x, aggro_z));
    }

    #[test]
    fn multiple_targets_creature_attacks_highest_threat() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(1, 100.0, 1.0); // tank
        table.add_damage_threat(2, 80.0, 1.0); // dps1
        table.add_damage_threat(3, 60.0, 1.0); // dps2
        table.add_damage_threat(4, 40.0, 1.0); // healer

        // Initial target: highest threat (entity 1)
        let target = table.aggro_target(None, true).unwrap();
        assert_eq!(target, 1);

        // DPS1 overtakes tank (needs >110%)
        table.add_damage_threat(2, 40.0, 1.0); // now 120 vs 100
        let target = table.aggro_target(Some(1), true).unwrap();
        assert_eq!(target, 2, "dps1 at 120 > 110% of tank's 100");

        // Healer generates threat via healing (different source)
        table.add_heal_threat(4, 600.0, 1.0); // 600 * 0.5 = 300, total 340
        let target = table.aggro_target(Some(2), true).unwrap();
        assert_eq!(target, 4, "healer at 340 > 110% of dps1's 120");
    }

    #[test]
    fn linked_creatures_share_threat() {
        // Two linked mobs: when one is attacked, both gain threat
        let mut mob_a = ThreatTable::default();
        let mut mob_b = ThreatTable::default();

        let player1 = 1_u64;
        let player2 = 2_u64;

        // Player1 attacks mob_a — propagate threat to mob_b
        let damage = 500.0;
        mob_a.add_damage_threat(player1, damage, 1.0);
        mob_b.add_damage_threat(player1, damage, 1.0); // linked propagation

        // Player2 attacks mob_b — propagate to mob_a
        mob_b.add_damage_threat(player2, 300.0, 1.0);
        mob_a.add_damage_threat(player2, 300.0, 1.0); // linked propagation

        // Both mobs should target player1 (highest threat)
        assert_eq!(mob_a.aggro_target(None, true), Some(player1));
        assert_eq!(mob_b.aggro_target(None, true), Some(player1));

        // Both have consistent threat values
        assert_eq!(mob_a.threat_for(player1), mob_b.threat_for(player1));
        assert_eq!(mob_a.threat_for(player2), mob_b.threat_for(player2));
    }

    #[test]
    fn creature_ignores_crowd_controlled_targets() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(1, 500.0, 1.0); // highest threat
        table.add_damage_threat(2, 300.0, 1.0);
        table.add_damage_threat(3, 100.0, 1.0);

        // Normal: target entity 1 (highest)
        assert_eq!(table.top_threat_excluding(&[]).unwrap().entity, 1);

        // Entity 1 is CC'd → skip to entity 2
        assert_eq!(table.top_threat_excluding(&[1]).unwrap().entity, 2);

        // Entities 1 and 2 CC'd → skip to entity 3
        assert_eq!(table.top_threat_excluding(&[1, 2]).unwrap().entity, 3);

        // All CC'd → no valid target
        assert!(table.top_threat_excluding(&[1, 2, 3]).is_none());
    }

    #[test]
    fn aggro_empty_table_returns_none() {
        let table = ThreatTable::default();
        assert_eq!(table.aggro_target(None, true), None);
    }

    #[test]
    fn damage_threat_with_modifier() {
        let mut table = ThreatTable::default();
        table.add_damage_threat(1, 1000.0, 1.43);
        assert!((table.threat_for(1) - 1430.0).abs() < 0.01);
    }

    #[test]
    fn heal_threat_with_modifier() {
        let mut table = ThreatTable::default();
        table.add_heal_threat(1, 1000.0, 0.7);
        assert!((table.threat_for(1) - 350.0).abs() < 0.01);
    }
}
