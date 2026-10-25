//! TC a352b1fa ArenaTeam.cpp:534-594 and pinned worldserver.conf defaults.
//! Accepted reference formula, not a claim of Blizzard's proprietary Retail MMR.
pub(super) const INITIAL_PERSONAL_RATING: u32 = 0;
pub(super) const INITIAL_MMR: u32 = 1500;
const MAX_MMR_DIFFERENCE: u32 = 150;
const DISCARD_SECONDS: f64 = 600.0;

#[derive(Clone, Debug)]
pub(super) struct QueuedTeam {
    pub leader: u64,
    pub members: Vec<u64>,
    pub levels: Vec<u8>,
    pub mmr: u32,
    pub queued_at: f64,
}
/// TC BattlegroundQueue.cpp:877-951: latest-team seed, oldest seed on clock updates.
pub(super) fn find_pair(
    queue: &[QueuedTeam],
    now: f64,
    seed: Option<u32>,
) -> Option<(usize, usize)> {
    let seed = seed.or_else(|| queue.first().map(|team| team.mmr))?;
    let mut eligible = queue
        .iter()
        .enumerate()
        .filter(|(_, team)| {
            team.mmr.abs_diff(seed) <= MAX_MMR_DIFFERENCE || now - team.queued_at > DISCARD_SECONDS
        })
        .map(|(index, _)| index);
    Some((eligible.next()?, eligible.next()?))
}

#[derive(Clone, Debug)]
pub(super) struct RatedPlayer {
    pub id: u64,
    pub rating: u32,
    pub mmr: u32,
    pub team: shared::battleground::BgTeam,
}
#[derive(Clone, Debug)]
pub(super) struct RatedMatch {
    pub players: Vec<RatedPlayer>,
    pub team_mmr: [u32; 2],
    pub applied: bool,
}

const ELO_DIVISOR: f32 = 650.0;
const WIN_MODIFIER_LOW: f32 = 48.0;
const WIN_MODIFIER_HIGH: f32 = 24.0;
const LOSS_MODIFIER: f32 = 24.0;
const MATCHMAKER_MODIFIER: f32 = 24.0;

fn win_chance(own: u32, opponent: u32) -> f32 {
    1.0 / (1.0 + (10.0_f32.ln() * (opponent as f32 - own as f32) / ELO_DIVISOR).exp())
}
pub(super) fn rating_delta(own: u32, opponent_mmr: u32, won: bool) -> i32 {
    let chance = win_chance(own, opponent_mmr);
    let modifier = if won {
        if own < 1000 {
            WIN_MODIFIER_LOW
        } else if own < 1300 {
            WIN_MODIFIER_LOW / 2.0 + WIN_MODIFIER_LOW / 2.0 * (1300.0 - own as f32) / 300.0
        } else {
            WIN_MODIFIER_HIGH
        }
    } else {
        LOSS_MODIFIER
    };
    let result = if won {
        modifier * (1.0 - chance)
    } else {
        -modifier * chance
    };
    result.ceil() as i32
}
pub(super) fn matchmaker_delta(own: u32, opponent: u32, won: bool) -> i32 {
    (((if won { 1.0 } else { 0.0 }) - win_chance(own, opponent)) * MATCHMAKER_MODIFIER).ceil()
        as i32
}
