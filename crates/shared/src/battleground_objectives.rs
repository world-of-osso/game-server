//! BG objective types: flag capture (WSG), node control (AB), reinforcements (AV).

use super::{BgInstance, BgTeam, BgTemplate};

// -- Objective: Flag Capture (WSG) --

/// State of a single flag in capture-the-flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagState {
    /// Flag is at its home base.
    AtBase,
    /// Flag is being carried by a player.
    Carried { carrier: u64 },
    /// Flag was dropped and is on the ground (returns to base after timeout).
    Dropped,
}

/// Flag capture state for both teams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagObjective {
    /// Team A's flag (defended by A, captured by B).
    pub flag_a: FlagState,
    /// Team B's flag (defended by B, captured by A).
    pub flag_b: FlagState,
}

impl Default for FlagObjective {
    fn default() -> Self {
        Self {
            flag_a: FlagState::AtBase,
            flag_b: FlagState::AtBase,
        }
    }
}

impl FlagObjective {
    /// A player picks up the enemy flag. Returns false if the flag isn't available.
    pub fn pickup(&mut self, team: BgTeam, carrier: u64) -> bool {
        let flag = self.enemy_flag_mut(team);
        let available = matches!(flag, FlagState::AtBase | FlagState::Dropped);
        if available {
            *flag = FlagState::Carried { carrier };
        }
        available
    }

    /// The flag carrier drops the flag (death or disconnect).
    pub fn drop_flag(&mut self, team: BgTeam) {
        let flag = self.enemy_flag_mut(team);
        if matches!(flag, FlagState::Carried { .. }) {
            *flag = FlagState::Dropped;
        }
    }

    /// A defender returns their team's dropped flag to base.
    pub fn return_flag(&mut self, team: BgTeam) -> bool {
        let flag = self.own_flag_mut(team);
        if *flag == FlagState::Dropped {
            *flag = FlagState::AtBase;
            return true;
        }
        false
    }

    /// Carrier reaches their own base with enemy flag. Awards a capture if
    /// the carrier's own flag is at base. Returns true if captured.
    pub fn capture(&mut self, team: BgTeam, instance: &mut BgInstance, tmpl: &BgTemplate) -> bool {
        let own_flag_at_base = *self.own_flag(team) == FlagState::AtBase;
        let enemy_flag = self.enemy_flag_mut(team);
        let is_carried = matches!(enemy_flag, FlagState::Carried { .. });
        if !is_carried || !own_flag_at_base {
            return false;
        }
        *enemy_flag = FlagState::AtBase;
        instance.add_score(team, 1, tmpl);
        true
    }

    fn enemy_flag_mut(&mut self, team: BgTeam) -> &mut FlagState {
        match team {
            BgTeam::A => &mut self.flag_b,
            BgTeam::B => &mut self.flag_a,
        }
    }

    fn own_flag_mut(&mut self, team: BgTeam) -> &mut FlagState {
        match team {
            BgTeam::A => &mut self.flag_a,
            BgTeam::B => &mut self.flag_b,
        }
    }

    fn own_flag(&self, team: BgTeam) -> &FlagState {
        match team {
            BgTeam::A => &self.flag_a,
            BgTeam::B => &self.flag_b,
        }
    }
}

// -- Objective: Node Control (AB) --

/// Who controls a capture node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeOwner {
    Neutral,
    Contested(BgTeam),
    Owned(BgTeam),
}

/// A capturable territory node.
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureNode {
    pub name: String,
    pub owner: NodeOwner,
}

/// Node control objective state.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeObjective {
    pub nodes: Vec<CaptureNode>,
    /// Resource points awarded per owned node per tick.
    pub points_per_node_per_tick: u32,
}

/// Points per tick based on number of owned nodes (AB-style scaling).
/// 1 node=10, 2=10, 3=10, 4=30, 5=30 per tick.
const AB_POINTS_PER_TICK: [u32; 6] = [0, 10, 10, 10, 30, 30];

impl NodeObjective {
    /// Create AB-style 5-node objective.
    pub fn arathi_basin() -> Self {
        let names = ["Stables", "Blacksmith", "Lumber Mill", "Gold Mine", "Farm"];
        Self {
            nodes: names
                .iter()
                .map(|&n| CaptureNode {
                    name: n.into(),
                    owner: NodeOwner::Neutral,
                })
                .collect(),
            points_per_node_per_tick: 10,
        }
    }

    /// Begin capturing a node. Transitions Neutral/enemy Owned → Contested.
    pub fn start_capture(&mut self, node_idx: usize, team: BgTeam) -> bool {
        let Some(node) = self.nodes.get_mut(node_idx) else {
            return false;
        };
        let can_contest = match node.owner {
            NodeOwner::Neutral => true,
            NodeOwner::Owned(owner) => owner != team,
            NodeOwner::Contested(_) => false,
        };
        if can_contest {
            node.owner = NodeOwner::Contested(team);
        }
        can_contest
    }

    /// Complete capture of a contested node. Transitions Contested → Owned.
    pub fn finish_capture(&mut self, node_idx: usize) -> bool {
        let Some(node) = self.nodes.get_mut(node_idx) else {
            return false;
        };
        if let NodeOwner::Contested(team) = node.owner {
            node.owner = NodeOwner::Owned(team);
            true
        } else {
            false
        }
    }

    /// Count nodes owned by a team.
    pub fn owned_count(&self, team: BgTeam) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.owner == NodeOwner::Owned(team))
            .count()
    }

    /// Tick resource points based on owned nodes and apply to instance score.
    pub fn tick_resources(&self, instance: &mut BgInstance, tmpl: &BgTemplate) {
        for team in [BgTeam::A, BgTeam::B] {
            let count = self.owned_count(team);
            let points = AB_POINTS_PER_TICK.get(count).copied().unwrap_or(30);
            if points > 0 {
                instance.add_score(team, points, tmpl);
            }
        }
    }
}

// -- Objective: Reinforcements (AV) --

/// Reinforcement depletion config for kill events.
pub const REINFORCEMENT_LOSS_PER_KILL: u32 = 1;

/// Process a player kill in a reinforcement BG.
/// Deducts from the victim's team.
pub fn on_kill_reinforcement(instance: &mut BgInstance, victim_team: BgTeam) {
    instance.deduct_reinforcements(victim_team, REINFORCEMENT_LOSS_PER_KILL);
}
